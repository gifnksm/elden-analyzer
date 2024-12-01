use color_eyre::eyre::{self, eyre};
use elden_analyzer_kernel::types::{clip_rect::ClipRect, rect::Rect};
use num_rational::Ratio;

use crate::{
    image_process::tesseract::Tesseract,
    operator::{
        ExtractText, HistogramBasedComponentDetector, HistogramBasedComponentDetectorBuilder,
        HistogramThreshold, PostProcess, Recognition, RectTextExtractorBuilder, TextAlign,
    },
    video_capture::Frame,
};

use super::{Component, Detection, DetectionPayload, ExtractedTexts};

pub(super) const COUNT: usize = 10;
pub(super) const NAMES: [&str; COUNT] = [
    "side_item0",
    "side_item1",
    "side_item2",
    "side_item3",
    "side_item4",
    "side_item5",
    "side_item6",
    "side_item7",
    "side_item8",
    "side_item9",
];

pub(super) fn components(frame_rect: Rect) -> Option<[Box<dyn Component>; COUNT]> {
    let uis = NAMES
        .iter()
        .zip(SIDE_ITEM_BOX_IN_FRAME)
        .map(|(name, base_rect)| {
            let c = SideItemComponent::new(name.to_string(), base_rect, frame_rect)?;
            Some(Box::new(c) as Box<_>)
        })
        .collect::<Option<Vec<_>>>()?;
    Some(uis.try_into().unwrap())
}

#[derive(Debug)]
struct Payload {
    count_digits: CountDigits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CountDigits {
    One,
    Two,
}

#[derive(Debug)]
struct SideItemComponent {
    name: String,
    d1_detector: HistogramBasedComponentDetector,
    d2_detector: HistogramBasedComponentDetector,
    text_extractor: Box<dyn ExtractText>,
    d1_extractor: Box<dyn ExtractText>,
    d2_extractor: Box<dyn ExtractText>,
}

impl Component for SideItemComponent {
    fn name(&self) -> &str {
        &self.name
    }

    fn detect(&self, frame: &Frame) -> eyre::Result<Detection> {
        if self.d1_detector.detect(frame) {
            let payload = Payload {
                count_digits: CountDigits::One,
            };
            return Ok(Detection::Found(Some(Box::new(payload))));
        }
        if self.d2_detector.detect(frame) {
            let payload = Payload {
                count_digits: CountDigits::Two,
            };
            return Ok(Detection::Found(Some(Box::new(payload))));
        }
        Ok(Detection::Absent)
    }

    fn extract_text(
        &self,
        tess: &mut Tesseract,
        frame: &Frame,
        payload: Option<DetectionPayload>,
    ) -> eyre::Result<ExtractedTexts> {
        let payload = payload
            .map(|p| {
                p.downcast::<Payload>()
                    .map_err(|_| eyre!("invalid payload"))
            })
            .transpose()?;

        let text = self.text_extractor.extract_text(tess, frame, None)?;

        let count = match payload.as_ref().map(|p| p.count_digits) {
            Some(CountDigits::One) => self.d1_extractor.extract_text(tess, frame, Some(1))?,
            Some(CountDigits::Two) => self.d2_extractor.extract_text(tess, frame, Some(2))?,
            _ => self.extract_count_chain(tess, frame)?,
        };
        let count = count.map_text(|text| format!("×{}", text));

        Ok(ExtractedTexts {
            result: vec![text, count],
        })
    }
}

impl SideItemComponent {
    fn new(name: String, base_rect: ClipRect, frame_rect: Rect) -> Option<Self> {
        let d1_detector = new_detector(base_rect, frame_rect, SIDE_ITEM_AREAS_IN_BOX[0])?;
        let d2_detector = new_detector(base_rect, frame_rect, SIDE_ITEM_AREAS_IN_BOX[1])?;
        let text_extractor = new_extractor(base_rect, frame_rect, TEXT_IN_BOX[0])?;
        let d1_extractor = new_extractor(base_rect, frame_rect, TEXT_IN_BOX[1])?;
        let d2_extractor = new_extractor(base_rect, frame_rect, TEXT_IN_BOX[2])?;
        Some(Self {
            name,
            d1_detector,
            d2_detector,
            text_extractor,
            d1_extractor,
            d2_extractor,
        })
    }

    fn extract_count_chain(
        &self,
        tess: &mut Tesseract,
        frame: &Frame,
    ) -> eyre::Result<Recognition> {
        let d1 = self.d1_extractor.extract_text(tess, frame, Some(1))?;
        let (text1, conf1) = match d1 {
            Recognition::Found(text, conf) => return Ok(Recognition::Found(text, conf)),
            Recognition::Possible(text, conf) => (text, conf),
        };

        let d2 = self.d2_extractor.extract_text(tess, frame, Some(2))?;
        let (text2, conf2) = match d2 {
            Recognition::Found(text, conf) => return Ok(Recognition::Found(text, conf)),
            Recognition::Possible(text, conf) => (text, conf),
        };

        if conf1 >= conf2 {
            Ok(Recognition::Possible(text1, conf1))
        } else {
            Ok(Recognition::Possible(text2, conf2))
        }
    }
}

fn new_detector(
    base_rect: ClipRect,
    frame_rect: Rect,
    areas: &[(HistogramThreshold, &[ClipRect])],
) -> Option<HistogramBasedComponentDetector> {
    let mut areas: Vec<_> = areas
        .iter()
        .map(|(thr, rects)| (thr.clone(), rects.to_vec()))
        .collect();
    areas.sort_by_key(|(_thr, rects)| -> Ratio<i32> { rects.iter().map(ClipRect::area).sum() }); // sort by ascending area

    HistogramBasedComponentDetectorBuilder {
        base_rect,
        level_width: SIDE_ITEM_LEVEL_WIDTH,
        areas,
    }
    .build(frame_rect)
}

fn new_extractor(
    base_rect: ClipRect,
    frame_rect: Rect,
    rect: (ClipRect, PostProcess, TextAlign),
) -> Option<Box<dyn ExtractText>> {
    let e = RectTextExtractorBuilder {
        base_rect,
        text_rect: rect.0, //TEXT_IN_BOX.to_vec(),
        post_process: rect.1,
        align: rect.2,
    }
    .build(frame_rect)?;
    Some(Box::new(e) as _)
}

const SIDE_ITEM_X0_IN_FRAME: i32 = 1364;
const SIDE_ITEM0_Y0_IN_FRAME: i32 = 822;
const SIDE_ITEM_WIDTH: i32 = 556;
const SIDE_ITEM_HEIGHT: i32 = 44;

const SIDE_ITEM_BOX_IN_FRAME: [ClipRect; COUNT] = {
    const WIDTH: i32 = 1920;
    const HEIGHT: i32 = 1080;
    const X0: i32 = SIDE_ITEM_X0_IN_FRAME;

    const fn rect((x0, y0): (i32, i32)) -> ClipRect {
        ClipRect::from_points(
            (x0, y0),
            (x0 + SIDE_ITEM_WIDTH - 1, y0 + SIDE_ITEM_HEIGHT - 1),
            (WIDTH, HEIGHT),
        )
    }

    [
        rect((X0, SIDE_ITEM0_Y0_IN_FRAME)),
        rect((X0, 756)), // -66
        rect((X0, 689)), // -67
        rect((X0, 623)), // -66
        rect((X0, 557)), // -66
        rect((X0, 490)), // -67
        rect((X0, 424)), // -66
        rect((X0, 358)), // -66
        rect((X0, 291)), // -67
        rect((X0, 225)), // -66
    ]
};

const SIDE_ITEM_LEVEL_WIDTH: u8 = 16;
const SIDE_ITEM_AREAS_IN_BOX: &[&[(HistogramThreshold, &[ClipRect])]] = {
    const WIDTH: i32 = SIDE_ITEM_WIDTH;
    const HEIGHT: i32 = SIDE_ITEM_HEIGHT;

    const X0: i32 = SIDE_ITEM_X0_IN_FRAME;
    const Y0: i32 = SIDE_ITEM0_Y0_IN_FRAME;

    const ITEM_X0: i32 = 1385;
    const ITEM_X1: i32 = 1710;
    const ITEM_WIDTH: i32 = ITEM_X1 + 1 - ITEM_X0;
    const TEXT_Y0: i32 = 838;
    const TEXT_Y1: i32 = 865;

    const TIMES1_X0: i32 = 1745;
    const TIMES1_X1: i32 = TIMES1_X0 + 15;
    const TIMES_Y0: i32 = TEXT_Y0 + 6;
    const TIMES_Y1: i32 = TEXT_Y1 - 6;
    const DIGIT1_X0: i32 = TIMES1_X1 + 5;
    const DIGIT1_X1: i32 = DIGIT1_X0 + 14;

    const TIMES2_X0: i32 = TIMES1_X0 - 16;
    const TIMES2_X1: i32 = TIMES1_X1 - 16;
    const DIGIT2_X0: i32 = DIGIT1_X0 - 16;
    const DIGIT2_X1: i32 = DIGIT1_X1;

    const fn rect((x0, y0): (i32, i32), (x1, y1): (i32, i32)) -> ClipRect {
        ClipRect::from_points((x0 - X0, y0 - Y0), (x1 - X0, y1 - Y0), (WIDTH, HEIGHT))
    }

    const TOP_BLANK0: ClipRect = rect((1600, Y0), (1792, TEXT_Y0 - 1));
    const TOP_BLANK1: ClipRect = rect((1849, Y0), (1919, TEXT_Y0 - 1));

    const LAST_LETTER: ClipRect = rect((ITEM_X1 - ITEM_WIDTH / 6, TEXT_Y0), (ITEM_X1, TEXT_Y1));
    const TIMES1_LETTER: ClipRect = rect((TIMES1_X0, TIMES_Y0), (TIMES1_X1, TIMES_Y1));
    const DIGIT1_LETTER: ClipRect = rect((DIGIT1_X0, TEXT_Y0), (DIGIT1_X1, TEXT_Y1));

    const TIMES2_LETTER: ClipRect = rect((TIMES2_X0, TIMES_Y0), (TIMES2_X1, TIMES_Y1));
    const DIGIT2_LETTER: ClipRect = rect((DIGIT2_X0, TEXT_Y0), (DIGIT2_X1, TEXT_Y1));

    const ALL_BLANK1: &[ClipRect] = &[
        TOP_BLANK0,
        TOP_BLANK1,
        rect((ITEM_X1 + 1, TEXT_Y0), (TIMES1_X0 - 1, TEXT_Y1)),
        rect((TIMES1_X1 + 1, TEXT_Y0), (DIGIT1_X0 - 1, TEXT_Y1)),
        rect((DIGIT1_X1 + 1, TEXT_Y0), (1792, TEXT_Y1)),
        rect((TIMES1_X0, TEXT_Y0), (TIMES1_X1, TIMES_Y0 - 1)),
        rect((TIMES1_X0, TIMES_Y1 + 1), (TIMES1_X1, TEXT_Y1)),
    ];

    const ALL_BLANK2: &[ClipRect] = &[
        TOP_BLANK0,
        TOP_BLANK1,
        rect((ITEM_X1 + 1, TEXT_Y0), (TIMES2_X0 - 1, TEXT_Y1)),
        rect((TIMES2_X1 + 1, TEXT_Y0), (DIGIT2_X0 - 1, TEXT_Y1)),
        rect((DIGIT2_X1 + 1, TEXT_Y0), (1792, TEXT_Y1)),
        rect((TIMES2_X0, TEXT_Y0), (TIMES2_X1, TIMES_Y0 - 1)),
        rect((TIMES2_X0, TIMES_Y1 + 1), (TIMES2_X1, TEXT_Y1)),
    ];

    const fn bg(name: &'static str) -> HistogramThreshold {
        HistogramThreshold::new(name, &[([0..=6, 0..=6, 0..=6], 0..=6)], 1.00)
    }

    const fn letter(name: &'static str) -> HistogramThreshold {
        HistogramThreshold::new(name, &[([11..=15, 11..=15, 11..=15], 12..=15)], 0.010)
    }

    const fn times_letter(name: &'static str) -> HistogramThreshold {
        // `×` => 0.084375 = (12 + 12) / 16 * 16 * 0.9
        HistogramThreshold::new(name, &[([11..=15, 11..=15, 11..=15], 11..=15)], 0.084)
    }

    const fn digit_letter(name: &'static str) -> HistogramThreshold {
        HistogramThreshold::new(name, &[([12..=15, 12..=15, 12..=15], 12..=15)], 0.045)
    }

    &[
        &[
            (bg("BG"), ALL_BLANK1),
            (letter("LAST_LETTER"), &[LAST_LETTER]),
            (times_letter("TIMES_LETTER"), &[TIMES1_LETTER]),
            (digit_letter("DIGIT_LETTER"), &[DIGIT1_LETTER]),
        ],
        &[
            (bg("BG"), ALL_BLANK2),
            (letter("LAST_LETTER"), &[LAST_LETTER]),
            (times_letter("TIMES_LETTER"), &[TIMES2_LETTER]),
            (digit_letter("DIGIT_LETTER"), &[DIGIT2_LETTER]),
        ],
    ]
};

const TEXT_IN_BOX: &[(ClipRect, PostProcess, TextAlign)] = {
    const WIDTH: i32 = SIDE_ITEM_WIDTH;
    const HEIGHT: i32 = SIDE_ITEM_HEIGHT;

    const X0: i32 = SIDE_ITEM_X0_IN_FRAME;
    const Y0: i32 = SIDE_ITEM0_Y0_IN_FRAME;

    const fn rect((x0, y0): (i32, i32), (x1, y1): (i32, i32)) -> ClipRect {
        ClipRect::from_points((x0 - X0, y0 - Y0), (x1 - X0, y1 - Y0), (WIDTH, HEIGHT))
    }

    &[
        (
            rect((1365, 838), (1710, 865)),
            PostProcess::ItemText,
            TextAlign::Right,
        ),
        (
            rect((1765 - 3, 838), (1779 + 3, 865)),
            PostProcess::Digits,
            TextAlign::Unspecified,
        ),
        (
            rect((1749 - 3, 838), (1779 + 3, 865)),
            PostProcess::Digits,
            TextAlign::Unspecified,
        ),
    ]
};
