use crate::{
    env,
    error::{ApiError, reject, reject_with},
    http,
};
use actix_web::{http::StatusCode, web};
use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageDecoder, ImageError, ImageReader, Limits, imageops::FilterType};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    io::Cursor,
    sync::{Arc, LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

const CDN_UPLOAD_URL: &str = "https://cdn.hackclub.com/api/v4/upload";
const MODERATION_URL: &str = "https://api.openai.com/v1/moderations";
const MAX_BYTES: usize = 150_000;
const MAX_SIDE: u32 = 1920;
const MAX_PIXELS: u64 = 50_000_000;
const MAX_ALLOC: u64 = 256 * 1024 * 1024;
const INVALID: &str = "Please choose a PNG, JPEG or WebP image";
const TOO_LARGE: &str = "Images can be at most 50 megapixels";
const UPLOADS_PER_HOUR: usize = 30;
const HOUR: Duration = Duration::from_secs(60 * 60);

static COMPRESSING: Semaphore = Semaphore::const_new(1);

#[derive(Default)]
struct Budget {
    active: HashSet<i32>,
    recent: HashMap<i32, Vec<Instant>>,
}

static BUDGET: LazyLock<Mutex<Budget>> = LazyLock::new(Default::default);

struct Active(i32);

pub struct Slot(Arc<Active>);

pub fn reserve(user_id: i32) -> Result<Slot, ApiError> {
    let mut budget = BUDGET.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    budget.recent.retain(|_, times| {
        times.retain(|time| now.duration_since(*time) < HOUR);
        !times.is_empty()
    });
    if budget.active.contains(&user_id) {
        return Err(reject_with(StatusCode::CONFLICT, "Another image is still uploading"));
    }
    let times = budget.recent.entry(user_id).or_default();
    if times.len() >= UPLOADS_PER_HOUR {
        return Err(reject_with(
            StatusCode::TOO_MANY_REQUESTS,
            "You've uploaded a lot of images recently. Try again later",
        ));
    }
    times.push(now);
    budget.active.insert(user_id);
    Ok(Slot(Arc::new(Active(user_id))))
}

impl Drop for Active {
    fn drop(&mut self) {
        BUDGET
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .active
            .remove(&self.0);
    }
}

pub fn warn_unconfigured() {
    if env::var("OPENAI_API_KEY").is_none() {
        log::warn!("OPENAI_API_KEY not set, snapshots will fail");
    }
}

pub fn ensure_image(bytes: &[u8]) -> Result<(), ApiError> {
    match bytes {
        [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, ..]
        | [0xff, 0xd8, 0xff, ..]
        | [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => {}
        _ => return Err(reject(INVALID)),
    }
    let (width, height) = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .ok_or_else(|| reject(INVALID))?;
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(reject(TOO_LARGE));
    }
    Ok(())
}

#[derive(Deserialize)]
struct Uploaded {
    url: String,
}

#[derive(Deserialize)]
struct Moderation {
    results: Vec<ModerationResult>,
}

#[derive(Deserialize)]
struct ModerationResult {
    flagged: bool,
    #[serde(default)]
    categories: BTreeMap<String, bool>,
}

pub async fn upload(slot: &Slot, bytes: &[u8]) -> Result<String, ApiError> {
    Ok(store(prepare(slot, bytes).await?).await?)
}

pub async fn prepare(slot: &Slot, bytes: &[u8]) -> Result<Vec<u8>, ApiError> {
    let bytes = bytes.to_vec();
    let active = slot.0.clone();
    let permit = COMPRESSING.acquire().await.map_err(anyhow::Error::from)?;
    web::block(move || {
        let _active = active;
        let _permit = permit;
        compress(&bytes)
    })
    .await
    .map_err(anyhow::Error::from)?
}

pub async fn image_flags(webp: &[u8]) -> Result<BTreeSet<String>, ApiError> {
    let key = env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let input = json!([{
        "type": "image_url",
        "image_url": { "url": format!("data:image/webp;base64,{}", STANDARD.encode(webp)) },
    }]);
    let flags = moderate(&key, input).await.map_err(|e| {
        log::warn!("Image moderation failed: {e:#}");
        reject("Couldn't check your image. Please try again")
    })?;
    Ok(flags.into_iter().flatten().collect())
}

pub async fn text_flags(texts: &[&str]) -> Result<Vec<BTreeSet<String>>, ApiError> {
    let key = env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let flags = moderate(&key, json!(texts)).await.map_err(|e| {
        log::warn!("Text moderation failed: {e:#}");
        reject("Couldn't check your text. Please try again")
    })?;
    if flags.len() != texts.len() {
        return Err(anyhow::anyhow!("moderation returned {} results for {} texts", flags.len(), texts.len()).into());
    }
    Ok(flags)
}

async fn moderate(key: &str, input: Value) -> anyhow::Result<Vec<BTreeSet<String>>> {
    let response = http::CLIENT
        .post(MODERATION_URL)
        .bearer_auth(key)
        .timeout(Duration::from_secs(30))
        .json(&json!({ "model": "omni-moderation-latest", "input": input }))
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("moderation returned {status}: {}", response.text().await?);
    }
    let moderation: Moderation = response.json().await?;
    anyhow::ensure!(!moderation.results.is_empty(), "moderation returned no results");
    Ok(moderation
        .results
        .into_iter()
        .map(|result| {
            let mut flags: BTreeSet<String> = result
                .categories
                .into_iter()
                .filter_map(|(category, hit)| hit.then_some(category))
                .collect();
            if result.flagged && flags.is_empty() {
                flags.insert("flagged".to_owned());
            }
            flags
        })
        .collect())
}

fn unreadable(error: ImageError) -> ApiError {
    match error {
        ImageError::Limits(_) => reject(TOO_LARGE),
        _ => reject(INVALID),
    }
}

fn decode(bytes: &[u8]) -> Result<DynamicImage, ApiError> {
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_ALLOC);
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| reject(INVALID))?;
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().map_err(unreadable)?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(reject(TOO_LARGE));
    }
    limits.reserve(decoder.total_bytes()).map_err(unreadable)?;
    let orientation = decoder.orientation().map_err(unreadable)?;
    let image = DynamicImage::from_decoder(decoder).map_err(unreadable)?;
    let mut image = if width > MAX_SIDE * 2 || height > MAX_SIDE * 2 {
        DynamicImage::ImageRgba8(image.thumbnail(MAX_SIDE, MAX_SIDE).into_rgba8())
    } else {
        let image = DynamicImage::ImageRgba8(image.into_rgba8());
        if width > MAX_SIDE || height > MAX_SIDE {
            image.resize(MAX_SIDE, MAX_SIDE, FilterType::Lanczos3)
        } else {
            image
        }
    };
    image.apply_orientation(orientation);
    Ok(image)
}

fn compress(bytes: &[u8]) -> Result<Vec<u8>, ApiError> {
    let mut image = decode(bytes)?;
    loop {
        let rgba = image.to_rgba8();
        for quality in [80.0, 65.0, 50.0] {
            let webp = webp::Encoder::from_rgba(&rgba, image.width(), image.height()).encode(quality);
            if webp.len() < MAX_BYTES {
                return Ok(webp.to_vec());
            }
        }
        image = image.resize(
            (image.width() * 3 / 4).max(1),
            (image.height() * 3 / 4).max(1),
            FilterType::Lanczos3,
        );
    }
}

pub async fn store(webp: Vec<u8>) -> anyhow::Result<String> {
    let key = env::var("CDN_API_KEY").context("CDN_API_KEY not set")?;
    let file = Part::bytes(webp).file_name("image.webp").mime_str("image/webp")?;
    let response = http::CLIENT
        .post(CDN_UPLOAD_URL)
        .bearer_auth(key)
        .timeout(Duration::from_secs(60))
        .multipart(Form::new().part("file", file))
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("CDN upload returned {status}: {}", response.text().await?);
    }
    Ok(response.json::<Uploaded>().await?.url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbImage};

    fn rejected(result: Result<impl Sized, ApiError>) -> Option<String> {
        match result {
            Err(ApiError::Rejected(_, message)) => Some(message),
            _ => None,
        }
    }

    fn crc(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
            }
        }
        !crc
    }

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut chunk = b"IHDR".to_vec();
        chunk.extend(width.to_be_bytes());
        chunk.extend(height.to_be_bytes());
        chunk.extend([8, 2, 0, 0, 0]);
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
        png.extend(&chunk);
        png.extend(crc(&chunk).to_be_bytes());
        for name in [b"IDAT", b"IEND"] {
            png.extend([0, 0, 0, 0]);
            png.extend(name);
            png.extend(crc(name).to_be_bytes());
        }
        png
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        RgbImage::from_fn(width, height, |x, y| image::Rgb([x as u8, y as u8, 7]))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn pixel_limit() {
        assert!(ensure_image(&png_header(4032, 3024)).is_ok());
        assert!(ensure_image(&png_header(5712, 4284)).is_ok());
        assert_eq!(rejected(ensure_image(&png_header(8000, 7000))).as_deref(), Some(TOO_LARGE));
        assert_eq!(rejected(compress(&png_header(8000, 7000))).as_deref(), Some(TOO_LARGE));
    }

    #[test]
    fn corrupt_images_are_rejected() {
        let mut truncated = png(64, 64);
        truncated.truncate(truncated.len() / 2);
        assert!(ensure_image(&truncated).is_ok());
        assert_eq!(rejected(compress(&truncated)).as_deref(), Some(INVALID));
        assert_eq!(rejected(compress(&png_header(64, 64))).as_deref(), Some(INVALID));
    }

    #[test]
    fn large_images_shrink() {
        for (width, height) in [(4100, 20), (2600, 30), (100, 50)] {
            let webp = compress(&png(width, height)).ok().unwrap();
            let image = image::load_from_memory(&webp).unwrap();
            assert!(image.width() <= MAX_SIDE && image.height() <= MAX_SIDE);
            assert_eq!(image.width(), width.min(MAX_SIDE));
        }
    }
}
