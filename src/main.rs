use anyhow::Result;
use clap::Parser;
use deviruchi::cli::Cli;
use deviruchi::core::Core;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut core = Core::new(cli);
    core.run().await
}
