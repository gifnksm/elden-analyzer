use std::path::PathBuf;

use color_eyre::eyre::{self, OptionExt};
use elden_analyzer::{
    components::Components,
    util::ImageLogger,
    video_capture::{Frame, TimestampRange, VideoCapture},
};
use tracing::info;

/// Analyze the video files to extract information
#[derive(clap::Parser, Debug)]
pub struct Args {
    /// The input file to process
    file: PathBuf,
    /// The frame to process
    #[clap(default_value = "-")]
    timestamp: Vec<TimestampRange>,
    /// Display output image
    #[clap(long, default_value = "false")]
    display_image: bool,
    #[clap(long, value_delimiter = ',')]
    filter: Option<Vec<String>>,
}

impl Args {
    #[tracing::instrument(name = "find_ui", skip_all)]
    pub(crate) fn run(&self) -> eyre::Result<()> {
        ImageLogger::init(self.display_image)?;

        let mut capture =
            tracing::trace_span!("open").in_scope(|| VideoCapture::open(&self.file))?;
        let components = Components::new(capture.rect()).ok_or_eyre("invalid frame size")?;

        let mut frame = Frame::empty();
        for ts_range in &self.timestamp {
            let mut decoder = capture.range_decoder(*ts_range)?;
            while tracing::trace_span!("decode-frame")
                .in_scope(|| decoder.decode_frame(&mut frame))?
            {
                process_frame(&components, &frame, self.filter.as_deref())?;
            }
        }

        Ok(())
    }
}

#[tracing::instrument(skip_all, fields(pos = %frame.position()))]
fn process_frame(
    components: &Components,
    frame: &Frame,
    filter: Option<&[String]>,
) -> eyre::Result<()> {
    let logger = ImageLogger::get();

    for component in components {
        if let Some(filter) = filter {
            if !filter.iter().any(|s| *s == component.name()) {
                continue;
            }
        }

        tracing::info_span!("detect-ui", name = component.name()).in_scope(
            || -> eyre::Result<()> {
                let result = component.detect(frame)?;
                info!(result = %result.kind());
                Ok(())
            },
        )?;
        logger.end_column();
    }
    logger.display(&format!("find-ui [{}]", frame.position()));

    Ok(())
}
