use std::ffi::CString;

use color_eyre::eyre;
use imageproc::image::GrayImage;
use tesseract_plumbing::{tesseract_sys::TessPageSegMode_PSM_SINGLE_LINE, TessBaseApi};

#[derive(Debug)]
pub struct Tesseract {
    tess: TessBaseApi,
}

impl Tesseract {
    pub fn new(datapath: Option<&str>, language: Option<&str>) -> eyre::Result<Self> {
        let mut tess = TessBaseApi::create();
        tess.init_2(
            datapath.map(CString::new).transpose()?.as_deref(),
            language.map(CString::new).transpose()?.as_deref(),
        )?;
        tess.set_page_seg_mode(TessPageSegMode_PSM_SINGLE_LINE);
        Ok(Self { tess })
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn recognize(&mut self, image: &GrayImage) -> eyre::Result<(String, i32)> {
        self.tess.set_image(
            image.as_raw(),
            image.width() as i32,
            image.height() as i32,
            1,
            image.width() as i32,
        )?;

        let conf = self.tess.mean_text_conf();

        let text = self
            .tess
            .get_utf8_text()?
            .as_ref()
            .to_string_lossy()
            .replace(|ch: char| ch.is_whitespace(), "");
        tracing::trace!(text, conf);
        Ok((text, conf))
    }
}
