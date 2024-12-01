use std::fmt;

use color_eyre::eyre;
use elden_analyzer_video::capture::Frame;

pub use self::{histogram_based::*, line_based::*};

mod histogram_based;
mod line_based;

pub trait DetectComponent: fmt::Debug + Send + Sync + 'static {
    fn detect(&self, frame: &Frame) -> eyre::Result<DetectionKind>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionKind {
    Found,
    Possible,
    Absent,
}

impl fmt::Display for DetectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectionKind::Found => write!(f, "Found"),
            DetectionKind::Possible => write!(f, "Possible"),
            DetectionKind::Absent => write!(f, "Absent"),
        }
    }
}
