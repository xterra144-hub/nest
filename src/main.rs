mod cli;
mod sync;
mod config;
mod global;
fn main() -> anyhow::Result<()>{
    cli::run()    
}

