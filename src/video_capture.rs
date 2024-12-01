use std::{path::Path, ptr};

use elden_analyzer_kernel::types::{
    rect::Rect,
    time::{Duration, FramePosition, Timestamp, TimestampRange},
};
use ffmpeg::{
    codec, decoder, format, frame, media, rescale::TIME_BASE, software::scaling, threading, Packet,
    Stream,
};
use imageproc::image::{ImageBuffer, Luma, Pixel as _, Rgb};
use num_rational::Ratio;
use num_traits::Signed;
use tracing::{debug, trace};

use crate::traits::ToRatio;

#[derive(custom_debug::Debug)]
pub struct Frame {
    pos: FramePosition,
    dur: Duration,
    #[debug(skip)]
    data: frame::Video,
}

impl Frame {
    pub fn empty() -> Self {
        Self {
            pos: FramePosition::default(),
            dur: Duration::default(),
            data: frame::Video::empty(),
        }
    }

    pub fn position(&self) -> FramePosition {
        self.pos
    }

    pub fn duration(&self) -> Duration {
        self.dur
    }

    pub fn data(&self, index: usize) -> &[u8] {
        self.data.data(index)
    }

    pub fn width(&self) -> u32 {
        self.data.width()
    }

    pub fn height(&self) -> u32 {
        self.data.height()
    }

    pub fn rect(&self) -> Rect {
        Rect::at(0, 0).of_size(self.width(), self.height())
    }

    pub fn to_rgb_image(&self) -> ImageBuffer<Rgb<u8>, &[u8]> {
        ImageBuffer::from_raw(self.width(), self.height(), self.data(0)).unwrap()
    }

    pub fn to_rgb_image_within(&self, rect: Rect) -> Option<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        let frame_rect = Rect::at(0, 0).of_size(self.width(), self.height());
        let rect = rect.intersect(frame_rect)?;
        let img = ImageBuffer::from_fn(rect.width(), rect.height(), |dx, dy| {
            let x = rect.left() as u32 + dx;
            let y = rect.top() as u32 + dy;
            let idx = ((y * self.width() + x) * 3) as usize;
            *Rgb::from_slice(&self.data(0)[idx..][..3])
        });
        Some(img)
    }

    pub fn to_gray_image(&self) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        ImageBuffer::from_fn(self.width(), self.height(), |x, y| {
            let idx = ((y * self.width() + x) * 3) as usize;
            let p = Rgb::from_slice(&self.data(0)[idx..][..3]);
            p.to_luma()
        })
    }

    pub fn to_gray_image_within(&self, rect: Rect) -> Option<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let frame_rect = Rect::at(0, 0).of_size(self.width(), self.height());
        let rect = rect.intersect(frame_rect)?;
        let img = ImageBuffer::from_fn(rect.width(), rect.height(), |dx, dy| {
            let x = rect.left() as u32 + dx;
            let y = rect.top() as u32 + dy;
            let idx = ((y * self.width() + x) * 3) as usize;
            let p = Rgb::from_slice(&self.data(0)[idx..][..3]);
            p.to_luma()
        });
        Some(img)
    }

    pub fn to_min_gray_image_within(&self, rect: Rect) -> Option<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let frame_rect = Rect::at(0, 0).of_size(self.width(), self.height());
        let rect = rect.intersect(frame_rect)?;
        let img = ImageBuffer::from_fn(rect.width(), rect.height(), |dx, dy| {
            let x = rect.left() as u32 + dx;
            let y = rect.top() as u32 + dy;
            let idx = ((y * self.width() + x) * 3) as usize;
            let v = self.data(0)[idx..][..3].iter().min().unwrap();
            [*v].into()
        });
        Some(img)
    }
}

#[derive(custom_debug::Debug)]
pub struct VideoCapture {
    dur: Duration,
    fps: Ratio<i64>,
    frames: usize,
    stream_time_base: Ratio<i64>,
    width: u32,
    height: u32,

    #[debug(skip)]
    ictx: format::context::Input,
    video_stream_idx: usize,
    #[debug(skip)]
    decoder: decoder::Video,
    #[debug(skip)]
    decoded: frame::Video,
    #[debug(skip)]
    scaler: scaling::Context,
    packet_sent: bool,
    skip_until: Option<Timestamp>,
    last_decoded: Option<FramePosition>,
}

impl VideoCapture {
    pub fn open(file: &Path) -> Result<Self, ffmpeg::Error> {
        let mut ictx = format::input(&file)?;

        let video_stream_idx = ictx
            .streams()
            .best(media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?
            .index();

        let mut context_decoder = ffmpeg::codec::context::Context::from_parameters(
            ictx.stream(video_stream_idx).unwrap().parameters(),
        )?;
        #[allow(clippy::needless_update)]
        context_decoder.set_threading(codec::threading::Config {
            kind: threading::Type::Frame,
            count: 16,
            ..Default::default() // for FFMPEG other than 6.0
        });
        let decoder = context_decoder.decoder().video()?;

        let scaler = scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            format::Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            scaling::Flags::BILINEAR,
        )?;

        let decoded = frame::Video::empty();

        let fps = get_fps(&mut ictx, video_stream_idx).unwrap_or(Ratio::ONE);
        let frames = get_frames(&mut ictx, video_stream_idx).unwrap_or(1) as usize;
        let duration = Duration::new(
            get_duration(&ictx, video_stream_idx)
                .unwrap_or_else(|| Ratio::from_integer(frames as i64) / fps),
        );
        let stream_time_base = ictx
            .stream(video_stream_idx)
            .unwrap()
            .time_base()
            .to_ratio();

        debug!(%duration, %fps, %frames);

        Ok(Self {
            fps,
            dur: duration,
            frames,
            stream_time_base,
            width: decoder.width(),
            height: decoder.height(),

            ictx,
            video_stream_idx,
            decoder,
            decoded,
            scaler,
            packet_sent: false,
            skip_until: None,
            last_decoded: None,
        })
    }

    pub fn duration(&self) -> Duration {
        self.dur
    }

    pub fn fps(&self) -> Ratio<i64> {
        self.fps
    }

    pub fn sec_per_frame(&self) -> Duration {
        Duration::new(self.fps.recip())
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rect(&self) -> Rect {
        Rect::at(0, 0).of_size(self.width, self.height)
    }

    pub fn seek(&mut self, ts: Timestamp) -> Result<(), ffmpeg::Error> {
        let seek_ts = (ts.as_ratio() / TIME_BASE.to_ratio()).floor().to_integer();
        trace!(%ts, %seek_ts);

        self.ictx.seek(seek_ts, ..seek_ts)?;
        self.decoder.flush();
        self.packet_sent = false;

        self.skip_until = Some(self.to_precise_frame_start(ts).timestamp());

        Ok(())
    }

    pub fn range_decoder(&mut self, range: TimestampRange) -> Result<RangeDecoder, ffmpeg::Error> {
        let start = match range {
            TimestampRange::Full => Timestamp::new(Ratio::ZERO),
            TimestampRange::Single(ts) => ts,
            TimestampRange::Range(start, _) => start,
            TimestampRange::RangeFrom(start) => start,
            TimestampRange::RangeTo(_) => Timestamp::new(self.dur.as_ratio()),
        };
        let start = self.to_precise_frame_start(start);

        let end = match range {
            TimestampRange::Full => Timestamp::new(self.dur.as_ratio()),
            TimestampRange::Single(ts) => ts + self.sec_per_frame(),
            TimestampRange::Range(_, end) => end,
            TimestampRange::RangeFrom(_) => Timestamp::new(self.dur.as_ratio()),
            TimestampRange::RangeTo(end) => end,
        };
        let end = self.to_precise_frame_end(end);

        self.seek(start.timestamp())?;

        let decoder = RangeDecoder {
            capture: self,
            start,
            end,
        };
        Ok(decoder)
    }

    fn read_video_packet(&mut self) -> Option<(Stream, Packet)> {
        let video_stream_index = self.video_stream_idx;
        self.ictx
            .packets()
            .find(|(stream, _packet)| stream.index() == video_stream_index)
    }

    fn decoded_frame_info(&self) -> FramePosition {
        let rough_ts =
            Ratio::from_integer(self.decoded.timestamp().unwrap()) * self.stream_time_base;
        self.to_precise_frame_pos(Timestamp::new(rough_ts))
    }

    fn frame_pos_of_index(&self, idx: usize) -> FramePosition {
        let ts = Timestamp::new(self.sec_per_frame().as_ratio() * idx as i64);
        FramePosition::new(idx, ts)
    }

    fn to_precise_frame_pos(&self, rough_ts: Timestamp) -> FramePosition {
        let frame_idx = (rough_ts.as_ratio() * self.fps).round().to_integer() as usize;
        self.frame_pos_of_index(frame_idx)
    }

    pub fn to_precise_frame_start(&self, rough_ts: Timestamp) -> FramePosition {
        let precise_pos = self.to_precise_frame_pos(rough_ts);
        if (rough_ts.as_ratio() - precise_pos.timestamp().as_ratio()).abs() < Ratio::new(1, 1000) {
            return precise_pos;
        }

        // If precise position is not close to enough, seek to the frame that contains the timestamp
        let frame_idx = (rough_ts.as_ratio() * self.fps).floor().to_integer() as usize;
        self.frame_pos_of_index(frame_idx)
    }

    pub fn to_precise_frame_end(&self, rough_ts: Timestamp) -> FramePosition {
        let precise_pos = self.to_precise_frame_pos(rough_ts);
        if (rough_ts.as_ratio() - precise_pos.timestamp().as_ratio()).abs() < Ratio::new(1, 1000) {
            return precise_pos;
        }

        // If precise position is not close to enough, seek to the frame that contains the timestamp
        let frame_idx = (rough_ts.as_ratio() * self.fps).ceil().to_integer() as usize;
        self.frame_pos_of_index(frame_idx)
    }

    fn write_frame_common(&mut self, rgb_frame: &mut Frame, pos: FramePosition) {
        self.last_decoded = None;
        rgb_frame.pos = pos;
        rgb_frame.dur = self.sec_per_frame();
    }

    fn write_normal_frame(
        &mut self,
        rgb_frame: &mut Frame,
        pos: FramePosition,
    ) -> Result<(), ffmpeg::Error> {
        self.write_frame_common(rgb_frame, pos);
        self.scaler.run(&self.decoded, &mut rgb_frame.data)?;

        Ok(())
    }

    fn write_eof_frame(&mut self, rgb_frame: &mut Frame, pos: Option<FramePosition>) {
        let pos =
            pos.unwrap_or_else(|| self.to_precise_frame_pos(Timestamp::new(self.dur.as_ratio())));
        self.write_frame_common(rgb_frame, pos);
    }

    fn decode_frame_inner(&mut self) -> Result<Option<FramePosition>, ffmpeg::Error> {
        loop {
            if !self.packet_sent {
                match self.read_video_packet() {
                    Some((_stream, packet)) => self.decoder.send_packet(&packet)?,
                    None => self.decoder.send_eof()?,
                }
                self.packet_sent = true;
            }

            match self.decoder.receive_frame(&mut self.decoded) {
                Ok(()) => {
                    let pos = self.decoded_frame_info();
                    self.last_decoded = Some(pos);

                    if let Some(until) = self.skip_until {
                        let end_ts = pos.timestamp() + self.sec_per_frame();
                        if end_ts <= until {
                            trace!(%pos, %end_ts, "skip frame");
                            continue;
                        }
                        self.skip_until = None;
                    }

                    return Ok(Some(pos));
                }
                Err(ffmpeg::Error::Eof) => {
                    trace!("EOF reached");
                    return Ok(None);
                }
                Err(ffmpeg::Error::Other {
                    errno: libc::EAGAIN,
                }) => {
                    self.packet_sent = false;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub fn decode_frame(&mut self, frame: &mut Frame) -> Result<bool, ffmpeg::Error> {
        match self.decode_frame_inner()? {
            Some(pos) => {
                self.write_normal_frame(frame, pos)?;
                Ok(true)
            }
            None => {
                self.write_eof_frame(frame, None);
                Ok(false)
            }
        }
    }
}

fn get_duration(ictx: &format::context::Input, stream_idx: usize) -> Option<Ratio<i64>> {
    // Borrow from OpenCV's implementation
    // https://github.com/opencv/opencv/blob/1ca526dcdb9c30600c70537e279f0c672057a1b9/modules/videoio/src/cap_ffmpeg_impl.hpp#L1892

    let duration = Ratio::from(ictx.duration()) * TIME_BASE.to_ratio();
    if duration > Ratio::ZERO {
        return Some(duration);
    }

    let stream = ictx.stream(stream_idx)?;
    let duration = Ratio::from(stream.duration()) * stream.time_base().to_ratio();
    if duration > Ratio::ZERO {
        return Some(duration);
    }

    None
}
fn get_fps(ictx: &mut format::context::Input, stream_idx: usize) -> Option<Ratio<i64>> {
    // Borrow from OpenCV's implementation
    // https://github.com/opencv/opencv/blob/1ca526dcdb9c30600c70537e279f0c672057a1b9/modules/videoio/src/cap_ffmpeg_impl.hpp#L1909

    let fps = ictx.stream(stream_idx)?.avg_frame_rate().to_ratio();
    if fps > Ratio::ZERO {
        return Some(fps);
    }

    let fps = unsafe {
        ffmpeg::ffi::av_guess_frame_rate(
            ictx.as_mut_ptr(),
            ictx.stream_mut(stream_idx)?.as_mut_ptr(),
            ptr::null_mut(),
        )
    }
    .to_ratio();
    if fps > Ratio::ZERO {
        return Some(fps);
    }

    let fps = ictx.stream(stream_idx)?.time_base().invert().to_ratio();
    if fps > Ratio::ZERO {
        return Some(fps);
    }

    None
}

fn get_frames(ictx: &mut format::context::Input, stream_idx: usize) -> Option<i64> {
    // Borrow from OpenCV's implementation
    // https://github.com/opencv/opencv/blob/1ca526dcdb9c30600c70537e279f0c672057a1b9/modules/videoio/src/cap_ffmpeg_impl.hpp#L1932

    let frames = ictx.stream(stream_idx)?.frames();
    if frames > 0 {
        return Some(frames);
    }

    let frames = (get_duration(ictx, stream_idx)? * get_fps(ictx, stream_idx)?)
        .round()
        .to_integer();
    if frames > 0 {
        return Some(frames);
    }

    None
}

#[derive(Debug)]
pub struct RangeDecoder<'a> {
    capture: &'a mut VideoCapture,
    start: FramePosition,
    end: FramePosition,
}

impl RangeDecoder<'_> {
    pub fn capture(&self) -> &VideoCapture {
        self.capture
    }

    pub fn start(&self) -> FramePosition {
        self.start
    }

    pub fn end(&self) -> FramePosition {
        self.end
    }

    pub fn decode_frame(&mut self, frame: &mut Frame) -> Result<bool, ffmpeg::Error> {
        match self.capture.decode_frame_inner()? {
            Some(pos) => {
                if pos.timestamp() >= self.end.timestamp() {
                    self.capture.write_eof_frame(frame, Some(pos));
                    return Ok(false);
                }
                self.capture.write_normal_frame(frame, pos)?;
                Ok(true)
            }
            None => {
                self.capture.write_eof_frame(frame, None);
                Ok(false)
            }
        }
    }
}
