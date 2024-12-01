use std::path::Path;

use color_eyre::eyre;
use elden_analyzer::{
    components::{ComponentContainer, Components},
    operator::DetectionKind,
    util::ImageLogger,
};
use elden_analyzer_video::capture::{Frame, VideoCapture};

fn load_image(path: impl AsRef<Path>) -> eyre::Result<Frame> {
    let mut capture = VideoCapture::open(path.as_ref())?;
    let mut frame = Frame::empty();
    if !capture.decode_frame(&mut frame)? {
        eyre::bail!("cannot found frame");
    }
    Ok(frame)
}

fn detect_components(path: impl AsRef<Path>) -> eyre::Result<ComponentContainer<DetectionKind>> {
    let frame = load_image(path)?;
    let components = Components::new(frame.rect()).unwrap();
    components
        .iter()
        .map(|c| c.detect(&frame).map(|res| res.kind()))
        .collect()
}

#[test]
fn test() -> eyre::Result<()> {
    use DetectionKind::*;

    ImageLogger::init(false)?;

    let res = detect_components("tests/assets/item_legend0.png")?;
    assert_eq!(res.main_item, Found);
    assert_eq!(res.side_item[0], Found);
    for i in 1..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_legend1.png")?;
    assert_eq!(res.main_item, Found);
    assert_eq!(res.side_item[0], Found);
    for i in 1..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_legend2.png")?;
    assert_eq!(res.main_item, Possible);
    assert_eq!(res.side_item[0], Found);
    assert_eq!(res.side_item[1], Found);
    for i in 2..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_rare0.png")?;
    assert_eq!(res.main_item, Found);
    assert_eq!(res.side_item[0], Found);
    assert_eq!(res.side_item[1], Found);
    assert_eq!(res.side_item[2], Found);
    assert_eq!(res.side_item[3], Found);
    assert_eq!(res.side_item[4], Found);
    assert_eq!(res.side_item[5], Found);
    for i in 6..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_rare1.png")?;
    assert_eq!(res.main_item, Found);
    assert_eq!(res.side_item[0], Found);
    assert_eq!(res.side_item[1], Found);
    assert_eq!(res.side_item[2], Found);
    assert_eq!(res.side_item[3], Found);
    for i in 4..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_rare2.png")?;
    assert_eq!(res.main_item, Found);
    assert_eq!(res.side_item[0], Found);
    for i in 1..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_common0.png")?;
    assert_eq!(res.main_item, Absent);
    assert_eq!(res.side_item[0], Found);
    for i in 1..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_common1.png")?;
    assert_eq!(res.main_item, Found);
    assert_eq!(res.side_item[0], Found);
    for i in 1..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_common2.png")?;
    assert_eq!(res.main_item, Found);
    assert_eq!(res.side_item[0], Found);
    assert_eq!(res.side_item[1], Found);
    for i in 2..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_common3.png")?;
    assert_eq!(res.main_item, Absent);
    assert_eq!(res.side_item[0], Found);
    for i in 1..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_common4.png")?;
    assert_eq!(res.main_item, Found);
    assert_eq!(res.side_item[0], Found);
    for i in 1..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/item_common5.png")?;
    assert_eq!(res.main_item, Absent);
    assert_eq!(res.side_item[0], Absent);
    for i in 1..8 {
        assert_eq!(res.side_item[i], Found, "{i}");
    }
    for i in 9..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/no_item0.png")?;
    assert_eq!(res.main_item, Absent);
    for i in 0..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/no_item1.png")?;
    assert_eq!(res.main_item, Absent);
    for i in 0..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    let res = detect_components("tests/assets/no_item2.png")?;
    assert_eq!(res.main_item, Absent);
    for i in 0..res.side_item.len() {
        assert_eq!(res.side_item[i], Absent, "{i}");
    }

    Ok(())
}
