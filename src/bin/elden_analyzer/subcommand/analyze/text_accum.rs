use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet, VecDeque},
    fs::File,
    io::Write as _,
    iter,
    sync::mpsc,
};

use color_eyre::eyre;

use elden_analyzer::{
    components::{ComponentContainer, ExtractedTexts},
    operator::Recognition,
};
use elden_analyzer_collections::seq_iter::SeqIter;
use elden_analyzer_kernel::types::time::{Duration, FramePosition};
use num_rational::Ratio;

use super::text_recognize::{self};

#[tracing::instrument(name = "text_accum", level = "debug", skip_all)]
pub(super) fn run(
    rx: mpsc::Receiver<(usize, text_recognize::Packet)>,
    start: FramePosition,
    sec_per_frame: Duration,
    mut output_span: Option<File>,
    mut output_tsv: Option<File>,
) -> eyre::Result<()> {
    let mut check_pos = start;
    let mut last_updated = start;
    let mut accum = ComponentContainer::from_fn(Accumulator::new);

    let mut write_span = |result| -> eyre::Result<()> {
        let AccumResult {
            name,
            start,
            end,
            text,
        } = result;

        tracing::info!(
            name,
            "{start}-{end} {text}",
            start = start.timestamp(),
            end = end.timestamp()
        );
        if let Some(output) = &mut output_span {
            writeln!(
                output,
                "{start}-{end} {text} ({name})",
                start = start.timestamp(),
                end = end.timestamp()
            )?;
        }
        Ok(())
    };

    if let Some(output) = &mut output_tsv {
        let header_text = accum
            .iter()
            .map(|accum| accum.name)
            .collect::<Vec<_>>()
            .join("\t");
        writeln!(output, "timestamp\t{header_text}")?;
    }

    let mut write_tsv = |start: FramePosition, results: Vec<&str>| -> eyre::Result<()> {
        tracing::debug!("{start} {results:?}", start = start.timestamp(),);
        if let Some(output) = &mut output_tsv {
            let results_text = results.join("\t");
            writeln!(output, "{start}\t{results_text}", start = start.timestamp(),)?;
        }
        Ok(())
    };

    for (_i, packet) in SeqIter::new(rx) {
        let pos = packet.position();
        let _span = tracing::trace_span!("frame", %pos).entered();

        match packet {
            text_recognize::Packet::Frame { pos, result } => {
                for (accum, result) in accum.iter_mut().zip(*result) {
                    if let Some(result) = accum.receive_frame(pos, result) {
                        write_span(result)?;
                    }
                }
            }
            text_recognize::Packet::EndOfFrames { pos } => {
                for accum in &mut accum {
                    if let Some(result) = accum.receive_end_of_frames(pos) {
                        write_span(result)?;
                    }
                }
            }
        }

        while check_pos.index() <= pos.index() {
            let all_available = accum
                .iter()
                .all(|accum| accum.prev_span_available(check_pos));
            if !all_available {
                break;
            }

            let mut updated = false;
            for accum in &accum {
                updated |= accum.is_span_end(check_pos);
            }

            if updated {
                let results = accum
                    .iter()
                    .map(|accum| accum.prev_span_result(check_pos).unwrap_or(""))
                    .collect::<Vec<_>>();
                write_tsv(last_updated, results)?;
                last_updated = check_pos;
            }

            check_pos = check_pos.next(sec_per_frame);
            for accum in &mut accum {
                accum.seek_result_to(check_pos);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct AccumResult {
    name: &'static str,
    start: FramePosition,
    end: FramePosition,
    text: String,
}

#[derive(Debug)]
struct Accumulator {
    name: &'static str,
    end_of_frames: Option<FramePosition>,
    found_start: Option<FramePosition>,
    accum: Vec<InnerAccumulator>,
    results: VecDeque<AccumResult>,
}

impl Accumulator {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            end_of_frames: None,
            found_start: None,
            accum: vec![],
            results: VecDeque::new(),
        }
    }

    fn receive_frame(
        &mut self,
        pos: FramePosition,
        result: Option<ExtractedTexts>,
    ) -> Option<AccumResult> {
        match result {
            Some(text) => self.handle_found(pos, text),
            None => self.handle_absent(pos),
        }
    }

    fn receive_end_of_frames(&mut self, pos: FramePosition) -> Option<AccumResult> {
        self.end_of_frames = Some(pos);
        self.handle_absent(pos)
    }

    fn prev_span_available(&self, end: FramePosition) -> bool {
        if self.end_of_frames.is_some() {
            return true;
        }

        let in_found_span = self
            .found_start
            .map_or(false, |start| start.index() >= end.index());
        !in_found_span
    }

    fn prev_span_result(&self, end_pos: FramePosition) -> Option<&str> {
        self.results
            .front()
            .filter(|result| {
                result.start.index() < end_pos.index() && result.end.index() >= end_pos.index()
            })
            .map(|res| res.text.as_str())
    }

    fn is_span_end(&self, end: FramePosition) -> bool {
        self.end_of_frames
            .map_or(false, |eof| eof.index() == end.index())
            || self
                .found_start
                .map_or(false, |start| start.index() == end.index())
            || self.results.front().map_or(false, |result| {
                result.start.index() == end.index() || result.end.index() == end.index()
            })
    }

    fn seek_result_to(&mut self, pos: FramePosition) {
        while self
            .results
            .front()
            .map_or(false, |result| result.end.index() < pos.index())
        {
            self.results.pop_front();
        }
    }

    fn handle_found(&mut self, pos: FramePosition, text: ExtractedTexts) -> Option<AccumResult> {
        if self.found_start.is_none() {
            self.found_start = Some(pos);
        }

        if self.accum.is_empty() {
            self.accum
                .extend(iter::repeat_with(Default::default).take(text.result.len()));
        }
        assert_eq!(self.accum.len(), text.result.len());

        for (accum, result) in self.accum.iter_mut().zip(text.result) {
            accum.insert(result);
        }

        None
    }

    fn handle_absent(&mut self, pos: FramePosition) -> Option<AccumResult> {
        let start = self.found_start.take()?;
        let end = pos;

        let mut segments = vec![];
        for accum in &mut self.accum {
            let text = accum.get_text();
            segments.push(text);
            accum.reset();
        }

        let result = AccumResult {
            name: self.name,
            start,
            end,
            text: segments.join(" "),
        };
        self.results.push_back(result.clone());
        Some(result)
    }
}

#[derive(Debug, Default)]
struct InnerAccumulator {
    found: HashSet<String>,
    possible: HashMap<String, Ratio<i32>>,
}

impl InnerAccumulator {
    fn insert(&mut self, result: Recognition) {
        match result {
            Recognition::Found(text, _) => {
                self.found.insert(text);
            }
            Recognition::Possible(text, conf) => {
                *self.possible.entry(text).or_default() += conf.as_ratio();
            }
        }
    }

    fn get_text(&self) -> String {
        if !self.found.is_empty() {
            return join_texts(self.found.iter().map(|s| s.as_str()));
        }

        let total_conf = self.possible.values().sum::<Ratio<i32>>();

        let mut texts = self
            .possible
            .iter()
            .map(|(text, conf)| (format!("??{text}"), *conf))
            .collect::<Vec<_>>();
        let threshold = total_conf * Ratio::new(1, 10);

        texts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let filtered = texts
            .iter()
            .filter(|(_, weight)| *weight >= threshold)
            .collect::<Vec<_>>();
        tracing::debug!(threshold = ?threshold, ?filtered, ?texts);

        if filtered.is_empty() {
            return join_texts(texts.iter().map(|(text, _)| text.as_str()));
        }

        join_texts(filtered.iter().map(|(text, _)| text.as_str()))
    }

    fn reset(&mut self) {
        self.found.clear();
        self.possible.clear();
    }
}

fn join_texts<S, I>(texts: I) -> String
where
    I: IntoIterator<Item = S>,
    S: Borrow<str>,
{
    const SEP: &str = "|";
    const OPEN: &str = "{";
    const CLOSE: &str = "}";

    let mut texts = texts.into_iter();
    let Some(first) = texts.next() else {
        return format!("{OPEN}{CLOSE}");
    };
    let Some(second) = texts.next() else {
        return first.borrow().to_owned();
    };

    let mut result = format!("{OPEN}{}{SEP}{}", first.borrow(), second.borrow());
    for text in texts {
        result += SEP;
        result += text.borrow();
    }

    result += CLOSE;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_texts() {
        assert_eq!(join_texts::<&str, _>([]), "{}");
        assert_eq!(join_texts(["a"]), "a");
        assert_eq!(join_texts(["a", "b"]), "{a|b}");
    }
}
