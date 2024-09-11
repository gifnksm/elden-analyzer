use std::ops::{Index, IndexMut};

use imageproc::image::GrayImage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vec2d<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
}

impl<T> Vec2d<T> {
    pub fn from_raw(width: usize, height: usize, data: Vec<T>) -> Self {
        assert_eq!(width * height, data.len());
        Self {
            width,
            height,
            data,
        }
    }

    pub fn new(width: usize, height: usize, init: T) -> Self
    where
        T: Clone,
    {
        Self::from_raw(width, height, vec![init; width * height])
    }

    pub fn from_fn(width: usize, height: usize, mut f: impl FnMut(usize, usize) -> T) -> Self {
        let data = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| f(x, y))
            .collect();
        Self::from_raw(width, height, data)
    }

    pub fn into_raw(self) -> Vec<T> {
        self.data
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn row(&self, y: usize) -> &[T] {
        let range = self.get_idx(0, y)..self.get_idx(0, y + 1);
        &self.data[range]
    }

    pub fn row_mut(&mut self, y: usize) -> &mut [T] {
        let range = self.get_idx(0, y)..self.get_idx(0, y + 1);
        &mut self.data[range]
    }

    pub fn rows(&self) -> impl Iterator<Item = &[T]> {
        self.data.chunks(self.width)
    }

    pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut [T]> {
        self.data.chunks_mut(self.width)
    }

    fn get_idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

impl<T> Index<(usize, usize)> for Vec2d<T> {
    type Output = T;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        let idx = self.get_idx(x, y);
        &self.data[idx]
    }
}

impl<T> IndexMut<(usize, usize)> for Vec2d<T> {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        let idx = self.get_idx(x, y);
        &mut self.data[idx]
    }
}

impl Vec2d<u8> {
    pub fn into_gray_image(self) -> GrayImage {
        GrayImage::from_raw(self.width as u32, self.height as u32, self.data).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index() {
        let vec2d = Vec2d::from_fn(3, 2, |x, y| x + y * 3);
        assert_eq!(vec2d[(0, 0)], 0);
        assert_eq!(vec2d[(1, 0)], 1);
        assert_eq!(vec2d[(2, 0)], 2);
        assert_eq!(vec2d[(0, 1)], 3);
        assert_eq!(vec2d[(1, 1)], 4);
        assert_eq!(vec2d[(2, 1)], 5);
    }

    #[test]
    fn row() {
        let vec2d = Vec2d::from_fn(3, 2, |x, y| x + y * 3);
        assert_eq!(vec2d.row(0), &[0, 1, 2]);
        assert_eq!(vec2d.row(1), &[3, 4, 5]);

        let mut rows = vec2d.rows();
        assert_eq!(rows.next(), Some([0, 1, 2].as_ref()));
        assert_eq!(rows.next(), Some([3, 4, 5].as_ref()));
        assert_eq!(rows.next(), None);
    }

    #[test]
    fn row_mut() {
        let mut vec2d = Vec2d::from_fn(3, 2, |x, y| x + y * 3);
        assert_eq!(vec2d.row_mut(0), &mut [0, 1, 2]);
        assert_eq!(vec2d.row_mut(1), &mut [3, 4, 5]);

        let mut rows = vec2d.rows_mut();
        assert_eq!(rows.next(), Some([0, 1, 2].as_mut()));
        assert_eq!(rows.next(), Some([3, 4, 5].as_mut()));
        assert_eq!(rows.next(), None);
    }
}
