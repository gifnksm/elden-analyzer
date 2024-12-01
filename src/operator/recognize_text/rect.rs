use std::ops::Range;

use color_eyre::eyre;
use elden_analyzer_kernel::types::{clip_rect::ClipRect, rect::Rect};
use elden_analyzer_video::capture::Frame;
use imageproc::{
    contrast::{self, ThresholdType},
    distance_transform::Norm,
    gradients,
    image::{
        buffer::ConvertBuffer as _,
        imageops::{self, FilterType},
        GrayImage, Pixel,
    },
    morphology,
};
use tracing::trace;

use crate::{
    image_process::tesseract::Tesseract, operator::Confidence, util::ImageLogger,
    video_capture::FrameExt as _,
};

use super::{ExtractText, PostProcess, Recognition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Unspecified,
}

#[derive(Debug)]
pub struct RectTextExtractorBuilder {
    pub base_rect: ClipRect,
    pub text_rect: ClipRect,
    pub post_process: PostProcess,
    pub align: TextAlign,
}

impl RectTextExtractorBuilder {
    pub fn build(&self, frame_rect: Rect) -> Option<RectTextExtractor> {
        let base_rect = self.base_rect.clip(frame_rect)?;
        let text_rect = self.text_rect.clip(base_rect)?;
        Some(RectTextExtractor {
            base_rect,
            text_rect,
            post_process: self.post_process,
            align: self.align,
        })
    }
}

#[derive(Debug)]
pub struct RectTextExtractor {
    base_rect: Rect,
    text_rect: Rect,
    post_process: PostProcess,
    align: TextAlign,
}

impl ExtractText for RectTextExtractor {
    #[tracing::instrument(level = "trace", skip_all)]
    fn extract_text(
        &self,
        tess: &mut Tesseract,
        frame: &Frame,
        num_chars: Option<usize>,
    ) -> eyre::Result<Recognition> {
        let logger = ImageLogger::get();

        if logger.display_image() {
            logger.log(frame.to_rgb_image_within(self.base_rect).unwrap());
        }

        recognize(
            tess,
            self.text_rect,
            self.post_process,
            self.align,
            frame,
            num_chars,
        )
    }
}

fn recognize(
    tess: &mut Tesseract,
    text_rect: Rect,
    pp: PostProcess,
    align: TextAlign,
    frame: &Frame,
    num_chars: Option<usize>,
) -> eyre::Result<Recognition> {
    let expected_height = 40; // x-height is 20px. see https://github.com/tesseract-ocr/tessdoc/blob/main/tess3/FAQ-Old.md#is-there-a-minimum--maximum-text-size-it-wont-read-screen-text
    let min_trim_width = 40;
    let trim_margin = 10;
    let clip_scale_factor = 1.2;
    let clip_binary_threshold = 0xc0;
    let mask_gradients_scale_factor = 1.2;
    let mask_gradients_threshold = 200;
    let mask_white_threshold = 0xc0;
    let mask_close_norm = Norm::L1;
    let mask_close_k = 2;

    let logger = ImageLogger::get();

    let rgb_image = tracing::trace_span!("rgb")
        .in_scope(|| logger.log(frame.to_rgb_image_within(text_rect).unwrap()));

    let size_scale = expected_height as f32 / text_rect.height() as f32;
    trace!(?size_scale);

    let rgb_image = tracing::trace_span!("resize").in_scope(|| {
        let width = (text_rect.width() as f32 * size_scale).round() as u32;
        let height = (text_rect.height() as f32 * size_scale).round() as u32;
        logger.log(imageops::resize(
            &rgb_image,
            width,
            height,
            FilterType::Lanczos3,
        ))
    });

    let gray_image: GrayImage =
        tracing::trace_span!("gray").in_scope(|| logger.log(rgb_image.convert()));

    let gray_image = clip_image(
        gray_image,
        clip_scale_factor,
        clip_binary_threshold,
        align,
        min_trim_width,
        trim_margin,
    );

    let recognize_binary_threshold =
        tracing::trace_span!("otsu-level").in_scope(|| contrast::otsu_level(&gray_image));
    tracing::trace!(recognize_binary_threshold);
    let binary_image = tracing::trace_span!("binary").in_scope(|| {
        logger.log(contrast::threshold(
            &gray_image,
            recognize_binary_threshold,
            ThresholdType::BinaryInverted,
        ))
    });
    let (text1, conf1) = match do_recognize(tess, &binary_image, pp, num_chars)? {
        Recognition::Found(text1, conf1) => return Ok(Recognition::Found(text1, conf1)),
        Recognition::Possible(text1, conf1) => (text1, conf1),
    };

    let (gray_min, gray_max) = gray_image
        .iter()
        .copied()
        .fold((u8::MAX, u8::MIN), |(min, max), v| (min.min(v), max.max(v)));
    let gray_width = gray_max - gray_min + 1;
    let gray_mid = (gray_min + gray_max) / 2;
    let scale = 255.0 / gray_width as f32 * mask_gradients_scale_factor;
    let scaled = scale_color(&gray_image, gray_mid / 4, scale);
    let grads = gradients::sobel_gradients(&scaled);
    let thr = mask_gradients_threshold;
    let mask = logger.log(GrayImage::from_fn(
        scaled.width(),
        scaled.height(),
        |x, y| {
            let g = grads[(x, y)].0[0];
            if g >= thr || scaled[(x, y)].0[0] > mask_white_threshold {
                return [255].into();
            }
            [0].into()
        },
    ));
    let mask = logger.log(morphology::close(&mask, mask_close_norm, mask_close_k));
    let masked = logger.log(GrayImage::from_fn(
        scaled.width(),
        scaled.height(),
        |x, y| [scaled[(x, y)].0[0] & mask[(x, y)].0[0]].into(),
    ));
    let masked_binary_threshold =
        tracing::trace_span!("otsu-level").in_scope(|| contrast::otsu_level(&masked));
    trace!(?masked_binary_threshold);
    let masked_binary_image = tracing::trace_span!("binary").in_scope(|| {
        logger.log(contrast::threshold(
            &masked,
            masked_binary_threshold,
            ThresholdType::BinaryInverted,
        ))
    });

    let res = match do_recognize(tess, &masked_binary_image, pp, num_chars)? {
        Recognition::Found(text2, conf2) => Recognition::Found(text2, conf2),
        Recognition::Possible(text2, conf2) => {
            if conf1 >= conf2 {
                Recognition::Possible(text1, conf1)
            } else {
                Recognition::Possible(text2, conf2)
            }
        }
    };
    Ok(res)
}

fn do_recognize(
    tess: &mut Tesseract,
    binary_image: &GrayImage,
    pp: PostProcess,
    num_chars: Option<usize>,
) -> eyre::Result<Recognition> {
    let (text, conf) = tess.recognize(binary_image)?;
    let conf = Confidence::new(conf);
    let (text, conf) = match pp.run(&text, conf) {
        Recognition::Found(text, conf) => (text, conf),
        Recognition::Possible(text, conf) => return Ok(Recognition::Possible(text, conf)),
    };
    let res = match num_chars {
        Some(num_chars) if text.chars().count() != num_chars => Recognition::Possible(text, conf),
        _ => Recognition::Found(text, conf),
    };
    Ok(res)
}

fn scale_color(gray_image: &GrayImage, mid: u8, color_scale: f32) -> GrayImage {
    let logger = ImageLogger::get();

    tracing::trace_span!("scale").in_scope(|| {
        logger.log(GrayImage::from_fn(
            gray_image.width(),
            gray_image.height(),
            |x, y| {
                let p = gray_image[(x, y)];
                p.map(|v| {
                    let v = (v as f32 - mid as f32) * color_scale + mid as f32;
                    f32::clamp(v.round(), 0.0, 255.0) as u8
                })
            },
        ))
    })
}

fn clip_image(
    gray_image: GrayImage,
    clip_scale_factor: f32,
    clip_binary_threshold: u8,
    align: TextAlign,
    min_trim_width: i32,
    trim_margin: i32,
) -> GrayImage {
    let logger = ImageLogger::get();

    let w = gray_image.width() as i32;
    let h = gray_image.height() as i32;
    let x_range = match align {
        TextAlign::Left => 0..min_trim_width,
        TextAlign::Right => w - min_trim_width..w,
        TextAlign::Center => {
            let cx = w / 2;
            cx - min_trim_width / 2..cx + min_trim_width / 2
        }
        TextAlign::Unspecified => 0..w,
    };
    let x_range = x_range.start.max(0)..x_range.end.min(w);

    let gray_max = x_range
        .flat_map(|x| {
            let gray_image = &gray_image;
            (0..h).map(move |y| gray_image[(x as u32, y as u32)][0])
        })
        .fold(u8::MIN, |max, p| max.max(p));
    let color_scale = (255.0 / gray_max as f32) * clip_scale_factor;
    trace!(?color_scale);

    let scaled = scale_color(&gray_image, 0, color_scale);

    let clip_binary = tracing::trace_span!("binary").in_scope(|| {
        logger.log(contrast::threshold(
            &scaled,
            clip_binary_threshold,
            ThresholdType::BinaryInverted,
        ))
    });

    tracing::trace_span!("clip").in_scope(|| {
        if let Some(clip_rect) = find_clip_rect(&clip_binary, align, min_trim_width, trim_margin) {
            tracing::trace!(?clip_rect);
            logger.log(GrayImage::from_fn(
                clip_rect.width(),
                clip_rect.height(),
                |x, y| gray_image[(x + clip_rect.left() as u32, y + clip_rect.top() as u32)],
            ))
        } else {
            gray_image
        }
    })
}

fn find_clip_rect(
    binary_image: &GrayImage,
    align: TextAlign,
    min_trim_width: i32,
    trim_margin: i32,
) -> Option<Rect> {
    assert!(min_trim_width > 0 && trim_margin >= 0);
    assert!(trim_margin <= min_trim_width);

    let search_start_x = match align {
        TextAlign::Left => 0,
        TextAlign::Right => binary_image.width() as i32 - 1,
        TextAlign::Center => binary_image.width() as i32 / 2,
        TextAlign::Unspecified => return None,
    };

    let mut is_white = vec![true; binary_image.width() as usize];
    for x in 0..binary_image.width() {
        for y in 0..binary_image.height() {
            if binary_image.get_pixel(x, y)[0] == 0 {
                is_white[x as usize] = false;
                break;
            }
        }
    }

    let white_segments = true_segments(&is_white)
        .filter(|range| range.end - range.start >= min_trim_width as usize)
        .collect::<Vec<_>>();

    let trim_start = white_segments
        .iter()
        .rev()
        .skip_while(|range| range.end as i32 >= search_start_x)
        .map(|range| range.end as i32 - 1 - trim_margin)
        .next()
        .unwrap_or(0);
    let trim_end = white_segments
        .iter()
        .skip_while(|range| range.start as i32 <= search_start_x)
        .map(|range| range.start as i32 + trim_margin + 1)
        .next()
        .unwrap_or(is_white.len() as i32);

    Some(Rect::at(trim_start, 0).of_size((trim_end - trim_start) as u32, binary_image.height()))
}

fn true_segments(vs: &[bool]) -> impl Iterator<Item = Range<usize>> + '_ {
    assert!(!vs.is_empty());

    let true_start = vs[0].then_some(0).into_iter().chain(
        vs.windows(2)
            .enumerate()
            .filter_map(|(i, xs)| (xs == [false, true]).then_some(i + 1)),
    );
    let true_end = vs
        .windows(2)
        .enumerate()
        .filter_map(|(i, xs)| (xs == [true, false]).then_some(i + 1))
        .chain(vs.last().unwrap().then_some(vs.len()));
    true_start.zip(true_end).map(|(start, end)| start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn test_true_segments() {
        assert!(true_segments(&[false]).eq::<[Range<usize>; 0]>([]));
        assert!(true_segments(&[true]).eq([0..1]));
        assert!(true_segments(&[true, false, true]).eq([0..1, 2..3]));
        assert!(true_segments(&[false, true, true]).eq([1..3]));
        assert!(true_segments(&[true, true, true, false]).eq([0..3]));
        assert!(
            true_segments(&[true, false, true, false, true, false, true, false]).eq([
                0..1,
                2..3,
                4..5,
                6..7
            ])
        );
    }
}
