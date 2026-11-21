//! 图片压缩：上传时统一解码后重新编码，减小磁盘占用
//!
//! 策略：
//! - 无透明通道 → 转 JPEG（质量 85），通常比原 PNG 小 80%~90%
//! - 有透明通道 → 保留 PNG，使用最高压缩级别
//! - 长边超过 4096 的大图 → 等比缩小
//! - GIF 保持原样（避免丢失动画）
//! - 任何处理失败都回退为原始字节，不影响上传

use image::imageops::FilterType as ResizeFilter;
use image::{DynamicImage, GenericImageView, ImageEncoder};
use std::io::Cursor;

/// 长边上限，超过则等比缩小
const MAX_DIM: u32 = 4096;
/// JPEG 编码质量（85 在体积与画质间比较均衡）
const JPEG_QUALITY: u8 = 85;

pub struct Compressed {
    pub bytes: Vec<u8>,
    /// 压缩后应使用的扩展名（jpg / png / 原扩展名）
    pub ext: String,
    pub width: i64,
    pub height: i64,
}

/// 压缩图片；失败时回退为原始数据（尺寸用 imagesize 读取，读不到则为 0）
pub fn compress(input: &[u8], fallback_ext: &str) -> Compressed {
    // GIF 可能有动画，不做重新编码
    if fallback_ext.eq_ignore_ascii_case("gif") {
        let (w, h) = imagesize::blob_size(input)
            .map(|s| (s.width as i64, s.height as i64))
            .unwrap_or((0, 0));
        return Compressed {
            bytes: input.to_vec(),
            ext: "gif".into(),
            width: w,
            height: h,
        };
    }

    match encode(input) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("图片压缩失败，使用原图: {e}");
            let (w, h) = imagesize::blob_size(input)
                .map(|s| (s.width as i64, s.height as i64))
                .unwrap_or((0, 0));
            Compressed {
                bytes: input.to_vec(),
                ext: fallback_ext.to_string(),
                width: w,
                height: h,
            }
        }
    }
}

fn encode(input: &[u8]) -> Result<Compressed, String> {
    let img = image::load_from_memory(input).map_err(|e| format!("图片解码失败: {e}"))?;
    let (w, h) = img.dimensions();
    let img: DynamicImage = if w.max(h) > MAX_DIM {
        img.resize(MAX_DIM, MAX_DIM, ResizeFilter::Lanczos3)
    } else {
        img
    };
    let (nw, nh) = img.dimensions();

    if img.color().has_alpha() {
        // 有透明通道：保留 PNG，最高压缩级别
        let rgba = img.to_rgba8();
        let mut buf = Cursor::new(Vec::new());
        image::codecs::png::PngEncoder::new_with_quality(
            &mut buf,
            image::codecs::png::CompressionType::Best,
            image::codecs::png::FilterType::Adaptive,
        )
        .write_image(&rgba, nw, nh, image::ExtendedColorType::Rgba8)
        .map_err(|e| format!("PNG 编码失败: {e}"))?;
        Ok(Compressed {
            bytes: buf.into_inner(),
            ext: "png".into(),
            width: nw as i64,
            height: nh as i64,
        })
    } else {
        // 无透明通道：转 JPEG
        let rgb = img.to_rgb8();
        let mut buf = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY)
            .write_image(&rgb, nw, nh, image::ExtendedColorType::Rgb8)
            .map_err(|e| format!("JPEG 编码失败: {e}"))?;
        Ok(Compressed {
            bytes: buf,
            ext: "jpg".into(),
            width: nw as i64,
            height: nh as i64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;

    /// 生成一张指定尺寸、指定通道数的 PNG 字节
    fn make_png(w: u32, h: u32, alpha: bool) -> Vec<u8> {
        let mut buf = Vec::new();
        if alpha {
            let img = image::RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 128]));
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(&img, w, h, image::ExtendedColorType::Rgba8)
                .unwrap();
        } else {
            let img = image::RgbImage::from_pixel(w, h, image::Rgb([10, 120, 200]));
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(&img, w, h, image::ExtendedColorType::Rgb8)
                .unwrap();
        }
        buf
    }

    #[test]
    fn opaque_png_becomes_jpeg() {
        let input = make_png(300, 200, false);
        let out = compress(&input, "png");
        assert_eq!(out.ext, "jpg");
        assert_eq!((out.width, out.height), (300, 200));
        // JPEG 魔数 FF D8
        assert_eq!(&out.bytes[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn transparent_png_stays_png() {
        let input = make_png(120, 80, true);
        let out = compress(&input, "png");
        assert_eq!(out.ext, "png");
        assert_eq!((out.width, out.height), (120, 80));
        // PNG 魔数
        assert_eq!(&out.bytes[..8], b"\x89PNG\r\n\x1a\n");
        // 透明通道应保留（RGBA8）
        let decoded = image::load_from_memory(&out.bytes).unwrap();
        assert!(decoded.color().has_alpha());
    }

    #[test]
    fn oversized_image_is_downscaled() {
        let input = make_png(5000, 1000, false);
        let out = compress(&input, "png");
        assert_eq!(out.width, MAX_DIM as i64);
        // 等比：5000 -> 4096，高 1000 -> 819
        assert_eq!(out.height, 819);
    }

    #[test]
    fn gif_is_kept_as_is() {
        let input = b"GIF89a-not-really".to_vec();
        let out = compress(&input, "gif");
        assert_eq!(out.ext, "gif");
        assert_eq!(out.bytes, input);
    }
}
