use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
};

use chrono::Utc;
use clap::Parser as _;
use color_eyre::eyre::{self};
use tracing::level_filters::LevelFilter;
use tracing_error::ErrorLayer;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};

use crate::subcommand::Subcommand;

mod subcommand;
mod tui;

#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(flatten)]
    log_args: LogArgs,
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(clap::Parser, Debug)]
struct LogArgs {
    #[clap(long, value_parser = parse_filter_arg)]
    console_filter: Option<Arc<EnvFilter>>,
    #[clap(long, default_value = "false")]
    emit_log: bool,
    #[clap(long, default_value = "log")]
    log_dir: PathBuf,
    #[clap(long, value_parser = parse_filter_arg)]
    log_filter: Option<Arc<EnvFilter>>,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let Args {
        log_args,
        subcommand,
    } = Args::parse();

    init_log(log_args)?;
    ffmpeg::init()?;

    subcommand.run()?;

    Ok(())
}

fn init_log(args: LogArgs) -> eyre::Result<()> {
    let LogArgs {
        console_filter,
        emit_log,
        log_dir,
        log_filter,
    } = args;

    let indicatif_layer = IndicatifLayer::new();
    let console_filter = console_filter
        .map(|f| Arc::into_inner(f).unwrap())
        .unwrap_or_else(|| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .parse_lossy("")
        });
    let console_layer = fmt::layer()
        .with_timer(fmt::time::Uptime::default())
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(indicatif_layer.get_stderr_writer())
        .with_filter(console_filter);

    let log_filter = log_filter
        .map(|f| Arc::into_inner(f).unwrap())
        .unwrap_or_else(|| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy()
        });

    let error_layer = ErrorLayer::default();

    let log_layer = emit_log
        .then(|| -> eyre::Result<_> {
            let utc = Utc::now();
            let timestamp = utc.format("%Y-%m-%d_%H-%M-%S");
            let log_path = log_dir.join(format!("{timestamp}.log"));
            fs::create_dir_all(&log_dir)?;
            let log_file = File::create(log_path)?;

            let layer = fmt::layer()
                .with_ansi(false)
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(Arc::new(log_file))
                .with_filter(log_filter);
            Ok(layer)
        })
        .transpose()?;

    tracing_subscriber::registry()
        .with(console_layer)
        .with(log_layer)
        .with(indicatif_layer)
        .with(error_layer)
        .init();

    Ok(())
}

fn parse_filter_arg(s: &str) -> eyre::Result<Arc<EnvFilter>> {
    let filter = EnvFilter::try_new(s)?;
    Ok(Arc::new(filter))
}
