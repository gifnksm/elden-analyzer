use elden_analyzer_kernel::types::rect::Rect;
use elden_analyzer_video::capture::Frame;
use imageproc::{
    filter,
    image::{buffer::ConvertBuffer as _, GrayImage, Luma, RgbImage},
};

use crate::{
    algorithm::{FilledLength, FindLineSegments, MeasureFilledLength},
    util::ImageLogger,
    video_capture::FrameExt as _,
};

use super::h_lines::{HLineType, HLines};

#[derive(Debug, Clone)]
pub struct LineFinder {
    pub h_canny: HLines,
    pub find_line_segments: FindLineSegments,
}

impl LineFinder {
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn measure_in(&self, frame: &Frame, ty: HLineType, clip_rect: Rect) -> FilledLength {
        let logger = ImageLogger::get();

        let gray_image = tracing::trace_span!("gray")
            .in_scope(|| logger.log(frame.to_min_gray_image_within(clip_rect).unwrap()));
        let gray_image = tracing::trace_span!("median")
            .in_scope(|| logger.log(filter::median_filter(&gray_image, 5, 0)));
        let gray_image = tracing::trace_span!("lines").in_scope(|| {
            let data = self.h_canny.run(ty, &gray_image);
            let image =
                GrayImage::from_raw(data.width() as u32, data.height() as u32, data.into_raw())
                    .unwrap();
            logger.log(image)
        });

        let lines = tracing::trace_span!("find-line-segments").in_scope(|| {
            let lines = (0..)
                .zip(gray_image.rows())
                .flat_map(|(y, row)| {
                    let cells = row.map(|Luma([v])| *v > 0);
                    self.find_line_segments.find(cells).map(move |xs| (xs, y))
                })
                .collect::<Vec<_>>();

            if logger.display_image() {
                logger.log({
                    let mut rgb_image: RgbImage = gray_image.convert();
                    for (xs, y) in &lines {
                        for x in xs.clone() {
                            let pixel = rgb_image.get_pixel_mut(x as u32, *y as u32);
                            if pixel[0] > 0 {
                                pixel[1] = 0;
                                pixel[2] = 0;
                            } else {
                                pixel[1] = 255;
                            }
                        }
                    }
                    rgb_image
                });
            }

            lines
        });

        let target_rect = Rect::at(0, 0).of_size(gray_image.width(), gray_image.height());
        MeasureFilledLength::from_rect(target_rect).measure(lines)
    }
}
