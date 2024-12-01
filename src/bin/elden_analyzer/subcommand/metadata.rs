use std::path::PathBuf;

use color_eyre::eyre;
use elden_analyzer_video::metadata;

/// Show metadata of the video file
#[derive(clap::Parser, Debug)]
pub struct Args {
    /// The input file to process
    file: PathBuf,
}

impl Args {
    #[tracing::instrument(name = "metadata", skip_all)]
    pub(crate) fn run(&self) -> eyre::Result<()> {
        metadata::dump(&std::io::stdout(), &self.file)?;
        Ok(())
    }
}
