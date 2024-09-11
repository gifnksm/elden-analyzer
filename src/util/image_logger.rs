use std::{
    mem,
    sync::{Arc, LazyLock, Mutex},
};

use color_eyre::eyre;
use imageproc::image::{buffer::ConvertBuffer, imageops, Rgb, RgbImage};

static CONFIG: LazyLock<Mutex<Option<ImageLoggerConfig>>> = LazyLock::new(|| Mutex::new(None));
static LOGGER: LazyLock<ImageLogger> = LazyLock::new(ImageLogger::new);

#[derive(Debug, Clone)]
struct ImageLoggerConfig {
    display_image: bool,
}

#[derive(Debug, Clone)]
pub struct ImageLogger(Arc<ImageLoggerInner>);

impl ImageLogger {
    fn new() -> Self {
        let conf = CONFIG.lock().unwrap().clone().unwrap();
        Self(Arc::new(ImageLoggerInner::new(conf)))
    }

    pub fn init(display_image: bool) -> eyre::Result<()> {
        let mut conf = CONFIG.lock().unwrap();
        if conf.is_some() {
            eyre::bail!("ImageLogger is already initialized")
        }
        *conf = Some(ImageLoggerConfig { display_image });
        Ok(())
    }

    pub fn get() -> &'static Self {
        &LOGGER
    }

    pub fn display_image(&self) -> bool {
        self.0.display_image()
    }

    pub fn log<T>(&self, img: T) -> T
    where
        T: ConvertBuffer<RgbImage>,
    {
        self.0.log(img)
    }

    pub fn display(&self, title: &str) {
        self.0.display(title);
    }

    pub fn end_column(&self) {
        self.0.end_column();
    }
}

#[derive(Debug)]
struct ImageLoggerInner {
    conf: ImageLoggerConfig,
    images: Mutex<Vec<Vec<RgbImage>>>,
}

impl ImageLoggerInner {
    fn new(conf: ImageLoggerConfig) -> Self {
        Self {
            conf,
            images: Mutex::new(vec![vec![]]),
        }
    }

    fn display_image(&self) -> bool {
        self.conf.display_image
    }

    fn log<T>(&self, img: T) -> T
    where
        T: ConvertBuffer<RgbImage>,
    {
        if self.display_image() {
            let img = img.convert();
            self.images.lock().unwrap().last_mut().unwrap().push(img);
        }
        img
    }

    fn display(&self, title: &str) {
        if !self.display_image() {
            return;
        }

        let images = { mem::replace(&mut *self.images.lock().unwrap(), vec![vec![]]) };
        if !images.is_empty() {
            let concatenated = concat_images(&images, 10, 10, Rgb([128, 128, 128]));
            imageproc::window::display_image(
                title,
                &concatenated,
                u32::min(concatenated.width(), 1024),
                u32::min(concatenated.height(), 768),
            );
        }
    }

    fn end_column(&self) {
        self.images.lock().unwrap().push(vec![]);
    }
}

fn concat_images(
    images: &[Vec<RgbImage>],
    x_margin: u32,
    y_margin: u32,
    bg_color: Rgb<u8>,
) -> RgbImage {
    let mut width = x_margin;
    let mut height = y_margin;
    let mut column_size = vec![];
    for column in images {
        let mut column_width = 0;
        let mut column_height = 0;
        for image in column {
            column_width = u32::max(column_width, image.width());
            column_height += image.height() + y_margin;
        }
        width += column_width + x_margin;
        height = u32::max(height, column_height);
        column_size.push((column_width, column_height));
    }

    let mut concatenated = RgbImage::from_pixel(width, height, bg_color);

    let mut x0 = x_margin;
    for (column, column_size) in images.iter().zip(&column_size) {
        let mut y = y_margin;
        for image in column {
            let x = x0 + column_size.0 / 2 - image.width() / 2;
            imageops::overlay(&mut concatenated, image, x as i64, y as i64);
            y += image.height() + y_margin;
        }
        x0 += column_size.0 + x_margin;
    }

    concatenated
}
