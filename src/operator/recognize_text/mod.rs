use std::{fmt, ops};

use color_eyre::eyre;
use elden_analyzer_video::capture::Frame;
use num_rational::Ratio;

use crate::image_process::tesseract::Tesseract;

pub use self::{post_process::*, rect::*};

mod post_process;
mod rect;

pub trait ExtractText: fmt::Debug + Send + Sync + 'static {
    fn extract_text(
        &self,
        tess: &mut Tesseract,
        frame: &Frame,
        num_digits: Option<usize>,
    ) -> eyre::Result<Recognition>;
}

#[derive(Debug, Clone)]
pub enum Recognition {
    Found(String, Confidence),
    Possible(String, Confidence),
}

impl fmt::Display for Recognition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Recognition::Found(text, conf) => write!(f, "{text}({conf})"),
            Recognition::Possible(text, conf) => write!(f, "??{text}({conf})"),
        }
    }
}

impl Recognition {
    pub fn map_text(self, f: impl FnOnce(String) -> String) -> Self {
        match self {
            Recognition::Found(text, conf) => Recognition::Found(f(text), conf),
            Recognition::Possible(text, conf) => Recognition::Possible(f(text), conf),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Confidence(Ratio<i32>);

impl Confidence {
    pub fn new(value: i32) -> Self {
        assert!((0..=100).contains(&value));
        Self(Ratio::new(value, 100))
    }

    pub fn as_ratio(self) -> Ratio<i32> {
        self.0
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", (self.0 * 100).round().to_integer())
    }
}

impl<T> ops::Add<T> for Confidence
where
    Ratio<i32>: ops::Add<T, Output = Ratio<i32>>,
{
    type Output = Confidence;

    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl<T> ops::Sub<T> for Confidence
where
    Ratio<i32>: ops::Sub<T, Output = Ratio<i32>>,
{
    type Output = Confidence;

    fn sub(self, rhs: T) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl<T> ops::Mul<T> for Confidence
where
    Ratio<i32>: ops::Mul<T, Output = Ratio<i32>>,
{
    type Output = Confidence;

    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl<T> ops::Div<T> for Confidence
where
    Ratio<i32>: ops::Div<T, Output = Ratio<i32>>,
{
    type Output = Confidence;

    fn div(self, rhs: T) -> Self::Output {
        Self(self.0 / rhs)
    }
}
