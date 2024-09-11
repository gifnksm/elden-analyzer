use imageproc::{
    filter,
    gradients::{horizontal_sobel, vertical_sobel},
    image::{GrayImage, RgbImage},
};

use crate::{collections::Vec2d, geometry::Neighbor, util::ImageLogger};

#[derive(Debug, Clone, Copy)]
pub enum HLineType {
    TopNegative,
    BottomPositive,
    BottomNegative,
}

#[derive(Debug, Clone, Copy)]

pub struct HLines {
    pub sigma: f32,
    pub low_threshold: u16,
    pub high_threshold: u16,
}

impl HLines {
    pub fn run(&self, ty: HLineType, image: &GrayImage) -> Vec2d<u8> {
        let logger = ImageLogger::get();

        let image = tracing::trace_span!("blur")
            .in_scope(|| logger.log(filter::gaussian_blur_f32(image, self.sigma)));
        let width = image.width() as usize;
        let height = image.height() as usize;

        let gx = horizontal_sobel(&image);
        let gy = vertical_sobel(&image);

        // Computes the intensity of the gravity of horizontal edges
        let filtered_gy = tracing::trace_span!("filtered-gy").in_scope(|| {
            let filtered_gy = Vec2d::from_fn(width, height, |x, y| {
                let (x, y) = (x as u32, y as u32);
                let gx = gx[(x, y)][0].unsigned_abs();
                let gy = gy[(x, y)][0].unsigned_abs();
                // tan 67.5 degree = 2.4
                // tan x = gy/gx >= 2.4
                // => gx * 24 <= gy * 10
                let mag = gy;
                if gx * 24 <= gy * 10 && mag >= self.low_threshold {
                    mag
                } else {
                    0
                }
            });

            if logger.display_image() {
                logger.log(GrayImage::from_fn(image.width(), image.height(), |x, y| {
                    let fgy = filtered_gy[(x as usize, y as usize)];
                    let gy = gy[(x, y)][0];
                    let v = if fgy > 0 {
                        (gy as f32 / (4.0 * self.high_threshold as f32) + 0.5) * 255.0
                    } else {
                        255.0 / 2.0
                    };
                    [v as u8].into()
                }));
            }

            filtered_gy
        });

        const HIGH_VALUE: u8 = 255;
        const LOW_VALUE: u8 = 128;

        // Finds local maxima to make the edges thinner
        let local_maximum = tracing::trace_span!("local-maximum").in_scope(|| {
            let mut local_maximum = Vec2d::new(width, height, 0);
            for y in 0..height {
                let top_row = &filtered_gy.row(y.saturating_sub(1));
                let bottom_row = &filtered_gy.row((y + 1).min(height - 1));
                let center_row = &filtered_gy.row(y);
                let dest_row = local_maximum.row_mut(y);

                for (dest, (top, (center, bottom))) in dest_row.iter_mut().zip(
                    top_row
                        .iter()
                        .copied()
                        .zip(center_row.iter().copied().zip(bottom_row.iter().copied())),
                ) {
                    if center >= top && center >= bottom {
                        if center >= self.high_threshold {
                            *dest = HIGH_VALUE;
                        } else if center >= self.low_threshold {
                            *dest = LOW_VALUE;
                        }
                    }
                }
            }

            if logger.display_image() {
                logger.log(RgbImage::from_fn(image.width(), image.height(), |x, y| {
                    let v = local_maximum[(x as usize, y as usize)];
                    if gy.get_pixel(x, y)[0] > 0 {
                        [v, v, 0].into()
                    } else {
                        [0, v, v].into()
                    }
                }));
            }

            local_maximum
        });

        // Finds edges based on threshold with hysterisis
        let edges = tracing::trace_span!("hysterisis").in_scope(|| {
            let mut edges = Vec2d::new(width, height, 0);
            let mut to_visit = vec![];
            for (y, row) in local_maximum.rows().enumerate() {
                for (x, in_v) in row.iter().copied().enumerate() {
                    let out_v = &mut edges[(x, y)];
                    if *out_v > 0 || in_v < HIGH_VALUE {
                        continue;
                    }
                    *out_v = HIGH_VALUE;
                    to_visit.push((x, y));

                    while let Some((nx, ny)) = to_visit.pop() {
                        for (nx, ny) in Neighbor::neighbors_in((nx, ny), width, height) {
                            let in_nv = local_maximum[(nx, ny)];
                            let out_nv = &mut edges[(nx, ny)];
                            if in_nv >= LOW_VALUE && *out_nv == 0 {
                                *out_nv = HIGH_VALUE;
                                to_visit.push((nx, ny));
                            }
                        }
                    }
                }
            }
            edges
        });

        // Fill lines
        tracing::trace_span!("fill-lines").in_scope(|| {
            let mut lines = Vec2d::new(width, 1, 0);
            let cond = match ty {
                HLineType::TopNegative | HLineType::BottomNegative => {
                    (&|x, y| gy[(x, y)][0] < 0) as &dyn Fn(u32, u32) -> bool
                }
                HLineType::BottomPositive => &|x, y| gy[(x, y)][0] > 0,
            };
            let y_iter = match ty {
                HLineType::TopNegative => {
                    (&|| Box::new(0..height) as Box<dyn Iterator<Item = usize>>)
                        as &dyn Fn() -> Box<dyn Iterator<Item = usize>>
                }
                HLineType::BottomPositive | HLineType::BottomNegative => {
                    &|| Box::new((0..height).rev()) as _
                }
            };
            for x in 0..width {
                let bottom_y = y_iter()
                    .take_while(|&y| cond(x as u32, y as u32))
                    .find(|&y| edges[(x, y)] == HIGH_VALUE);
                if bottom_y.is_some() {
                    lines[(x, 0)] = HIGH_VALUE;
                }
            }
            lines
        })
    }
}
