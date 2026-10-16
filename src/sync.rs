use anyhow::{bail, Context, Result};
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::config::Config;

pub enum Reason {
    New,
    Changed,
}

pub enum Action {
    CreateDir(PathBuf),
    RemoveDir(PathBuf),
    Copy { from: PathBuf, to: PathBuf, reason: Reason },
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Action::CreateDir(p) => write!(f, "+ dir      {}", p.display()),
            Action::RemoveDir(p) => write!(f, "- remove   {}", p.display()),
            Action::Copy { to, reason: Reason::New, .. } => write!(f, "+ new      {}", to.display()),
            Action::Copy { to, reason: Reason::Changed, .. } => write!(f, "~ changed  {}", to.display()),
        }
    }
}

/// Строит список действий, ничего не меняя на диске.
pub fn plan(source: &Path, dest: &Path) -> Result<Vec<Action>> {
    if !source.is_dir() {
        bail!("source {} is not a folder", source.display());
    }
    if dest.starts_with(source) {
        bail!("backup {} is inside source {}", dest.display(), source.display());
    }

    let mut actions = Vec::new();

    if !dest.exists() {
        actions.push(Action::CreateDir(dest.to_path_buf()));
    }

    for entry in WalkDir::new(source).min_depth(1) {
        let entry = entry.with_context(|| format!("cannot read {}", source.display()))?;
        let to = dest.join(entry.path().strip_prefix(source)?);
        let file_type = entry.file_type();

        if file_type.is_dir() {
            if !to.exists() {
                actions.push(Action::CreateDir(to));
            }
        } else if file_type.is_file() {
            if let Some(reason) = check(entry.path(), &to)? {
                actions.push(Action::Copy { from: entry.path().to_path_buf(), to, reason });
            }
        }
        // симлинки пропускаются
    }

    Ok(actions)
}

fn check(from: &Path, to: &Path) -> Result<Option<Reason>> {
    let to_meta = match fs::metadata(to) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Some(Reason::New)),
        Err(e) => return Err(e).with_context(|| format!("cannot stat {}", to.display())),
    };
    let from_meta = fs::metadata(from)
        .with_context(|| format!("cannot stat {}", from.display()))?;

    if from_meta.len() != to_meta.len() || from_meta.modified()? > to_meta.modified()? {
        Ok(Some(Reason::Changed))
    } else {
        Ok(None)
    }
}

pub fn apply(actions: &[Action]) -> Result<()> {
    for action in actions {
        match action {
            Action::CreateDir(path) => {
                fs::create_dir_all(path)
                    .with_context(|| format!("cannot create {}", path.display()))?;
            }
            Action::RemoveDir(path) => {
                fs::remove_dir_all(path)
                    .with_context(|| format!("cannot remove {}", path.display()))?;
            }
            Action::Copy { from, to, .. } => {
                fs::copy(from, to).with_context(|| {
                    format!("cannot copy {} -> {}", from.display(), to.display())
                })?;
            }
        }
    }
    Ok(())
}

/// Проверяет, что верхний уровень backup_dir соответствует записям в конфиге.
/// Если есть папки, которых нет в конфиге — ошибка.
pub fn verify_backup_dir(backup_dir: &Path, conf: &Config) -> Result<()> {
    for entry in std::fs::read_dir(backup_dir)
        .with_context(|| format!("cannot read {}", backup_dir.display()))?
    {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !conf.backups.iter().any(|b| {
            b.backup_dir
                .file_name()
                .map(|n| n.to_string_lossy() == name)
                .unwrap_or(false)
        }) {
            bail!(
                "folder '{}' exists in {} but not found in nest_backup_jornal.toml. Add it to config or remove it.",
                name,
                backup_dir.display()
            );
        }
    }
    Ok(())
}

/// Безопасный deploy: копирует из backup_dir в source, не трогая существующие файлы.
pub fn plan_safe_deploy(backup_dir: &Path, source: &Path) -> Result<Vec<Action>> {
    if !backup_dir.is_dir() {
        bail!("backup dir {} does not exist", backup_dir.display());
    }

    let mut actions = Vec::new();

    if !source.exists() {
        actions.push(Action::CreateDir(source.to_path_buf()));
    }

    for entry in WalkDir::new(backup_dir).min_depth(1) {
        let entry = entry.with_context(|| format!("cannot read {}", backup_dir.display()))?;
        let to = source.join(entry.path().strip_prefix(backup_dir)?);
        let file_type = entry.file_type();

        if file_type.is_dir() {
            if !to.exists() {
                actions.push(Action::CreateDir(to));
            }
        } else if file_type.is_file() {
            if !to.exists() {
                actions.push(Action::Copy {
                    from: entry.path().to_path_buf(),
                    to,
                    reason: Reason::New,
                });
            }
        }
    }

    Ok(actions)
}

/// Force deploy: удаляет source полностью и копирует заново из backup_dir.
pub fn plan_force_deploy(backup_dir: &Path, source: &Path) -> Result<Vec<Action>> {
    if !backup_dir.is_dir() {
        bail!("backup dir {} does not exist", backup_dir.display());
    }

    let mut actions = Vec::new();

    if source.exists() {
        actions.push(Action::RemoveDir(source.to_path_buf()));
    }

    actions.push(Action::CreateDir(source.to_path_buf()));

    for entry in WalkDir::new(backup_dir).min_depth(1) {
        let entry = entry.with_context(|| format!("cannot read {}", backup_dir.display()))?;
        let to = source.join(entry.path().strip_prefix(backup_dir)?);
        let file_type = entry.file_type();

        if file_type.is_dir() {
            if !to.exists() {
                actions.push(Action::CreateDir(to));
            }
        } else if file_type.is_file() {
            actions.push(Action::Copy {
                from: entry.path().to_path_buf(),
                to,
                reason: Reason::New,
            });
        }
    }

    Ok(actions)
}
