use std::{iter, ops::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Segment {
    pub vote: u32,
    pub start: i32,
    pub end: i32,
}

impl Segment {
    fn new(i: i32) -> Self {
        Self {
            vote: 1,
            start: i,
            end: i + 1,
        }
    }

    pub fn range(&self) -> Range<i32> {
        self.start..self.end
    }
}

impl From<Range<i32>> for Segment {
    fn from(r: Range<i32>) -> Self {
        Self {
            vote: (r.end - r.start) as u32,
            start: r.start,
            end: r.end,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FindLineSegments {
    pub vote_threshold: u32,
    pub min_line_len: i32,
    pub max_line_gap: i32,
}

impl FindLineSegments {
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn find(&self, seq: impl IntoIterator<Item = bool>) -> impl Iterator<Item = Range<i32>> {
        find_line_segments(
            seq,
            self.vote_threshold,
            self.min_line_len,
            self.max_line_gap,
        )
    }
}

pub fn find_line_segments(
    seq: impl IntoIterator<Item = bool>,
    vote_threshold: u32,
    min_line_len: i32,
    max_line_gap: i32,
) -> impl Iterator<Item = Range<i32>> {
    let it = seq.into_iter();
    let it = continuous_points(it);
    let it = join_segments(it, max_line_gap);
    filter_segments(it, vote_threshold, min_line_len)
}

fn filter_segments(
    segments: impl IntoIterator<Item = Segment>,
    vote_threshold: u32,
    min_line_len: i32,
) -> impl Iterator<Item = Range<i32>> {
    segments
        .into_iter()
        .filter(move |seg| seg.vote >= vote_threshold && seg.end - seg.start >= min_line_len)
        .map(|seg| seg.range())
}

fn join_segments(
    segments: impl IntoIterator<Item = Segment>,
    max_line_gap: i32,
) -> impl Iterator<Item = Segment> {
    let mut it = segments.into_iter().fuse();
    let mut state: Option<Segment> = None;
    iter::from_fn(move || loop {
        match it.next() {
            Some(seg) => match &mut state {
                Some(prev_seg) if seg.start - prev_seg.end <= max_line_gap => {
                    prev_seg.end = seg.end;
                    prev_seg.vote += seg.vote;
                    continue;
                }
                Some(prev_seg) => {
                    let ret_seg = *prev_seg;
                    state = Some(seg);
                    return Some(ret_seg);
                }
                None => {
                    state = Some(seg);
                    continue;
                }
            },
            None => return state.take(),
        }
    })
}

fn continuous_points(pts: impl IntoIterator<Item = bool>) -> impl Iterator<Item = Segment> {
    let mut it = (0..).zip(pts).fuse();
    let mut state: Option<Segment> = None;
    iter::from_fn(move || loop {
        match it.next() {
            Some((i, true)) => match &mut state {
                Some(seg) if seg.end == i => {
                    seg.vote += 1;
                    seg.end = i + 1;
                    continue;
                }
                Some(seg) => {
                    let seg = *seg;
                    state = Some(Segment::new(i));
                    return Some(seg);
                }
                None => {
                    state = Some(Segment::new(i));
                }
            },
            Some((_, false)) => {
                if let Some(seg) = state.take() {
                    return Some(seg);
                }
                continue;
            }
            None => return state.take(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn test_continuous_points() {
        fn cp(seq: impl IntoIterator<Item = bool>) -> Vec<Segment> {
            continuous_points(seq).collect()
        }

        let seq = [true; 10];
        assert_eq!(cp(seq), [(0..10).into()]);

        let seq = [false; 10];
        assert_eq!(cp(seq), []);

        let seq = [false, true, false, true, false, true, true, true];
        assert_eq!(cp(seq), [(1..2).into(), (3..4).into(), (5..8).into()]);

        let seq = [
            true, false, true, true, false, false, true, false, false, false, true, true, false,
        ];
        assert_eq!(
            cp(seq),
            [(0..1).into(), (2..4).into(), (6..7).into(), (10..12).into()]
        );
    }

    #[test]
    fn test_join_segments() {
        fn js(seq: impl IntoIterator<Item = bool>, max_line_gap: i32) -> Vec<Segment> {
            join_segments(continuous_points(seq), max_line_gap).collect()
        }

        let seq = [true; 10];
        assert_eq!(js(seq, 0), [(0..10).into()]);

        let seq = [false; 10];
        assert_eq!(js(seq, 0), []);

        let seq = [false, true, false, true, false, true, true, true];
        assert_eq!(js(seq, 0), [(1..2).into(), (3..4).into(), (5..8).into()]);
        assert_eq!(
            js(seq, 1),
            [Segment {
                vote: 5,
                start: 1,
                end: 8
            }]
        );

        let seq = [
            true, false, true, true, false, false, true, false, false, false, true, true, false,
        ];
        assert_eq!(
            js(seq, 0),
            [(0..1).into(), (2..4).into(), (6..7).into(), (10..12).into()]
        );
        assert_eq!(
            js(seq, 1),
            [
                Segment {
                    vote: 3,
                    start: 0,
                    end: 4
                },
                (6..7).into(),
                (10..12).into()
            ]
        );
        assert_eq!(
            js(seq, 2),
            [
                Segment {
                    vote: 4,
                    start: 0,
                    end: 7
                },
                (10..12).into()
            ]
        );
        assert_eq!(
            js(seq, 3),
            [Segment {
                vote: 6,
                start: 0,
                end: 12
            }]
        );
    }
}
