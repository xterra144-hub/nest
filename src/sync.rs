use anyhow::{bail, Context, Result};
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub enum Reason {
    New,
    Changed,
}

pub enum Action {
    CreateDir(PathBuf),
    Copy { from: PathBuf, to: PathBuf, reason: Reason },
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Action::CreateDir(p) => write!(f, "+ dir      {}", p.display()),
            Action::Copy { to, reason: Reason::New, .. } => write!(f, "+ new      {}", to.display()),
            Action::Copy { to, reason: Reason::Changed, .. } => write!(f, "~ changed  {}", to.display()),
        }
    }
}

/// Строит список действий, ничего не меняя на диске.
pub fn plan(source: &Path, dest: &Path) -> Result<Vec<Action>> {
    if !source.exists() {
        bail!("source {} does not exist", source.display());
    }
    if dest.starts_with(source) {
        bail!("backup {} is inside source {}", dest.display(), source.display());
    }

    let mut actions = Vec::new();

    if !dest.exists() {
        actions.push(Action::CreateDir(dest.to_path_buf()));
    }

    if source.is_file() {
        let to = if dest.is_file() {
            dest.to_path_buf()
        } else {
            let name = source.file_name().context("source has no file name")?;
            dest.join(name)
        };
        if let Some(reason) = check(source, &to)? {
            actions.push(Action::Copy { from: source.to_path_buf(), to, reason });
        }
        return Ok(actions);
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
            Action::Copy { from, to, .. } => {
                fs::copy(from, to).with_context(|| {
                    format!("cannot copy {} -> {}", from.display(), to.display())
                })?;
            }
        }
    }
    Ok(())
}
