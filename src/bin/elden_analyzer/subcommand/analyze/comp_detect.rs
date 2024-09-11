use std::time::Instant;

use color_eyre::eyre;
use elden_analyzer::{
    components::{Component, ComponentContainer, Components, Detection},
    video_capture::{Frame, FramePosition},
};

use super::decode;

#[derive(Debug)]
pub(super) enum Packet {
    Frame {
        pos: FramePosition,
        frame: Frame,
        result: Box<ComponentContainer<Detection>>,
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

#[tracing::instrument(name = "comp_detect", level = "trace", skip_all, fields(pos = %packet.position()))]
pub(super) fn run(components: &Components, packet: decode::Packet) -> eyre::Result<Packet> {
    let packet = match packet {
        decode::Packet::Frame { pos, frame } => {
            let result = components
                .iter()
                .map(|component| judge(&**component, &frame))
                .collect::<eyre::Result<_>>()?;
            let result = Box::new(result);
            Packet::Frame { pos, frame, result }
        }
        decode::Packet::EndOfFrames { pos } => Packet::EndOfFrames { pos },
    };
    Ok(packet)
}

fn judge(component: &dyn Component, rgb_frame: &Frame) -> eyre::Result<Detection> {
    let start = Instant::now();
    let result = component.detect(rgb_frame)?;
    let elapsed = start.elapsed();

    tracing::trace!(name = component.name(), result = %result.kind(), ?elapsed);

    Ok(result)
}
