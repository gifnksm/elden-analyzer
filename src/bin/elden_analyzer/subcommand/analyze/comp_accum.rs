use std::{collections::VecDeque, sync::mpsc};

use color_eyre::eyre;
use elden_analyzer::{
    components::{ComponentContainer, Detection, DetectionPayload},
    video_capture::{Frame, FramePosition},
};
use elden_analyzer_collections::SeqIter;

use super::comp_detect;

#[derive(Debug)]
pub(super) enum AccumDetection {
    Found(Option<DetectionPayload>),
    Absent,
}

#[derive(Debug)]
pub(super) enum Packet {
    Frame {
        pos: FramePosition,
        frame: Frame,
        result: Box<ComponentContainer<AccumDetection>>,
    },
    EndOfFrames {
        pos: FramePosition,
    },
}

impl Packet {
    pub(super) fn position(&self) -> FramePosition {
        match self {
            Packet::Frame { pos, .. } => *pos,
            Packet::EndOfFrames { pos } => *pos,
        }
    }
}

#[tracing::instrument(name = "comp_accum", level = "debug", skip_all)]
pub(super) fn run(
    comp_detect_rx: mpsc::Receiver<(usize, comp_detect::Packet)>,
    comp_accum_tx: mpsc::Sender<(usize, Packet)>,
) -> eyre::Result<()> {
    let mut j = 0;
    let mut send_packet = move |packet| -> eyre::Result<()> {
        comp_accum_tx.send((j, packet))?;
        j += 1;
        Ok(())
    };

    let mut accum = ComponentContainer::from_fn(Accumulator::new);
    let mut pending_packets = VecDeque::new();

    for (_i, packet) in SeqIter::new(comp_detect_rx) {
        let pos = packet.position();
        let _span = tracing::trace_span!("frame", %pos).entered();

        match packet {
            comp_detect::Packet::Frame { pos, frame, result } => {
                pending_packets.push_back((pos, Some(frame)));
                for (accum, result) in accum.iter_mut().zip(*result) {
                    accum.receive_frame(pos, result);
                }
            }
            comp_detect::Packet::EndOfFrames { pos } => {
                pending_packets.push_back((pos, None));
                for accum in &mut accum {
                    accum.receive_end_of_frames(pos);
                }
            }
        }

        while let Some((pos, _)) = pending_packets.front() {
            let completed = accum.iter().all(|accum| accum.handled(*pos));
            if !completed {
                break;
            }

            let (pos, frame) = pending_packets.pop_front().unwrap();
            let result = accum
                .iter_mut()
                .map(|accum| accum.pop_packet().unwrap().1)
                .collect();
            let result = Box::new(result);

            if let Some(frame) = frame {
                send_packet(Packet::Frame { pos, frame, result })?;
            } else {
                assert!(result
                    .iter()
                    .all(|result| matches!(result, AccumDetection::Absent)));
                send_packet(Packet::EndOfFrames { pos })?;
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Accumulator {
    name: &'static str,

    pending_packets: VecDeque<(usize, AccumDetection)>,
    found_start: Option<FramePosition>,
    last_found: usize,
    possibles: VecDeque<(FramePosition, Option<DetectionPayload>)>,
}

impl Accumulator {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            pending_packets: VecDeque::new(),
            found_start: None,
            last_found: usize::MAX,
            possibles: VecDeque::new(),
        }
    }

    fn handled(&self, pos: FramePosition) -> bool {
        self.pending_packets
            .front()
            .map(|(idx, _)| *idx == pos.index())
            .unwrap_or_default()
    }

    fn pop_packet(&mut self) -> Option<(usize, AccumDetection)> {
        self.pending_packets.pop_front()
    }

    fn receive_frame(&mut self, pos: FramePosition, result: Detection) {
        match (result, self.last_found == pos.index() - 1) {
            (Detection::Found(payload), _) | (Detection::Possible(payload), true) => {
                self.handle_found(pos, payload);
            }
            (Detection::Possible(payload), false) => {
                self.handle_possible(pos, payload);
            }
            (Detection::Absent, _) => {
                self.handle_absent(pos);
            }
        }
    }

    fn receive_end_of_frames(&mut self, pos: FramePosition) {
        self.handle_absent(pos);
    }

    fn handle_found(&mut self, pos: FramePosition, payload: Option<DetectionPayload>) {
        self.last_found = pos.index();
        if self.found_start.is_none() {
            if let Some((pos, _)) = self.possibles.front() {
                self.found_start = Some(*pos);
            } else {
                self.found_start = Some(pos);
            }
        }
        self.pending_packets.extend(
            self.possibles
                .drain(..)
                .map(|(pos, payload)| (pos.index(), AccumDetection::Found(payload))),
        );
        self.pending_packets
            .push_back((pos.index(), AccumDetection::Found(payload)));
    }

    fn handle_possible(&mut self, pos: FramePosition, payload: Option<DetectionPayload>) {
        const EXPIRE_FRAMES: usize = 60;
        self.possibles.push_back((pos, payload));
        let drain_count = self
            .possibles
            .iter()
            .take_while(|(pkt_pos, _)| pkt_pos.index() + EXPIRE_FRAMES < pos.index())
            .count();
        self.pending_packets.extend(
            self.possibles
                .drain(..drain_count)
                .map(|(pos, _)| (pos.index(), AccumDetection::Absent)),
        );
    }

    fn handle_absent(&mut self, pos: FramePosition) {
        self.pending_packets.extend(
            self.possibles
                .drain(..)
                .map(|(pos, _)| (pos.index(), AccumDetection::Absent)),
        );
        self.pending_packets
            .push_back((pos.index(), AccumDetection::Absent));

        if let Some(start) = self.found_start.take() {
            assert!(self.possibles.is_empty());
            let end = pos;
            tracing::debug!(name = self.name, %start, %end, "found UI");
        }
    }
}
