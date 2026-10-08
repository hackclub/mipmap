use crate::http;
use actix_web::{HttpResponse, get, web};
use futures_util::future::join_all;
use image::{DynamicImage, GrayImage, Rgba, RgbaImage, imageops::FilterType};
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::Cursor,
    sync::{
        Arc, LazyLock, PoisonError, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};
use tract_onnx::prelude::*;

const METADATA: &str = include_str!("../../frontend/src/lib/assets/item_metadata.json");
const MODEL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/silueta.onnx"));
const SIDE: usize = 320;
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];
const CUTOFF: f32 = 160.0;
const RAMP: f32 = 4.0;
const MAX_DOWNLOAD: usize = 20_000_000;
const MAX_OUTPUT: usize = 100_000;
const WHITE: u8 = 0xf0;
const IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

#[derive(Deserialize)]
struct Meta {
    image: String,
}

#[derive(Clone)]
struct Stored {
    content_type: String,
    bytes: web::Bytes,
}

static IMAGES: LazyLock<RwLock<HashMap<u32, Stored>>> = LazyLock::new(Default::default);
static READY: AtomicBool = AtomicBool::new(false);

pub fn spawn() {
    actix_web::rt::spawn(async {
        let metadata: HashMap<u32, Meta> = match serde_json::from_str(METADATA) {
            Ok(metadata) => metadata,
            Err(e) => {
                log::error!("Parsing item metadata failed: {e:#}");
                return;
            }
        };
        let downloaded: Vec<(u32, Stored)> = join_all(metadata.into_iter().map(|(id, meta)| async move {
            match download(&meta.image).await {
                Ok(stored) => Some((id, stored)),
                Err(e) => {
                    log::warn!("Loading item image {id} failed: {e:#}");
                    None
                }
            }
        }))
        .await
        .into_iter()
        .flatten()
        .collect();
        {
            let mut images = IMAGES.write().unwrap_or_else(PoisonError::into_inner);
            for (id, stored) in &downloaded {
                images.insert(*id, stored.clone());
            }
        }
        let pending: Vec<(u32, web::Bytes)> = downloaded
            .into_iter()
            .map(|(id, stored)| (id, stored.bytes))
            .collect();
        let finished = web::block(move || -> anyhow::Result<()> {
            let model = load_model()?;
            let workers = std::thread::available_parallelism().map_or(1, |cores| cores.get());
            let next = std::sync::atomic::AtomicUsize::new(0);
            std::thread::scope(|scope| {
                for _ in 0..workers {
                    scope.spawn(|| {
                        while let Some((id, bytes)) =
                            pending.get(next.fetch_add(1, Ordering::Relaxed))
                        {
                            match cut_out(&model, bytes) {
                                Ok(Some(stored)) => {
                                    IMAGES.write().unwrap_or_else(PoisonError::into_inner).insert(*id, stored);
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    log::warn!("Removing the background of item image {id} failed: {e:#}")
                                }
                            }
                        }
                    });
                }
            });
            Ok(())
        })
        .await;
        match finished {
            Ok(Ok(())) => log::info!("Item images ready"),
            Ok(Err(e)) => log::error!("Loading the background removal model failed: {e:#}"),
            Err(e) => log::error!("Background removal stopped: {e:#}"),
        }
        READY.store(true, Ordering::Relaxed);
    });
}

type Model = Arc<TypedRunnableModel>;

fn load_model() -> anyhow::Result<Model> {
    tract_onnx::onnx()
        .model_for_read(&mut Cursor::new(MODEL))?
        .with_input_fact(0, f32::fact([1, 3, SIDE, SIDE]).into())?
        .into_optimized()?
        .into_runnable()
}

async fn download(url: &str) -> anyhow::Result<Stored> {
    let mut response = http::CLIENT.get(url).send().await?.error_for_status()?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !IMAGE_TYPES.contains(&content_type.as_str()) {
        anyhow::bail!("unsupported content type {content_type:?}");
    }
    if response.content_length().is_some_and(|length| length > MAX_DOWNLOAD as u64) {
        anyhow::bail!("image is too large");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > MAX_DOWNLOAD {
            anyhow::bail!("image is too large");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(Stored { content_type, bytes: bytes.into() })
}

fn is_white(pixel: &Rgba<u8>) -> bool {
    pixel.0[3] == 255 && pixel.0[..3].iter().all(|&channel| channel >= WHITE)
}

fn mostly_white_edge(image: &RgbaImage) -> bool {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return false;
    }
    let edge: Vec<&Rgba<u8>> = (0..width)
        .flat_map(|x| [(x, 0), (x, height - 1)])
        .chain((1..height.saturating_sub(1)).flat_map(|y| [(0, y), (width - 1, y)]))
        .map(|(x, y)| image.get_pixel(x, y))
        .collect();
    edge.iter().filter(|pixel| is_white(pixel)).count() * 2 > edge.len()
}

fn cut_out(model: &Model, bytes: &[u8]) -> anyhow::Result<Option<Stored>> {
    if image::guess_format(bytes).is_err() {
        return Ok(None);
    }
    let image = image::load_from_memory(bytes)?;
    let mut rgba = image.to_rgba8();
    if !mostly_white_edge(&rgba) {
        return Ok(None);
    }
    let mask = matte(model, &image)?;
    for (pixel, alpha) in rgba.pixels_mut().zip(mask.pixels()) {
        pixel.0[3] = alpha.0[0];
    }
    Ok(Some(Stored { content_type: "image/webp".into(), bytes: compress(rgba).into() }))
}

fn matte(model: &Model, image: &DynamicImage) -> anyhow::Result<GrayImage> {
    let small = image::imageops::resize(&image.to_rgb8(), SIDE as u32, SIDE as u32, FilterType::Lanczos3);
    let input = tract_ndarray::Array4::from_shape_fn((1, 3, SIDE, SIDE), |(_, c, y, x)| {
        (f32::from(small.get_pixel(x as u32, y as u32).0[c]) / 255.0 - MEAN[c]) / STD[c]
    });
    let outputs = model.run(tvec!(Tensor::from(input).into()))?;
    let logits = outputs[0].to_plain_array_view::<f32>()?;
    let mask = GrayImage::from_fn(SIDE as u32, SIDE as u32, |x, y| {
        let value = logits[[0, 0, y as usize, x as usize]];
        image::Luma([(255.0_f32 / (1.0 + (-value).exp())).round() as u8])
    });
    let mut mask = image::imageops::resize(&mask, image.width(), image.height(), FilterType::Lanczos3);
    for pixel in mask.pixels_mut() {
        let ramp = (f32::from(pixel.0[0]) - CUTOFF) * 255.0 / (255.0 - CUTOFF) * RAMP;
        pixel.0[0] = ramp.clamp(0.0, 255.0) as u8;
    }
    Ok(mask)
}

fn compress(image: RgbaImage) -> Vec<u8> {
    let mut image = DynamicImage::ImageRgba8(image);
    loop {
        let rgba = image.to_rgba8();
        for quality in [80.0, 65.0, 50.0] {
            let webp = webp::Encoder::from_rgba(&rgba, image.width(), image.height()).encode(quality);
            if webp.len() <= MAX_OUTPUT {
                return webp.to_vec();
            }
        }
        image = image.resize(
            (image.width() * 3 / 4).max(1),
            (image.height() * 3 / 4).max(1),
            FilterType::Lanczos3,
        );
    }
}

#[get("/{id}")]
pub async fn item_image(id: web::Path<u32>) -> HttpResponse {
    let images = IMAGES.read().unwrap_or_else(PoisonError::into_inner);
    match images.get(&id.into_inner()) {
        Some(stored) => HttpResponse::Ok()
            .content_type(stored.content_type.as_str())
            .insert_header((
                "Cache-Control",
                if READY.load(Ordering::Relaxed) { "public, max-age=3600" } else { "no-store" },
            ))
            .body(stored.bytes.clone()),
        None => HttpResponse::NotFound().finish(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_the_threshold_itself() {
        assert!(mostly_white_edge(&RgbaImage::from_pixel(5, 5, Rgba([0xf0, 0xf0, 0xf1, 255]))));
        assert!(!mostly_white_edge(&RgbaImage::from_pixel(5, 5, Rgba([0xef, 0xf0, 0xf0, 255]))));
    }

    #[test]
    fn needs_a_mostly_white_edge() {
        let mut image = RgbaImage::from_pixel(10, 10, Rgba([10, 10, 10, 255]));
        assert!(!mostly_white_edge(&image));
        for x in 0..10 {
            image.put_pixel(x, 0, Rgba([255, 255, 255, 255]));
            image.put_pixel(x, 9, Rgba([255, 255, 255, 255]));
        }
        image.put_pixel(0, 5, Rgba([255, 255, 255, 255]));
        assert!(mostly_white_edge(&image));
    }
}
