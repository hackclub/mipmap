use crate::{
    env,
    error::{ApiError, reject},
    http,
};
use actix_web::web;
use anyhow::Context;
use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::{io::Cursor, sync::LazyLock, thread, time::Duration};
use tokio::sync::Semaphore;

const CDN_UPLOAD_URL: &str = "https://cdn.hackclub.com/api/v4/upload";
const MAX_BYTES: usize = 150_000;
const MAX_SIDE: u32 = 1920;
const MAX_UPLOAD_SIDE: u32 = 4000;

static COMPRESSING: LazyLock<Semaphore> = LazyLock::new(|| {
    Semaphore::new(thread::available_parallelism().map_or(1, |cores| (cores.get() / 2).max(1)))
});

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

pub async fn upload(bytes: &[u8]) -> anyhow::Result<String> {
    let bytes = bytes.to_vec();
    let permit = COMPRESSING.acquire().await?;
    let webp = web::block(move || {
        let _permit = permit;
        compress(&bytes)
    })
    .await??;
    store(webp).await
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

async fn store(webp: Vec<u8>) -> anyhow::Result<String> {
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

