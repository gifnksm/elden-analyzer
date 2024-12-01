use std::{io::Write, path::Path};

use ffmpeg::{format, media};

use super::Result;

pub fn dump(mut out: impl Write, file: &impl AsRef<Path>) -> Result<()> {
    let context = format::input(file)?;

    writeln!(&mut out, "Metadata")?;
    for (k, v) in context.metadata().iter() {
        writeln!(&mut out, "\t{k}: {v}")?;
    }

    if let Some(stream) = context.streams().best(media::Type::Video) {
        writeln!(&mut out, "\tBest video stream index: {}", stream.index())?;
    }

    if let Some(stream) = context.streams().best(media::Type::Audio) {
        writeln!(&mut out, "\tBest audio stream index: {}", stream.index())?;
    }

    if let Some(stream) = context.streams().best(media::Type::Subtitle) {
        writeln!(&mut out, "\tBest subtitle stream index: {}", stream.index())?;
    }

    writeln!(&mut out, "\tduration (timebase): {}", context.duration())?;
    writeln!(
        &mut out,
        "\tduration (seconds): {:.6}",
        context.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE)
    )?;

    for stream in context.streams() {
        writeln!(&mut out, "stream #{}:", stream.index())?;
        writeln!(&mut out, "\ttime_base: {}", stream.time_base())?;
        writeln!(&mut out, "\tstart_time: {}", stream.start_time())?;
        writeln!(
            &mut out,
            "\tduration (stream timebase): {}",
            stream.duration()
        )?;
        writeln!(
            &mut out,
            "\tduration (seconds): {:.6}",
            stream.duration() as f64 * f64::from(stream.time_base())
        )?;
        writeln!(&mut out, "\tframes: {}", stream.frames())?;
        writeln!(&mut out, "\tdisposition: {:?}", stream.disposition())?;
        writeln!(&mut out, "\tdiscard: {:?}", stream.discard())?;
        writeln!(&mut out, "\trate: {}", stream.rate())?;

        let codec = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
        writeln!(&mut out, "\tmedium: {:?}", codec.medium())?;
        writeln!(&mut out, "\tid: {:?}", codec.id())?;

        match codec.medium() {
            media::Type::Video => {
                if let Ok(video) = codec.decoder().video() {
                    writeln!(&mut out, "\tbit_rate: {}", video.bit_rate())?;
                    writeln!(&mut out, "\tmax_rate: {}", video.max_bit_rate())?;
                    writeln!(&mut out, "\tdelay: {}", video.delay())?;
                    writeln!(&mut out, "\tvideo.width: {}", video.width())?;
                    writeln!(&mut out, "\tvideo.height: {}", video.height())?;
                    writeln!(&mut out, "\tvideo.format: {:?}", video.format())?;
                    writeln!(&mut out, "\tvideo.has_b_frames: {}", video.has_b_frames())?;
                    writeln!(&mut out, "\tvideo.aspect_ratio: {}", video.aspect_ratio())?;
                    writeln!(&mut out, "\tvideo.color_space: {:?}", video.color_space())?;
                    writeln!(&mut out, "\tvideo.color_range: {:?}", video.color_range())?;
                    writeln!(
                        &mut out,
                        "\tvideo.color_primaries: {:?}",
                        video.color_primaries()
                    )?;
                    writeln!(
                        &mut out,
                        "\tvideo.color_transfer_characteristic: {:?}",
                        video.color_transfer_characteristic()
                    )?;
                    writeln!(
                        &mut out,
                        "\tvideo.chroma_location: {:?}",
                        video.chroma_location()
                    )?;
                    writeln!(&mut out, "\tvideo.references: {}", video.references())?;
                    writeln!(
                        &mut out,
                        "\tvideo.intra_dc_precision: {}",
                        video.intra_dc_precision()
                    )?;
                }
            }
            media::Type::Audio => {
                if let Ok(audio) = codec.decoder().audio() {
                    writeln!(&mut out, "\tbit_rate: {}", audio.bit_rate())?;
                    writeln!(&mut out, "\tmax_rate: {}", audio.max_bit_rate())?;
                    writeln!(&mut out, "\tdelay: {}", audio.delay())?;
                    writeln!(&mut out, "\taudio.rate: {}", audio.rate())?;
                    writeln!(&mut out, "\taudio.channels: {}", audio.channels())?;
                    writeln!(&mut out, "\taudio.format: {:?}", audio.format())?;
                    writeln!(&mut out, "\taudio.frames: {}", audio.frames())?;
                    writeln!(&mut out, "\taudio.align: {}", audio.align())?;
                    writeln!(
                        &mut out,
                        "\taudio.channel_layout: {:?}",
                        audio.channel_layout()
                    )?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}
