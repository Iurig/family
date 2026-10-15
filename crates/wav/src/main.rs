//! `wav`: the shared WAV commands on their own, without a plugin's `render`.

use clap::Parser;

/// Checks WAV files' levels and clicks, and splits and joins stereo files.
#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: family_wav::Command,
}

fn main() -> anyhow::Result<()> {
    Args::parse().command.run()
}
