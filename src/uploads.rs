use crate::{
    env,
    error::{ApiError, reject, reject_with},
    http,
};
use actix_web::{http::StatusCode, web};
use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    io::Cursor,
    sync::{LazyLock, Mutex, PoisonError},
    thread,
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

const CDN_UPLOAD_URL: &str = "https://cdn.hackclub.com/api/v4/upload";
const MODERATION_URL: &str = "https://api.openai.com/v1/moderations";
const MAX_BYTES: usize = 150_000;
const MAX_SIDE: u32 = 1920;
const MAX_UPLOAD_SIDE: u32 = 4000;
const UPLOADS_PER_HOUR: usize = 30;
const HOUR: Duration = Duration::from_secs(60 * 60);

static COMPRESSING: LazyLock<Semaphore> = LazyLock::new(|| {
    Semaphore::new(thread::available_parallelism().map_or(1, |cores| (cores.get() / 2).max(1)))
});

#[derive(Default)]
struct Budget {
    active: HashSet<i32>,
    recent: HashMap<i32, Vec<Instant>>,
}

static BUDGET: LazyLock<Mutex<Budget>> = LazyLock::new(Default::default);

pub struct Slot(i32);

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
    Ok(Slot(user_id))
}

impl Drop for Slot {
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
        log::warn!("OPENAI_API_KEY not set, image uploads and snapshots will fail");
    }
}

pub fn ensure_image(bytes: &[u8]) -> Result<(), ApiError> {
    const INVALID: &str = "Please choose a PNG, JPEG or WebP image";
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
    if width > MAX_UPLOAD_SIDE || height > MAX_UPLOAD_SIDE {
        return Err(reject(format!(
            "Images can be at most {MAX_UPLOAD_SIDE} x {MAX_UPLOAD_SIDE} pixels"
        )));
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
    let webp = prepare(slot, bytes).await?;
    if !image_flags(&webp).await?.is_empty() {
        return Err(reject("This image isn't allowed"));
    }
    Ok(store(webp).await?)
}

pub async fn prepare(_slot: &Slot, bytes: &[u8]) -> Result<Vec<u8>, ApiError> {
    let bytes = bytes.to_vec();
    let permit = COMPRESSING.acquire().await.map_err(anyhow::Error::from)?;
    Ok(web::block(move || {
        let _permit = permit;
        compress(&bytes)
    })
    .await
    .map_err(anyhow::Error::from)??)
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

fn compress(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    if image.width() > MAX_SIDE || image.height() > MAX_SIDE {
        image = image.resize(MAX_SIDE, MAX_SIDE, FilterType::Lanczos3);
    }
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

