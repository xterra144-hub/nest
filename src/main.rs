mod cli;
mod sync;
mod config;
fn main() -> anyhow::Result<()>{
    cli::run()
}
