use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, LazyLock, Mutex},
    thread::{self, JoinHandle},
};

use color_eyre::eyre::{self, OptionExt as _};
use elden_analyzer::{
    components::Components, image_process::tesseract::Tesseract, util::ImageLogger,
    video_capture::VideoCapture,
};
use elden_analyzer_kernel::types::time::TimestampRange;
use lockfree_object_pool::LinearObjectPool;
use rayon::{prelude::*, ThreadPoolBuilder};
use tracing::Span;

use crate::tui::ProgressBarBuilder;

mod comp_accum;
mod comp_detect;
mod decode;
mod text_accum;
mod text_recognize;

/// Analyze the video files to extract information
#[derive(clap::Parser, Debug)]
pub struct Args {
    /// Input file to process
    input: PathBuf,
    /// Frames to process
    #[clap(default_value = "-")]
    timestamp: TimestampRange,
    /// Output span file
    #[clap(long)]
    output_span: Option<PathBuf>,
    /// Output TSV file
    #[clap(long)]
    output_tsv: Option<PathBuf>,
}

impl Args {
    #[tracing::instrument(name = "analyze", skip_all)]
    pub(crate) fn run(&self) -> eyre::Result<()> {
        ImageLogger::init(false)?;

        process_file(
            &self.input,
            self.timestamp,
            self.output_span.as_deref(),
            self.output_tsv.as_deref(),
        )?;
        Ok(())
    }
}

#[tracing::instrument(name = "file", skip_all, fields(path = %file.file_name().unwrap_or_default().to_string_lossy()))]
fn process_file(
    file: &Path,
    timestamp: TimestampRange,
    output_span: Option<&Path>,
    output_tsv: Option<&Path>,
) -> eyre::Result<()> {
    let mut capture = VideoCapture::open(file)?;
    let mut decoder = capture.range_decoder(timestamp)?;
    let base_rect = decoder.capture().rect();

    let tess = LinearObjectPool::new(
        move || LazyLock::new(move || Mutex::new(Tesseract::new(None, Some("jpn")).unwrap())),
        |_v| {},
    );

    let output_span = output_span.map(File::create).transpose()?;
    let output_tsv = output_tsv.map(File::create).transpose()?;

    let components = Arc::new(Components::new(base_rect).ok_or_eyre("invalid frame size")?);

    let start = decoder.start();
    let end = decoder.end();
    let fps = decoder.capture().fps();
    let sec_per_frame = decoder.capture().sec_per_frame();

    let pbar_builder = ProgressBarBuilder::new(start, end, fps);
    let pbar = pbar_builder.build(Span::current());

    let (cap_tx, cap_rx) = mpsc::channel();
    let (comp_detect_tx, comp_detect_rx) = mpsc::channel();
    let (comp_accum_tx, comp_accum_rx) = mpsc::channel();
    let (text_recognize_tx, text_recognize_rx) = mpsc::channel();

    let comp_detect_thread = tracing::info_span!("comp_tedect").in_scope(|| {
        let components = Arc::clone(&components);
        spawn_streaming_thread(cap_rx, comp_detect_tx, "comp_detect", move |packet| {
            comp_detect::run(&components, packet)
        })
    });

    let comp_accum_thread = spawn_accumulate_thread("comp_accum", move || {
        comp_accum::run(comp_detect_rx, comp_accum_tx)
    })?;

    let text_recognize_thread = tracing::info_span!("text_recognize").in_scope(|| {
        let components = Arc::clone(&components);
        spawn_streaming_thread(
            comp_accum_rx,
            text_recognize_tx,
            "text_recognize",
            move |packet| text_recognize::run(&components, &tess, packet),
        )
    });

    let text_accum_thread = spawn_accumulate_thread("text_accum", move || {
        text_accum::run(
            text_recognize_rx,
            start,
            sec_per_frame,
            output_span,
            output_tsv,
        )
    })?;

    tracing::info!(%start, %end, %fps, "capture start");

    decode::run(&pbar, cap_tx, &mut decoder)?;

    comp_detect_thread.join().unwrap()?;
    comp_accum_thread.join().unwrap()?;
    text_recognize_thread.join().unwrap()?;
    text_accum_thread.join().unwrap()?;

    tracing::info!("completed");

    Ok(())
}

fn spawn_streaming_thread<Input, Output, F>(
    rx: mpsc::Receiver<(usize, Input)>,
    tx: mpsc::Sender<(usize, Output)>,
    name: &'static str,
    f: F,
) -> JoinHandle<eyre::Result<()>>
where
    Input: Send + Sync + 'static,
    Output: Send + Sync + 'static,
    F: Fn(Input) -> eyre::Result<Output> + Send + Sync + 'static,
{
    let root_span = Span::current();
    thread::spawn(move || -> eyre::Result<()> {
        let _span = root_span.clone().entered();
        ThreadPoolBuilder::default()
            .thread_name(move |n| format!("{name}#{n}"))
            .build()?
            .install(move || -> eyre::Result<()> {
                rx.into_iter().par_bridge().try_for_each(
                    move |(i, packet)| -> eyre::Result<_> {
                        let _span = root_span.enter();
                        let packet = f(packet)?;
                        tx.send((i, packet))?;
                        Ok(())
                    },
                )?;
                Ok(())
            })?;
        Ok(())
    })
}

fn spawn_accumulate_thread<F>(name: &str, f: F) -> eyre::Result<JoinHandle<eyre::Result<()>>>
where
    F: FnOnce() -> eyre::Result<()> + Send + 'static,
{
    let root_span = Span::current();
    let handler = thread::Builder::new()
        .name(name.into())
        .spawn(move || -> eyre::Result<()> {
            let _span = root_span.enter();
            f()
        })?;
    Ok(handler)
}
