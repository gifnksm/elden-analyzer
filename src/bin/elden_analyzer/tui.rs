use std::fmt;

use indicatif::{ProgressState, ProgressStyle};
use num_rational::Ratio;
use tracing::Span;
use tracing_indicatif::span_ext::IndicatifSpanExt;

use elden_analyzer::video_capture::{Duration, FramePosition, Timestamp};

#[derive(Debug, Clone, Copy)]
pub struct ProgressBarBuilder {
    start: FramePosition,
    end: FramePosition,
    avg_fps: Ratio<i64>,
}

impl ProgressBarBuilder {
    pub fn new(start: FramePosition, end: FramePosition, fps: Ratio<i64>) -> Self {
        Self {
            start,
            end,
            avg_fps: fps,
        }
    }

    pub fn build(&self, span: Span) -> ProgressBar {
        pb_setup(
            &span,
            self.start.timestamp(),
            self.end.timestamp(),
            self.avg_fps,
        );
        ProgressBar {
            start: self.start,
            span,
        }
    }
}

#[derive(Debug)]
pub struct ProgressBar {
    start: FramePosition,
    span: Span,
}

impl ProgressBar {
    pub fn set_position(&self, pos: FramePosition) {
        pb_set_position(&self.span, pos, self.start.timestamp())
    }
}

fn pb_setup(span: &Span, start: Timestamp, end: Timestamp, avg_fps: Ratio<i64>) {
    if start > end {
        return;
    }

    static TEMPLATE: &str = "{spinner:.green} [{elapsed_precise}] {wide_bar:.cyan/blue}\n    {cur_pos}/{end_pos} ({per_sec}, {fps}, ETA: {eta_precise})";
    span.pb_set_style(
        &ProgressStyle::with_template(TEMPLATE)
            .unwrap()
            .with_key(
                "cur_pos",
                move |state: &ProgressState, w: &mut dyn fmt::Write| {
                    let cur_pos = start + Duration::new(Ratio::new(state.pos() as i64, 1000));
                    write!(w, "{cur_pos}").unwrap()
                },
            )
            .with_key(
                "end_pos",
                move |_state: &ProgressState, w: &mut dyn fmt::Write| write!(w, "{end}").unwrap(),
            )
            .with_key(
                "per_sec",
                move |state: &ProgressState, w: &mut dyn fmt::Write| {
                    let per_sec = state.per_sec() / 1000.0;
                    write!(w, "{per_sec:.3}s/s").unwrap()
                },
            )
            .with_key(
                "fps",
                move |state: &ProgressState, w: &mut dyn fmt::Write| {
                    let avg_fps = (*avg_fps.numer() as f64) / (*avg_fps.denom() as f64);
                    let per_sec = (state.per_sec() / 1000.0) * avg_fps;
                    write!(w, "{per_sec:.0}fr/s").unwrap()
                },
            ),
    );
    span.pb_set_length((end - start).as_msec() as u64);
}

fn pb_set_position(span: &Span, pos: FramePosition, start: Timestamp) {
    span.pb_set_position((pos.timestamp() - start).as_msec() as u64);
}
