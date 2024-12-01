use color_eyre::eyre;
use elden_analyzer_kernel::types::{clip_rect::ClipRect, rect::Rect};
use elden_analyzer_video::capture::Frame;
use imageproc::drawing;
use num_rational::Ratio;
use num_traits::ToPrimitive as _;

use crate::{
    image_process::{h_lines::HLineType, line_finder::LineFinder},
    util::ImageLogger,
    video_capture::FrameExt as _,
};

use super::{DetectComponent, DetectionKind};

pub struct LineBasedComponentDetectorBuilder {
    pub line_finder: LineFinder,

    pub base_rect: ClipRect,
    pub horizontal_line_clip_rect: Vec<(HLineType, ClipRect)>,

    pub found_threshold: f32,
    pub possible_threshold: f32,
}

impl LineBasedComponentDetectorBuilder {
    pub fn build(&self, frame_rect: Rect) -> Option<LineBasedComponentDetector> {
        let base_rect = self.base_rect.clip(frame_rect)?;
        let horizontal_line_clip_rect = self
            .horizontal_line_clip_rect
            .iter()
            .map(|(ty, clip_rect)| (*ty, clip_rect.clip(base_rect).unwrap()))
            .collect::<Vec<_>>();
        Some(LineBasedComponentDetector {
            line_finder: self.line_finder.clone(),
            base_rect,
            horizontal_line_clip_rect,
            found_threshold: self.found_threshold,
            possible_threshold: self.possible_threshold,
        })
    }
}

#[derive(Debug)]
pub struct LineBasedComponentDetector {
    line_finder: LineFinder,
    base_rect: Rect,
    horizontal_line_clip_rect: Vec<(HLineType, Rect)>,

    found_threshold: f32,
    possible_threshold: f32,
}

impl DetectComponent for LineBasedComponentDetector {
    #[tracing::instrument(level = "trace", skip_all)]
    fn detect(&self, frame: &Frame) -> eyre::Result<DetectionKind> {
        let logger = ImageLogger::get();
        if logger.display_image() {
            let base_rect = self.base_rect;

            let mut rgb_image = logger.log(frame.to_rgb_image_within(base_rect).unwrap());
            for (_ty, rect) in &self.horizontal_line_clip_rect {
                let rect = imageproc::rect::Rect::at(
                    rect.left() - base_rect.left(),
                    rect.top() - base_rect.top(),
                )
                .of_size(rect.width(), rect.height());
                drawing::draw_hollow_rect_mut(&mut rgb_image, rect, [255, 0, 0].into());
            }

            logger.log(rgb_image);
        }

        let mut total_accuracy = Ratio::new(1, 1);
        for (ty, rect) in self.horizontal_line_clip_rect.iter().copied() {
            let seg_len = self.line_finder.measure_in(frame, ty, rect);
            let accuracy = Ratio::new(seg_len.filled_len(), seg_len.base_len());
            let accuracy_val = accuracy.to_f32().unwrap();
            tracing::trace!(accuracy_val);

            if accuracy_val < self.possible_threshold {
                return Ok(DetectionKind::Absent);
            }
            if accuracy < total_accuracy {
                total_accuracy = accuracy
            }
        }

        let accuracy_val = total_accuracy.to_f32().unwrap();
        tracing::trace!(accuracy_val);
        let result = if accuracy_val > self.found_threshold {
            DetectionKind::Found
        } else if accuracy_val > self.possible_threshold {
            DetectionKind::Possible
        } else {
            DetectionKind::Absent
        };
        Ok(result)
    }
}
