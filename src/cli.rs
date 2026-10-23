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
    /// deploy backups from backup_dir to source
    Deploy(DeployArgs),
    /// show where is backup folders 
    Where,
}

#[derive(Args)]
struct InitArgs {
    /// path to folder
    folder: std::path::PathBuf
}

#[derive(Args)]
struct AddArgs {
    ///path to source folder
    source: std::path::PathBuf,
    ///path to backup folder
    backup_dir: std::path::PathBuf,
    ///link name
    #[arg(long)]
    name: String
}
#[derive(Args)]
struct RemoveArgs {
    ///link name
    name: String
}

#[derive(Args)]
struct DeployArgs {
    /// force overwrite source folder
    #[arg(long)]
    force: bool,
}



pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init(a) => Config::init(&a.folder),
        Command::Add(a) => Config::load(&cli.dir)?.add(&a.name, &a.source, &a.backup_dir),
        Command::Remove(a) => Config::load(&cli.dir)?.remove(&a.name),
        Command::Diff => run_diff(&cli.dir),
        Command::Sync => run_sync(&cli.dir),
        Command::Status => run_status(&cli.dir),
        Command::Deploy(a) => run_deploy(&cli.dir, a.force),
        Command::Where => run_where(),
    }
}

fn run_diff(dir: &std::path::Path) -> anyhow::Result<()> {
    let conf = Config::load(dir)?;

    for b in &conf.backups {
        let actions = sync::plan(&b.source, &b.backup_dir).with_context(|| format!("backup '{}'", b.name))?;
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
        let actions = sync::plan(&conf.backups[i].source, &conf.backups[i].backup_dir)
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
        "Backup dir".bold().to_string(),
        "Last backup".bold().to_string(),
        "Need sync".bold().to_string(),
    ]);

    for b in &conf.backups {
        let need_sync = match sync::plan(&b.source, &b.backup_dir) {
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
            b.backup_dir.display().to_string(),
            b.last_backup.clone(),
            need_sync_str,
        ]);
    }

    println!("{table}");
    Ok(())
}

fn run_deploy(dir: &std::path::Path, force: bool) -> anyhow::Result<()> {
    let conf = Config::load(dir)?;

    if conf.backups.is_empty() {
        println!("{}", "No backups configured".yellow());
        return Ok(());
    }

    // Проверка целостности: верхний уровень backup_dir должен соответствовать toml
    for b in &conf.backups {
        if !b.backup_dir.is_dir() {
            anyhow::bail!("backup dir '{}' does not exist", b.backup_dir.display());
        }
        sync::verify_backup_dir(&b.backup_dir, &conf)?;
    }

    for i in 0..conf.backups.len() {
        let actions = if force {
            sync::plan_force_deploy(&conf.backups[i].backup_dir, &conf.backups[i].source)
                .with_context(|| format!("backup '{}'", conf.backups[i].name))?
        } else {
            sync::plan_safe_deploy(&conf.backups[i].backup_dir, &conf.backups[i].source)
                .with_context(|| format!("backup '{}'", conf.backups[i].name))?
        };

        if actions.is_empty() {
            println!("'{}': up to date", conf.backups[i].name);
            continue;
        }

        sync::apply(&actions).with_context(|| format!("backup '{}'", conf.backups[i].name))?;
        println!("'{}': {} changes applied", conf.backups[i].name, actions.len());
    }
    Ok(())
}
fn run_where() -> anyhow::Result<()> {
    let conf = crate::global::GlobalConfig::load()?;
    for p in &conf.projects {
        println!("{}", p.display());
    }
    Ok(())
}
