use imageproc::rect::Rect;
use num_rational::Ratio;

#[derive(Debug, Clone, Copy)]
pub struct ClipRect {
    top: Ratio<i32>,
    left: Ratio<i32>,
    right: Ratio<i32>,
    bottom: Ratio<i32>,
}

impl ClipRect {
    pub const fn new(
        (left, top): (Ratio<i32>, Ratio<i32>),
        (right, bottom): (Ratio<i32>, Ratio<i32>),
    ) -> Self {
        Self {
            top,
            left,
            right,
            bottom,
        }
    }

    pub fn area(&self) -> Ratio<i32> {
        (self.right - self.left) * (self.bottom - self.top)
    }

    pub fn clip(&self, base_rect: Rect) -> Option<Rect> {
        let bcx = Ratio::new(base_rect.left() + base_rect.right() + 1, 2);
        let bcy = Ratio::new(base_rect.top() + base_rect.bottom() + 1, 2);
        let bw = Ratio::from_integer(base_rect.width() as i32);
        let bh = Ratio::from_integer(base_rect.height() as i32);

        let clip_left = (bcx + self.left * bw)
            .floor()
            .to_integer()
            .clamp(base_rect.left(), base_rect.right());
        let clip_right = (bcx + self.right * bw)
            .ceil()
            .to_integer()
            .clamp(base_rect.left(), base_rect.right());

        let clip_top = (bcy + self.top * bh)
            .floor()
            .to_integer()
            .clamp(base_rect.top(), base_rect.bottom());
        let clip_bottom = (bcy + self.bottom * bh)
            .ceil()
            .to_integer()
            .clamp(base_rect.top(), base_rect.bottom());

        let clip_width = (clip_right - clip_left) as u32 + 1;
        let clip_height = (clip_bottom - clip_top) as u32 + 1;

        (clip_left >= 0 && clip_top >= 0 && clip_width > 0 && clip_height > 0)
            .then(|| Rect::at(clip_left, clip_top).of_size(clip_width, clip_height))
    }
}
