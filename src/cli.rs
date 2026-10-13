use clap::{Args,  Parser, Subcommand};
use crate::{
    config::Config, 
    sync::{self}
};
use anyhow::Context;
use colored::Colorize;
use comfy_table::{Table, ContentArrangement};
#[derive(Parser)]
#[command(name = "nest", version, about = "backups manager")]
pub struct Cli {
    #[arg(long, global = true, default_value = ".")]
    dir: std::path::PathBuf,
    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand)]
enum Command {
    /// nest init in folder
    Init(InitArgs),
    /// add a link between source and backup
    Add(AddArgs),
    /// remove a link between source and backup
    Remove(RemoveArgs),
    /// sync source and backup
    Sync,
    /// show diff between now and before
    Diff,
    /// show status of all backups
    Status,
}

#[derive(Args)]
struct InitArgs {
    /// path to folder
    folder: std::path::PathBuf
}

#[derive(Args)]
struct AddArgs {
    ///path to backup file
    backup: std::path::PathBuf,
    ///path to backup file
    source: std::path::PathBuf,
    ///link name
    #[arg(long)]
    name: String
}
#[derive(Args)]
struct RemoveArgs {
    ///link name
    name: String
}



pub fn run(cli: Cli) -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init(a) => Config::init(&a.folder),
        Command::Add(a) => Config::load(&cli.dir)?.add(&a.name, &a.source, &a.backup),
        Command::Remove(a) => Config::load(&cli.dir)?.remove(&a.name),
        Command::Diff => run_diff(&cli.dir),
        Command::Sync => run_sync(&cli.dir),
        Command::Status => run_status(&cli.dir),
    }
}

fn run_diff(dir: &std::path::Path) -> anyhow::Result<()> {
    let conf = Config::load(dir)?;

    for b in &conf.backups {
        let actions = sync::plan(&b.source, &b.backup_file).with_context(|| format!("backup '{}'", b.name))?;
        if actions.is_empty() {
            println!("'{}': up to date", b.name);
        } else {
            println!("'{}': {} changes", b.name, actions.len());
            for a in &actions {
                println!("  {a}");
            }
        }
    }
    Ok(())
}

fn run_sync(dir: &std::path::Path) -> anyhow::Result<()> {
    let mut conf = Config::load(dir)?;

    for i in 0..conf.backups.len() {
        let actions = sync::plan(&conf.backups[i].source, &conf.backups[i].backup_file)
            .with_context(|| format!("backup '{}'", conf.backups[i].name))?;

        if actions.is_empty() {
            println!("'{}': up to date", conf.backups[i].name);
            continue;
        }

        sync::apply(&actions).with_context(|| format!("backup '{}'", conf.backups[i].name))?;
        conf.backups[i].last_backup = chrono::Local::now().to_rfc3339();
        println!("'{}': {} changes applied", conf.backups[i].name, actions.len());
    }
    conf.save()?;
    Ok(())
}

fn run_status(dir: &std::path::Path) -> anyhow::Result<()> {
    let conf = Config::load(dir)?;

    if conf.backups.is_empty() {
        println!("{}", "No backups configured".yellow());
        return Ok(());
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        "Name".bold().to_string(),
        "Source".bold().to_string(),
        "Backup".bold().to_string(),
        "Last backup".bold().to_string(),
        "Need sync".bold().to_string(),
    ]);

    for b in &conf.backups {
        let need_sync = match sync::plan(&b.source, &b.backup_file) {
            Ok(actions) => !actions.is_empty(),
            Err(_) => true,
        };

        let need_sync_str = if need_sync {
            "✗ yes".red().to_string()
        } else {
            "✓ no".green().to_string()
        };

        table.add_row(vec![
            b.name.clone(),
            b.source.display().to_string(),
            b.backup_file.display().to_string(),
            b.last_backup.clone(),
            need_sync_str,
        ]);
    }

    println!("{table}");
    Ok(())
}
