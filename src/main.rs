mod cli;
mod sync;
mod config;
use clap::Parser;
use crate::cli::Cli;
fn main() -> anyhow::Result<()>{
    cli::run(Cli::parse())
}
