pub mod capture;
pub mod metadata;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("FFmpeg error: {0}")]
    Ffmpeg(#[from] ffmpeg::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn init() -> Result<()> {
    ffmpeg::init()?;
    Ok(())
}
