use std::ops::RangeInclusive;

use elden_analyzer_kernel::types::{clip_rect::ClipRect, rect::Rect};
use imageproc::image::{Luma, Pixel as _, Rgb, RgbImage};
use num_rational::Ratio;
use num_traits::ToPrimitive as _;

use crate::{util::ImageLogger, video_capture::Frame};

#[derive(Debug)]
pub struct HistogramBasedComponentDetectorBuilder {
    pub base_rect: ClipRect,
    pub level_width: u8,
    pub areas: Vec<(HistogramThreshold, Vec<ClipRect>)>,
}

impl HistogramBasedComponentDetectorBuilder {
    pub fn build(&self, frame_rect: Rect) -> Option<HistogramBasedComponentDetector> {
        let base_rect = self.base_rect.clip(frame_rect)?;
        let areas = self
            .areas
            .iter()
            .map(|(thr, clip_rects)| {
                (
                    thr.clone(),
                    clip_rects
                        .iter()
                        .map(|clip_rect| clip_rect.clip(base_rect).unwrap())
                        .collect(),
                )
            })
            .collect();
        Some(HistogramBasedComponentDetector {
            base_rect,
            level_width: self.level_width,
            areas,
        })
    }
}

#[derive(Debug, Clone)]
pub struct HistogramThreshold {
    pub name: &'static str,
    pub found_range: &'static [([RangeInclusive<u8>; 3], RangeInclusive<u8>)],
    pub found_threshold: f32,
}

impl HistogramThreshold {
    pub const fn new(
        name: &'static str,
        found_range: &'static [([RangeInclusive<u8>; 3], RangeInclusive<u8>)],
        found_threshold: f32,
    ) -> Self {
        Self {
            name,
            found_range,
            found_threshold,
        }
    }
}

#[derive(Debug)]
pub struct HistogramBasedComponentDetector {
    base_rect: Rect,
    level_width: u8,
    areas: Vec<(HistogramThreshold, Vec<Rect>)>,
}

impl HistogramBasedComponentDetector {
    pub fn detect(&self, frame: &Frame) -> bool {
        let logger = ImageLogger::get();

        let base_rect = self.base_rect;
        let img = tracing::trace_span!("rgb")
            .in_scope(|| logger.log(frame.to_rgb_image_within(base_rect).unwrap()));

        let u8_to_level = |v: u8| -> u8 { ((v as f32) / self.level_width as f32).round() as u8 };
        let level_to_u8 = |v: u8| -> u8 { v.saturating_mul(self.level_width) };

        let to_level_rgb = |p: Rgb<u8>| -> Rgb<u8> { p.map(u8_to_level) };
        let to_level_luma = |p: Rgb<u8>| -> Luma<u8> { p.to_luma().map(u8_to_level) };

        let in_range_rgb = |range: &[([RangeInclusive<u8>; 3], RangeInclusive<u8>)], p: Rgb<u8>| {
            let p = to_level_rgb(p);
            range
                .iter()
                .any(|r| r.0.iter().zip(p.0).all(|(r, v)| r.contains(&v)))
        };

        let in_range_luma = |range: &[([RangeInclusive<u8>; 3], RangeInclusive<u8>)],
                             p: Rgb<u8>| {
            let v = to_level_luma(p);
            range.iter().any(|r| r.1.contains(&v.0[0]))
        };

        if logger.display_image() {
            let rgb_leveled = {
                let mut img = img.clone();
                img.pixels_mut()
                    .for_each(|p| *p = to_level_rgb(*p).map(level_to_u8));
                img
            };

            let gray_leveled = {
                let mut img = img.clone();
                img.pixels_mut().for_each(|p| {
                    let v = to_level_luma(*p).map(level_to_u8)[0];
                    *p = [v, v, v].into();
                });
                img
            };

            {
                let init = [0, 64, 64].into();
                let mut rgb_out = RgbImage::from_pixel(img.width(), img.height(), init);
                let mut gray_out = RgbImage::from_pixel(img.width(), img.height(), init);
                for (thr, areas) in &self.areas {
                    for area in areas {
                        for x in area.left()..=area.right() {
                            let x = (x - base_rect.left()) as u32;
                            for y in area.top()..=area.bottom() {
                                let y = (y - base_rect.top()) as u32;
                                if rgb_out[(x, y)] != [255, 0, 0].into() {
                                    if in_range_rgb(thr.found_range, img[(x, y)]) {
                                        rgb_out.put_pixel(x, y, rgb_leveled[(x, y)]);
                                    } else {
                                        rgb_out.put_pixel(x, y, [255, 0, 0].into());
                                    }
                                }
                                if gray_out[(x, y)] != [255, 0, 0].into() {
                                    if in_range_luma(thr.found_range, img[(x, y)]) {
                                        gray_out.put_pixel(x, y, gray_leveled[(x, y)]);
                                    } else {
                                        gray_out.put_pixel(x, y, [255, 0, 0].into());
                                    }
                                }
                            }
                        }
                    }
                }
                logger.log(rgb_leveled);
                logger.log(rgb_out);
                logger.log(gray_leveled);
                logger.log(gray_out);
            }
        }

        for (idx, (thr, rects)) in self.areas.iter().enumerate() {
            let mut area = 0;
            let mut num_found = 0;
            for rect in rects {
                let img = frame.to_rgb_image_within(*rect).unwrap();
                area += (rect.width() * rect.height()) as i32;
                for p in img.pixels() {
                    if in_range_rgb(thr.found_range, *p) && in_range_luma(thr.found_range, *p) {
                        num_found += 1;
                    }
                }
            }

            let found_ratio = Ratio::new(num_found, area);
            let found_ratio_val = found_ratio.to_f32().unwrap();
            let found = found_ratio_val >= thr.found_threshold;
            tracing::trace!(idx, name = thr.name, accuracy = found_ratio_val, found);
            if !found {
                return false;
            }
        }

        true
    }
}
