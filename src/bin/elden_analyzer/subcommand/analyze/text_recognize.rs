use std::sync::{LazyLock, Mutex};

use color_eyre::eyre;
use elden_analyzer::{
    components::{Component, ComponentContainer, Components, DetectionPayload, ExtractedTexts},
    image_process::tesseract::Tesseract,
    video_capture::{Frame, FramePosition},
};
use lockfree_object_pool::LinearObjectPool;

use super::comp_accum::{self, AccumDetection};

#[derive(Debug)]
pub(super) enum Packet {
    Frame {
        pos: FramePosition,
        result: Box<ComponentContainer<Option<ExtractedTexts>>>,
    },
    EndOfFrames {
        pos: FramePosition,
    },
}

impl Packet {
    pub(super) fn position(&self) -> FramePosition {
        match self {
            Self::Frame { pos, .. } => *pos,
            Self::EndOfFrames { pos } => *pos,
        }
    }
}

#[tracing::instrument(name = "text_recognize", level = "trace", skip_all, fields(pos = %packet.position()))]
pub(super) fn run(
    components: &Components,
    tess: &LinearObjectPool<LazyLock<Mutex<Tesseract>, impl FnOnce() -> Mutex<Tesseract>>>,
    packet: comp_accum::Packet,
) -> eyre::Result<Packet> {
    let packet = match packet {
        comp_accum::Packet::Frame { pos, frame, result } => {
            let result = result
                .into_iter()
                .zip(components)
                .map(
                    |(found, component)| -> eyre::Result<Option<ExtractedTexts>> {
                        let payload = match found {
                            AccumDetection::Found(payload) => payload,
                            AccumDetection::Absent => return Ok(None),
                        };
                        let text = recognize(&**component, tess, pos, &frame, payload)?;
                        Ok(Some(text))
                    },
                )
                .collect::<eyre::Result<_>>()?;
            let result = Box::new(result);
            Packet::Frame { pos, result }
        }
        comp_accum::Packet::EndOfFrames { pos } => Packet::EndOfFrames { pos },
    };
    Ok(packet)
}

fn recognize(
    component: &dyn Component,
    tess: &LinearObjectPool<LazyLock<Mutex<Tesseract>, impl FnOnce() -> Mutex<Tesseract>>>,
    pos: FramePosition,
    frame: &Frame,
    payload: Option<DetectionPayload>,
) -> eyre::Result<ExtractedTexts> {
    let tess = tess.pull();
    let mut tess = tess.lock().unwrap();
    let result = component.extract_text(&mut tess, frame, payload)?;
    tracing::trace!(name = component.name(), %pos, ?result);
    Ok(result)
}
