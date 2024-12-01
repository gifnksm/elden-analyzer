use color_eyre::eyre;
use elden_analyzer_kernel::types::{clip_rect::ClipRect, rect::Rect};
use num_rational::Ratio;

use crate::{
    algorithm::FindLineSegments,
    image_process::{
        h_lines::{HLineType, HLines},
        line_finder::LineFinder,
        tesseract::Tesseract,
    },
    operator::{
        DetectComponent, DetectionKind, ExtractText, LineBasedComponentDetectorBuilder,
        PostProcess, RectTextExtractorBuilder, TextAlign,
    },
    video_capture::Frame,
};

use super::{Component, Detection, DetectionPayload, ExtractedTexts};

pub(super) const NAME: &str = "main_item";

pub(super) fn component(frame_rect: Rect) -> Option<Box<dyn Component>> {
    let c = MainItemComponent::new(frame_rect)?;
    Some(Box::new(c) as _)
}

#[derive(Debug)]
struct MainItemComponent {
    name: String,
    detector: Box<dyn DetectComponent>,
    extractor: Box<dyn ExtractText>,
}

impl Component for MainItemComponent {
    fn name(&self) -> &str {
        &self.name
    }

    fn detect(&self, frame: &Frame) -> eyre::Result<Detection> {
        let det = match self.detector.detect(frame)? {
            DetectionKind::Found => Detection::Found(None),
            DetectionKind::Possible => Detection::Possible(None),
            DetectionKind::Absent => Detection::Absent,
        };
        Ok(det)
    }

    fn extract_text(
        &self,
        tess: &mut Tesseract,
        frame: &Frame,
        _payload: Option<DetectionPayload>,
    ) -> eyre::Result<ExtractedTexts> {
        let res = self.extractor.extract_text(tess, frame, None)?;
        Ok(ExtractedTexts { result: vec![res] })
    }
}

impl MainItemComponent {
    fn new(frame_rect: Rect) -> Option<Self> {
        let detector = new_detector(frame_rect)?;
        let extractor = new_extractor(frame_rect)?;

        Some(Self {
            name: NAME.to_string(),
            detector,
            extractor,
        })
    }
}

fn new_detector(frame_rect: Rect) -> Option<Box<dyn DetectComponent>> {
    let mut horizontal_line_clip_rect = MAIN_ITEM_HBARS_IN_BOX.to_vec();
    horizontal_line_clip_rect.sort_by_key(|(_ty, rect)| ClipRect::area(rect)); // sort by ascending area

    let d = LineBasedComponentDetectorBuilder {
        line_finder: LineFinder {
            h_canny: HLines {
                sigma: 1.0,
                low_threshold: 0,
                high_threshold: 10,
            },
            find_line_segments: FindLineSegments {
                vote_threshold: 60,
                min_line_len: 10,
                max_line_gap: 15,
            },
        },
        base_rect: MAIN_ITEM_BOX_IN_FRAME,
        horizontal_line_clip_rect,

        found_threshold: 0.80,
        possible_threshold: 0.20,
    }
    .build(frame_rect)?;
    Some(Box::new(d))
}

fn new_extractor(frame_rect: Rect) -> Option<Box<dyn ExtractText>> {
    let e = RectTextExtractorBuilder {
        base_rect: MAIN_ITEM_BOX_IN_FRAME,
        text_rect: MAIN_ITEM_TEXT_IN_BOX,
        post_process: PostProcess::ItemText,
        align: TextAlign::Center,
    }
    .build(frame_rect)?;
    Some(Box::new(e))
}

const MAIN_ITEM_BOX_IN_FRAME: ClipRect = ClipRect::new(
    (Ratio::new_raw(-1750, 10000), Ratio::new_raw(625, 10000)),
    (Ratio::new_raw(1750, 10000), Ratio::new_raw(3150, 10000)),
);

const MAIN_ITEM_HBARS_IN_BOX: &[(HLineType, ClipRect)] = &[
    // TOP
    (
        HLineType::TopNegative,
        ClipRect::new(
            (Ratio::new_raw(-48, 100), Ratio::new_raw(-475, 1000)),
            (Ratio::new_raw(48, 100), Ratio::new_raw(-460, 1000)),
        ),
    ),
    // MIDDLE
    (
        HLineType::BottomPositive,
        ClipRect::new(
            (Ratio::new_raw(-28, 100), Ratio::new_raw(-250, 1000)),
            (Ratio::new_raw(33, 100), Ratio::new_raw(-240, 1000)),
        ),
    ),
    (
        HLineType::TopNegative,
        ClipRect::new(
            (Ratio::new_raw(-28, 100), Ratio::new_raw(-225, 1000)),
            (Ratio::new_raw(33, 100), Ratio::new_raw(-215, 1000)),
        ),
    ),
    // BOTTOM
    (
        HLineType::BottomPositive,
        ClipRect::new(
            (Ratio::new_raw(-48, 100), Ratio::new_raw(455, 1000)),
            (Ratio::new_raw(48, 100), Ratio::new_raw(465, 1000)),
        ),
    ),
];

const MAIN_ITEM_TEXT_IN_BOX: ClipRect = ClipRect::new(
    (Ratio::new_raw(-24, 100), Ratio::new_raw(-35, 100)),
    (Ratio::new_raw(24, 100), Ratio::new_raw(-26, 100)),
);
