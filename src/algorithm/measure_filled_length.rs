use std::{
    fmt, iter,
    ops::{self, Range},
};

use imageproc::rect::Rect;
use num_rational::Ratio;

#[derive(Debug, Clone, Copy, Default)]
pub struct FilledLength {
    filled_len: i32,
    base_len: i32,
}

impl iter::Sum for FilledLength {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), ops::Add::add)
    }
}

impl ops::Add for FilledLength {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            filled_len: self.filled_len + rhs.filled_len,
            base_len: self.base_len + rhs.base_len,
        }
    }
}

impl ops::AddAssign for FilledLength {
    fn add_assign(&mut self, rhs: Self) {
        self.filled_len += rhs.filled_len;
        self.base_len += rhs.base_len;
    }
}

impl fmt::Display for FilledLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            filled_len,
            base_len,
        } = *self;
        let percent = filled_len as f64 / base_len as f64 * 100.0;
        write!(f, "{percent:.2}%",)
    }
}

impl FilledLength {
    pub fn filled_len(&self) -> i32 {
        self.filled_len
    }

    pub fn base_len(&self) -> i32 {
        self.base_len
    }
}

#[derive(Debug)]
pub struct MeasureFilledLength {
    rect: Rect,
}

impl MeasureFilledLength {
    pub fn from_rect(rect: Rect) -> Self {
        Self { rect }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn measure(&self, lines: impl IntoIterator<Item = (Range<i32>, i32)>) -> FilledLength {
        let filled_len = self.filled_len(lines);
        let base_len = self.base_len();
        FilledLength {
            filled_len,
            base_len,
        }
    }

    fn filled_len(&self, lines: impl IntoIterator<Item = (Range<i32>, i32)>) -> i32 {
        let mut bucket = [false; 64];
        let bucket_count = bucket.len() as i32;
        for (lxs, ly) in lines {
            if ly < self.rect.top() || ly > self.rect.bottom() {
                continue; // out of bar
            }

            let bx0 = self.rect.left();
            let bx1 = self.rect.right();
            let w = self.rect.width() as i32;

            assert!(lxs.start < lxs.end);
            let start = lxs.start.clamp(bx0, bx1) - bx0;
            let end = lxs.end.clamp(bx0, bx1 + 1) - bx0;

            let start = start * bucket_count / w;
            let end = ((end - 1) * bucket_count) / w + 1;

            let start = start as usize;
            let end = end as usize;
            bucket[start..end].fill(true);
        }

        let filled_count = bucket.into_iter().filter(|filled| *filled).count();
        let ratio = Ratio::new(filled_count as i32, bucket.len() as i32);
        let bar_len = Ratio::from_integer(self.rect.width() as i32);

        (bar_len * ratio).round().to_integer()
    }

    fn base_len(&self) -> i32 {
        self.rect.width() as i32
    }
}
