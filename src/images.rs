use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, Context, TextureHandle, TextureId, TextureOptions, Vec2};
use image::{AnimationDecoder, GenericImageView};

/// Limites pensados para manter o consumo de RAM baixo: avatares pequenos,
/// anexos com no máximo 400x320 e GIFs com poucos quadros.
const MAX_STATIC_WIDTH: u32 = 400;
const MAX_STATIC_HEIGHT: u32 = 320;
const MAX_GIF_WIDTH: u32 = 240;
const MAX_GIF_HEIGHT: u32 = 240;
const MAX_GIF_FRAMES: usize = 16;
const MAX_ENTRIES: usize = 80;
const MAX_FAILED: usize = 400;

struct Entry {
    frames: Vec<TextureHandle>,
    delays: Vec<f32>,
    total: f32,
    started: Instant,
    last_used: u64,
    size: Vec2,
}

impl Entry {
    fn draw(&self, _tick: u64) -> Draw {
        let mut index = 0;
        if self.frames.len() > 1 {
            let elapsed = self.started.elapsed().as_secs_f32() % self.total.max(0.05);
            let mut remaining = elapsed;
            for (i, delay) in self.delays.iter().enumerate() {
                if remaining < *delay {
                    index = i;
                    break;
                }
                remaining -= *delay;
            }
        }

        Draw {
            texture: self.frames[index].id(),
            size: self.size,
            animated: self.frames.len() > 1,
        }
    }

    fn touch(&mut self, tick: u64) {
        self.last_used = tick;
    }
}

#[derive(Clone, Copy)]
pub struct Draw {
    pub texture: TextureId,
    pub size: Vec2,
    pub animated: bool,
}

#[derive(Default)]
pub struct ImageCache {
    entries: HashMap<String, Entry>,
    pending: HashSet<String>,
    failed: HashSet<String>,
    requests: Vec<String>,
    texture_seq: u64,
    tick: u64,
}

impl ImageCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Devolve a textura se a imagem já estiver pronta. Caso contrário,
    /// agenda o download uma única vez e devolve `None` (a UI mostra o
    /// fallback até o evento `ImageLoaded` chegar).
    pub fn get(&mut self, ctx: &Context, url: &str) -> Option<Draw> {
        self.tick += 1;
        let tick = self.tick;

        if let Some(entry) = self.entries.get_mut(url) {
            entry.touch(tick);
            let draw = entry.draw(tick);
            if draw.animated {
                ctx.request_repaint_after(Duration::from_millis(60));
            }
            return Some(draw);
        }

        if !self.pending.contains(url) && !self.failed.contains(url) {
            self.pending.insert(url.to_string());
            self.requests.push(url.to_string());
            ctx.request_repaint();
        }

        None
    }

    pub fn take_requests(&mut self) -> Vec<String> {
        std::mem::take(&mut self.requests)
    }

    pub fn mark_failed(&mut self, url: String) {
        self.pending.remove(&url);
        self.failed.insert(url);
    }

    pub fn receive(&mut self, ctx: &Context, url: String, bytes: Vec<u8>) {
        self.pending.remove(&url);

        match decode(ctx, bytes, self.texture_seq) {
            Some(entry) => {
                self.texture_seq += entry.frames.len() as u64 + 1;
                self.entries.insert(url, entry);
                self.evict();
            }
            None => {
                self.failed.insert(url);
                if self.failed.len() > MAX_FAILED {
                    self.failed.clear();
                }
            }
        }

        ctx.request_repaint();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.pending.clear();
        self.failed.clear();
        self.requests.clear();
    }

    fn evict(&mut self) {
        while self.entries.len() > MAX_ENTRIES {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(url, _)| url.clone())
            else {
                break;
            };
            self.entries.remove(&oldest);
        }
    }
}

fn decode(ctx: &Context, bytes: Vec<u8>, seq: u64) -> Option<Entry> {
    let format = image::guess_format(&bytes).ok()?;

    // Evita "bombas" de descompressão: uma imagem pequena em disco pode
    // declarar dimensões gigantescas e tentar alocar gigabytes na RAM.
    if let Ok(reader) = image::ImageReader::new(std::io::Cursor::new(bytes.as_slice()))
        .with_guessed_format()
    {
        if let Ok((width, height)) = reader.into_dimensions() {
            if width as u64 * height as u64 > 40_000_000 {
                return None;
            }
        }
    }

    if format == image::ImageFormat::Gif {
        if let Some(entry) = decode_gif(ctx, &bytes, seq) {
            return Some(entry);
        }
        // GIF com um único quadro ou animação inválida: tenta o caminho estático.
    }

    if format == image::ImageFormat::WebP {
        if let Some(entry) = decode_webp(ctx, &bytes, seq) {
            return Some(entry);
        }
        // WebP estático cai no decodificador normal abaixo.
    }

    let image = image::load_from_memory(&bytes).ok()?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let scale = (MAX_STATIC_WIDTH as f32 / width as f32)
        .min(MAX_STATIC_HEIGHT as f32 / height as f32)
        .min(1.0);
    let new_width = ((width as f32 * scale).round() as u32).max(1);
    let new_height = ((height as f32 * scale).round() as u32).max(1);

    let image = if scale < 1.0 {
        image.resize(new_width, new_height, image::imageops::FilterType::Triangle)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let color = ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        rgba.as_raw(),
    );
    let texture = ctx.load_texture(
        format!("phoenix-img-{seq}"),
        color,
        TextureOptions::LINEAR,
    );

    Some(Entry {
        frames: vec![texture],
        delays: vec![0.0],
        total: 0.0,
        started: Instant::now(),
        last_used: 0,
        size: Vec2::new(width as f32, height as f32),
    })
}

fn decode_gif(ctx: &Context, bytes: &[u8], seq: u64) -> Option<Entry> {
    let decoder = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes)).ok()?;

    let mut frames = Vec::new();
    let mut delays = Vec::new();
    let mut target_size: Option<(u32, u32)> = None;
    let mut display_size = Vec2::new(1.0, 1.0);

    for (index, frame) in decoder.into_frames().take(MAX_GIF_FRAMES).enumerate() {
        let Ok(frame) = frame else {
            break;
        };

        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay = if denominator == 0 {
            0.1
        } else {
            (numerator as f32 / denominator as f32 / 1000.0).clamp(0.02, 1.0)
        };

        let buffer = frame.into_buffer();
        let (width, height) = buffer.dimensions();
        if width == 0 || height == 0 {
            return None;
        }

        let (new_width, new_height) = *target_size.get_or_insert_with(|| {
            let scale = (MAX_GIF_WIDTH as f32 / width as f32)
                .min(MAX_GIF_HEIGHT as f32 / height as f32)
                .min(1.0);
            (
                ((width as f32 * scale).round() as u32).max(1),
                ((height as f32 * scale).round() as u32).max(1),
            )
        });

        let buffer = if (new_width, new_height) != (width, height) {
            image::imageops::resize(
                &buffer,
                new_width,
                new_height,
                image::imageops::FilterType::Triangle,
            )
        } else {
            buffer
        };

        let (width, height) = buffer.dimensions();
        let color =
            ColorImage::from_rgba_unmultiplied([width as usize, height as usize], buffer.as_raw());
        let texture = ctx.load_texture(
            format!("phoenix-gif-{seq}-{index}"),
            color,
            TextureOptions::LINEAR,
        );
        frames.push(texture);
        delays.push(delay);
        display_size = Vec2::new(width as f32, height as f32);
    }

    if frames.is_empty() {
        return None;
    }

    let total: f32 = delays.iter().sum();

    Some(Entry {
        frames,
        delays,
        total,
        started: Instant::now(),
        last_used: 0,
        size: display_size,
    })
}

fn decode_webp(ctx: &Context, bytes: &[u8], seq: u64) -> Option<Entry> {
    let mut decoder =
        image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).ok()?;
    if !decoder.is_animated() {
        return None;
    }

    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || width as u64 * height as u64 > 40_000_000 {
        return None;
    }

    let scale = (MAX_GIF_WIDTH as f32 / width as f32)
        .min(MAX_GIF_HEIGHT as f32 / height as f32)
        .min(1.0);
    let new_width = ((width as f32 * scale).round() as u32).max(1);
    let new_height = ((height as f32 * scale).round() as u32).max(1);

    let has_alpha = decoder.has_alpha();
    let buffer_size = decoder.output_buffer_size()?;
    let mut buffer = vec![0u8; buffer_size];
    let count = decoder.num_frames().min(MAX_GIF_FRAMES as u32);

    let mut frames = Vec::new();
    let mut delays = Vec::new();

    for index in 0..count {
        let duration_ms = decoder.read_frame(&mut buffer).ok()?;

        // O WebP animado devolve RGB ou RGBA dependendo do alpha do arquivo.
        let rgba: Vec<u8> = if has_alpha {
            buffer.clone()
        } else {
            let mut out = Vec::with_capacity(buffer.len() / 3 * 4);
            for pixel in buffer.chunks_exact(3) {
                out.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
            out
        };

        let frame = image::RgbaImage::from_raw(width, height, rgba)?;
        let frame = if (new_width, new_height) != (width, height) {
            image::imageops::resize(
                &frame,
                new_width,
                new_height,
                image::imageops::FilterType::Triangle,
            )
        } else {
            frame
        };

        let color = ColorImage::from_rgba_unmultiplied(
            [new_width as usize, new_height as usize],
            frame.as_raw(),
        );
        frames.push(ctx.load_texture(
            format!("phoenix-webp-{seq}-{index}"),
            color,
            TextureOptions::LINEAR,
        ));
        delays.push((duration_ms as f32 / 1000.0).clamp(0.02, 1.0));
    }

    if frames.is_empty() {
        return None;
    }

    let total: f32 = delays.iter().sum();
    Some(Entry {
        frames,
        delays,
        total,
        started: Instant::now(),
        last_used: 0,
        size: Vec2::new(new_width as f32, new_height as f32),
    })
}

/// Desenha uma textura em um retângulo arredondado (círculo, no caso de
/// avatares). Usa a própria malha do egui, sem depender do carregador de
/// imagens opcional.
pub fn paint_rounded_texture(
    painter: &egui::Painter,
    rect: egui::Rect,
    texture: TextureId,
    rounding: egui::Rounding,
) {
    painter.add(egui::Shape::Rect(egui::epaint::RectShape {
        rect,
        rounding,
        fill: egui::Color32::WHITE,
        stroke: egui::Stroke::NONE,
        blur_width: 0.0,
        fill_texture_id: texture,
        uv: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Delay, Frame, Rgba, RgbaImage};

    fn test_context() -> Context {
        let ctx = Context::default();
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(320.0, 240.0),
        ));
        let _ = ctx.run(input, |_| {});
        ctx
    }

    #[test]
    fn decodes_static_image() {
        let ctx = test_context();
        let mut cache = ImageCache::new();

        let image = RgbaImage::from_pixel(12, 8, Rgba([220, 40, 40, 255]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();

        cache.receive(&ctx, "static".to_string(), bytes);
        let draw = cache.get(&ctx, "static").expect("imagem estatica");
        assert!(!draw.animated);
        assert_eq!(draw.size, Vec2::new(12.0, 8.0));
    }

    #[test]
    fn decodes_animated_gif_with_frame_limit() {
        let ctx = test_context();
        let mut cache = ImageCache::new();

        let mut bytes = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
            for color in [[255u8, 0, 0, 255], [0, 255, 0, 255]] {
                encoder
                    .encode_frame(Frame::from_parts(
                        RgbaImage::from_pixel(6, 6, Rgba(color)),
                        0,
                        0,
                        Delay::from_numer_denom_ms(100, 1),
                    ))
                    .unwrap();
            }
        }

        cache.receive(&ctx, "animation".to_string(), bytes);
        let draw = cache.get(&ctx, "animation").expect("gif animado");
        assert!(draw.animated);
        assert_eq!(draw.size, Vec2::new(6.0, 6.0));
    }

    #[test]
    fn rejects_invalid_bytes() {
        let ctx = test_context();
        let mut cache = ImageCache::new();
        cache.receive(&ctx, "broken".to_string(), b"not an image".to_vec());
        assert!(cache.get(&ctx, "broken").is_none());
    }
}
