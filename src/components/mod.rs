use std::{any::Any, array, fmt, iter};

use color_eyre::eyre;
use elden_analyzer_collections::array::array_from_iter;
use elden_analyzer_kernel::types::rect::Rect;
use elden_analyzer_video::capture::Frame;

use crate::{
    image_process::tesseract::Tesseract,
    operator::{DetectionKind, ExtractText, Recognition},
};

mod main_item;
mod side_item;

pub type DetectionPayload = Box<dyn Any + Send + Sync + 'static>;

#[derive(Debug)]
pub enum Detection {
    Found(Option<DetectionPayload>),
    Possible(Option<DetectionPayload>),
    Absent,
}

impl Detection {
    pub fn kind(&self) -> DetectionKind {
        match self {
            Detection::Found(_) => DetectionKind::Found,
            Detection::Possible(_) => DetectionKind::Possible,
            Detection::Absent => DetectionKind::Absent,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ExtractedTexts {
    pub result: Vec<Recognition>,
}

impl fmt::Display for ExtractedTexts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DebugElem<'a>(&'a Recognition);
        impl fmt::Debug for DebugElem<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        f.debug_list()
            .entries(self.result.iter().map(DebugElem))
            .finish()
    }
}

pub trait Component: fmt::Debug + Send + Sync + 'static {
    fn name(&self) -> &str;
    fn detect(&self, frame: &Frame) -> eyre::Result<Detection>;
    fn extract_text(
        &self,
        tess: &mut Tesseract,
        frame: &Frame,
        payload: Option<DetectionPayload>,
    ) -> eyre::Result<ExtractedTexts>;
}

#[derive(Debug, Clone, Copy)]
pub struct ComponentContainer<T> {
    pub main_item: T,
    pub side_item: [T; side_item::COUNT],
}

const COMPONENTS_COUNT: usize = 1 + side_item::COUNT;

pub type Components = ComponentContainer<Box<dyn Component>>;
pub type TextRecognizerComponents = ComponentContainer<Box<dyn ExtractText>>;

impl Components {
    pub fn new(frame_rect: Rect) -> Option<Self> {
        Some(Self {
            main_item: main_item::component(frame_rect)?,
            side_item: side_item::components(frame_rect)?,
        })
    }
}

impl<T> ComponentContainer<T> {
    pub fn from_fn(mut f: impl FnMut(&'static str) -> T) -> Self {
        Self {
            main_item: f(main_item::NAME),
            side_item: side_item::NAMES.map(f),
        }
    }

    pub fn iter(&self) -> Iter<T> {
        let Self {
            main_item,
            side_item,
        } = self;
        let it = iter::once(main_item).chain(side_item);
        let iter = array_from_iter(it).into_iter();
        Iter { iter }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        let Self {
            main_item,
            side_item,
        } = self;
        let it = iter::once(main_item).chain(side_item);
        let iter = array_from_iter(it).into_iter();
        IterMut { iter }
    }
}

impl<A> FromIterator<A> for ComponentContainer<A> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = A>,
    {
        let mut iter = iter.into_iter();
        let main_item = iter.next().unwrap();
        let side_item = array_from_iter(iter.by_ref().take(side_item::COUNT));
        assert!(iter.next().is_none());

        ComponentContainer {
            main_item,
            side_item,
        }
    }
}

impl<T> IntoIterator for ComponentContainer<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        let Self {
            main_item,
            side_item,
        } = self;
        let it = iter::once(main_item).chain(side_item);
        let iter = array_from_iter(it).into_iter();
        IntoIter { iter }
    }
}

impl<'a, T> IntoIterator for &'a ComponentContainer<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut ComponentContainer<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Debug)]
pub struct IntoIter<T> {
    iter: array::IntoIter<T, COMPONENTS_COUNT>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Debug)]
pub struct Iter<'a, T> {
    iter: array::IntoIter<&'a T, COMPONENTS_COUNT>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    iter: array::IntoIter<&'a mut T, COMPONENTS_COUNT>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
