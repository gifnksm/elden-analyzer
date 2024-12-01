use std::{fmt, str::FromStr};

use num_rational::Ratio;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    dur: Ratio<i64>,
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_sec = self.dur.trunc().to_integer();
        let msec = (self.dur.fract() * Ratio::from_integer(1000)).to_integer();
        let hour = total_sec / 3600;
        let min = (total_sec / 60) % 60;
        let sec = total_sec % 60;
        write!(f, "{hour:02}:{min:02}:{sec:02}.{msec:03}")
    }
}

impl Duration {
    pub fn new(dur: Ratio<i64>) -> Self {
        Self { dur }
    }

    pub fn as_ratio(&self) -> Ratio<i64> {
        self.dur
    }

    pub fn as_msec(&self) -> i64 {
        (self.dur * Ratio::from_integer(1000)).to_integer()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    ts: Ratio<i64>,
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_sec = self.ts.trunc().to_integer();
        let msec = (self.ts.fract() * Ratio::from_integer(1000)).to_integer();
        let hour = total_sec / 3600;
        let min = (total_sec / 60) % 60;
        let sec = total_sec % 60;
        write!(f, "{hour:02}:{min:02}:{sec:02}.{msec:03}")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TimestampParseError {
    #[error("Invalid format")]
    InvalidFormat,
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}

impl FromStr for Timestamp {
    type Err = TimestampParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':').rev();

        let sec_parts = parts.next().ok_or(Self::Err::InvalidFormat)?;
        let (sec, msec) = sec_parts.split_once('.').unwrap_or((sec_parts, "000"));
        let sec = sec.parse::<i64>()?;
        let msec = msec.parse::<i64>()?;

        let min = if let Some(min_parts) = parts.next() {
            min_parts.parse::<i64>()?
        } else {
            0
        };

        let hour = if let Some(hour_parts) = parts.next() {
            hour_parts.parse::<i64>()?
        } else {
            0
        };

        if parts.next().is_some() {
            return Err(Self::Err::InvalidFormat);
        }

        let ts = Timestamp::new(Ratio::new(
            hour * 3600 * 1000 + min * 60 * 1000 + sec * 1000 + msec,
            1000,
        ));
        Ok(ts)
    }
}

impl Timestamp {
    pub fn new(ts: Ratio<i64>) -> Self {
        Self { ts }
    }

    pub fn as_ratio(&self) -> Ratio<i64> {
        self.ts
    }
}

impl std::ops::Sub for Timestamp {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        Duration::new(self.ts - rhs.ts)
    }
}

impl std::ops::Add<Duration> for Timestamp {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self::new(self.ts + rhs.dur)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TimestampRange {
    Full,
    Single(Timestamp),
    Range(Timestamp, Timestamp),
    RangeFrom(Timestamp),
    RangeTo(Timestamp),
}

impl FromStr for TimestampRange {
    type Err = TimestampParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((start, end)) = s.split_once('-') {
            let start = (!start.is_empty()).then(|| start.parse()).transpose()?;
            let end = (!end.is_empty()).then(|| end.parse()).transpose()?;
            match (start, end) {
                (Some(start), Some(end)) => {
                    if start <= end {
                        return Ok(Self::Range(start, end));
                    }
                    return Err(Self::Err::InvalidFormat);
                }
                (Some(start), None) => return Ok(Self::RangeFrom(start)),
                (None, Some(end)) => return Ok(Self::RangeTo(end)),
                (None, None) => return Ok(Self::Full),
            }
        }

        let ts = Timestamp::from_str(s)?;
        Ok(Self::Single(ts))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FramePosition {
    idx: usize,
    ts: Timestamp,
}

impl fmt::Display for FramePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.ts, self.idx)
    }
}

impl FramePosition {
    pub fn new(idx: usize, ts: Timestamp) -> Self {
        Self { idx, ts }
    }

    pub fn index(&self) -> usize {
        self.idx
    }

    pub fn timestamp(&self) -> Timestamp {
        self.ts
    }

    pub fn next(&self, sec_per_frame: Duration) -> FramePosition {
        Self::new(self.idx + 1, self.ts + sec_per_frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timestamp() {
        fn p(s: &str) -> String {
            s.parse::<Timestamp>().unwrap().to_string()
        }
        assert_eq!(p("01:23:45.678"), "01:23:45.678");
        assert_eq!(p("01:23:45"), "01:23:45.000");
        assert_eq!(p("3672"), "01:01:12.000");
    }
}
