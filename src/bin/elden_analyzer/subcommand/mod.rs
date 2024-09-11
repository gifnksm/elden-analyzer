use color_eyre::eyre;

mod analyze;
mod find_ui;
mod metadata;
mod recognize_text;

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    Analyze(analyze::Args),
    FindUi(find_ui::Args),
    RecognizeText(recognize_text::Args),
    Metadata(metadata::Args),
}

impl Subcommand {
    pub fn run(&self) -> eyre::Result<()> {
        match self {
            Subcommand::Analyze(args) => args.run()?,
            Subcommand::FindUi(args) => args.run()?,
            Subcommand::RecognizeText(args) => args.run()?,
            Subcommand::Metadata(args) => args.run()?,
        }

        Ok(())
    }
}
