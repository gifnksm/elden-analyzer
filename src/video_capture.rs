use elden_analyzer_kernel::types::rect::Rect;
use elden_analyzer_video::capture::Frame;
use imageproc::image::{ImageBuffer, Luma, Pixel as _, Rgb};

pub trait FrameExt {
    fn to_rgb_image(&self) -> ImageBuffer<Rgb<u8>, &[u8]>;
    fn to_rgb_image_within(&self, rect: Rect) -> Option<ImageBuffer<Rgb<u8>, Vec<u8>>>;
    fn to_gray_image(&self) -> ImageBuffer<Luma<u8>, Vec<u8>>;
    fn to_gray_image_within(&self, rect: Rect) -> Option<ImageBuffer<Luma<u8>, Vec<u8>>>;
    fn to_min_gray_image_within(&self, rect: Rect) -> Option<ImageBuffer<Luma<u8>, Vec<u8>>>;
}

impl FrameExt for Frame {
    fn to_rgb_image(&self) -> ImageBuffer<Rgb<u8>, &[u8]> {
        ImageBuffer::from_raw(self.width(), self.height(), self.data(0)).unwrap()
    }

    fn to_rgb_image_within(&self, rect: Rect) -> Option<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        let frame_rect = Rect::at(0, 0).of_size(self.width(), self.height());
        let rect = rect.intersect(frame_rect)?;
        let img = ImageBuffer::from_fn(rect.width(), rect.height(), |dx, dy| {
            let x = rect.left() as u32 + dx;
            let y = rect.top() as u32 + dy;
            let idx = ((y * self.width() + x) * 3) as usize;
            *Rgb::from_slice(&self.data(0)[idx..][..3])
        });
        Some(img)
    }

    fn to_gray_image(&self) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        ImageBuffer::from_fn(self.width(), self.height(), |x, y| {
            let idx = ((y * self.width() + x) * 3) as usize;
            let p = Rgb::from_slice(&self.data(0)[idx..][..3]);
            p.to_luma()
        })
    }

    fn to_gray_image_within(&self, rect: Rect) -> Option<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let frame_rect = Rect::at(0, 0).of_size(self.width(), self.height());
        let rect = rect.intersect(frame_rect)?;
        let img = ImageBuffer::from_fn(rect.width(), rect.height(), |dx, dy| {
            let x = rect.left() as u32 + dx;
            let y = rect.top() as u32 + dy;
            let idx = ((y * self.width() + x) * 3) as usize;
            let p = Rgb::from_slice(&self.data(0)[idx..][..3]);
            p.to_luma()
        });
        Some(img)
    }

    fn to_min_gray_image_within(&self, rect: Rect) -> Option<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let frame_rect = Rect::at(0, 0).of_size(self.width(), self.height());
        let rect = rect.intersect(frame_rect)?;
        let img = ImageBuffer::from_fn(rect.width(), rect.height(), |dx, dy| {
            let x = rect.left() as u32 + dx;
            let y = rect.top() as u32 + dy;
            let idx = ((y * self.width() + x) * 3) as usize;
            let v = self.data(0)[idx..][..3].iter().min().unwrap();
            [*v].into()
        });
        Some(img)
    }
}
