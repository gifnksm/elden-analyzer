use std::path::PathBuf;

use color_eyre::eyre;
use ffmpeg::{format, media};

/// Show metadata of the video file
#[derive(clap::Parser, Debug)]
pub struct Args {
    /// The input file to process
    file: PathBuf,
}

impl Args {
    #[tracing::instrument(name = "metadata", skip_all)]
    pub(crate) fn run(&self) -> eyre::Result<()> {
        let context = match format::input(&self.file) {
            Ok(context) => context,
            Err(error) => {
                println!("error: {}", error);
                return Ok(());
            }
        };
        for (k, v) in context.metadata().iter() {
            println!("{}: {}", k, v);
        }

        if let Some(stream) = context.streams().best(media::Type::Video) {
            println!("Best video stream index: {}", stream.index());
        }

        if let Some(stream) = context.streams().best(media::Type::Audio) {
            println!("Best audio stream index: {}", stream.index());
        }

        if let Some(stream) = context.streams().best(media::Type::Subtitle) {
            println!("Best subtitle stream index: {}", stream.index());
        }

        println!("duration (timebase): {}", context.duration());
        println!(
            "duration (seconds): {:.6}",
            context.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE)
        );

        for stream in context.streams() {
            println!("stream index {}:", stream.index());
            println!("\ttime_base: {}", stream.time_base());
            println!("\tstart_time: {}", stream.start_time());
            println!("\tduration (stream timebase): {}", stream.duration());
            println!(
                "\tduration (seconds): {:.6}",
                stream.duration() as f64 * f64::from(stream.time_base())
            );
            println!("\tframes: {}", stream.frames());
            println!("\tdisposition: {:?}", stream.disposition());
            println!("\tdiscard: {:?}", stream.discard());
            println!("\trate: {}", stream.rate());

            let codec = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
            println!("\tmedium: {:?}", codec.medium());
            println!("\tid: {:?}", codec.id());

            match codec.medium() {
                media::Type::Video => {
                    if let Ok(video) = codec.decoder().video() {
                        println!("\tbit_rate: {}", video.bit_rate());
                        println!("\tmax_rate: {}", video.max_bit_rate());
                        println!("\tdelay: {}", video.delay());
                        println!("\tvideo.width: {}", video.width());
                        println!("\tvideo.height: {}", video.height());
                        println!("\tvideo.format: {:?}", video.format());
                        println!("\tvideo.has_b_frames: {}", video.has_b_frames());
                        println!("\tvideo.aspect_ratio: {}", video.aspect_ratio());
                        println!("\tvideo.color_space: {:?}", video.color_space());
                        println!("\tvideo.color_range: {:?}", video.color_range());
                        println!("\tvideo.color_primaries: {:?}", video.color_primaries());
                        println!(
                            "\tvideo.color_transfer_characteristic: {:?}",
                            video.color_transfer_characteristic()
                        );
                        println!("\tvideo.chroma_location: {:?}", video.chroma_location());
                        println!("\tvideo.references: {}", video.references());
                        println!("\tvideo.intra_dc_precision: {}", video.intra_dc_precision());
                    }
                }
                media::Type::Audio => {
                    if let Ok(audio) = codec.decoder().audio() {
                        println!("\tbit_rate: {}", audio.bit_rate());
                        println!("\tmax_rate: {}", audio.max_bit_rate());
                        println!("\tdelay: {}", audio.delay());
                        println!("\taudio.rate: {}", audio.rate());
                        println!("\taudio.channels: {}", audio.channels());
                        println!("\taudio.format: {:?}", audio.format());
                        println!("\taudio.frames: {}", audio.frames());
                        println!("\taudio.align: {}", audio.align());
                        println!("\taudio.channel_layout: {:?}", audio.channel_layout());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
