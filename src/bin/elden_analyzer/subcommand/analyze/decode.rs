use std::sync::mpsc;

use color_eyre::eyre;
use elden_analyzer::video_capture::{Frame, RangeDecoder};
use elden_analyzer_kernel::types::time::FramePosition;

use crate::tui::ProgressBar;

#[derive(Debug)]
pub(super) enum Packet {
    Frame { pos: FramePosition, frame: Frame },
    EndOfFrames { pos: FramePosition },
}

impl Packet {
    pub(super) fn position(&self) -> FramePosition {
        match self {
            Packet::Frame { pos, .. } => *pos,
            Packet::EndOfFrames { pos } => *pos,
        }
    }
}

#[tracing::instrument(name = "decode", level = "debug", skip_all)]
pub(super) fn run(
    pbar: &ProgressBar,
    cap_tx: mpsc::Sender<(usize, Packet)>,
    decoder: &mut RangeDecoder,
) -> eyre::Result<()> {
    let mut next_pos = decoder.start();
    for i in 0.. {
        let _span = tracing::trace_span!("frame", pos = %next_pos).entered();

        let mut frame = Frame::empty();
        if !decoder.decode_frame(&mut frame)? {
            let pos = decoder.end();
            let packet = Packet::EndOfFrames { pos };
            cap_tx.send((i, packet)).unwrap();
            pbar.set_position(pos);
            break;
        }

        let pos = frame.position();
        let packet = Packet::Frame { pos, frame };
        cap_tx.send((i, packet)).unwrap();
        pbar.set_position(pos);
        next_pos = pos.next(decoder.capture().sec_per_frame());
    }

    Ok(())
}
