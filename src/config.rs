use serde::{Deserialize, Serialize};
use anyhow::Context;

const CONF_FILE: &str = "nest_backup_jornal.toml";

#[derive(Deserialize, Serialize)]
pub struct Backup{
    pub name: String,
    pub source: std::path::PathBuf,
    pub backup_dir:std::path::PathBuf,
    pub last_backup: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config{
    #[serde(default)]
    pub backups: Vec<Backup>,
    #[serde(skip)]
    pub conf_file: std::path::PathBuf
}



impl Config {
    pub fn init(dir: &std::path::Path) -> anyhow::Result<()> {
        if !dir.is_dir() {
            anyhow::bail!("{} not exist or not folder", dir.display())
        }
        let path = dir.join("nest_backup_jornal.toml");
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                anyhow::bail!("nest already initialized in {}", dir.display())
            }
            Err(e) => Err(e).with_context(|| format!("cannot to create {}",path.display()))
        }
    }
    pub fn load(dir: &std::path::Path) -> anyhow::Result<Self> {
        let path = dir.join(CONF_FILE);
        let text = std::fs::read_to_string(&path).with_context(|| format!("cannot read {} did you run 'nest init'? ", path.display()))?;
        let mut conf: Config = toml::from_str(&text).with_context(|| format!("invalid config {}", path.display()))?;
        conf.conf_file = path;
        Ok(conf)
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let text = toml::to_string_pretty(self).context("cannot serialize config")?;
        std::fs::write(&self.conf_file, text).with_context(|| format!("cannot write {}", self.conf_file.display()))
    }
    pub fn add(&mut self, name: &str, source: &std::path::Path, backup_dir: &std::path::Path) -> anyhow::Result<()> {
        anyhow::ensure!(!self.backups.iter().any(|b| b.name == name), "backup {} already exists",name);
        anyhow::ensure!(source.is_dir(), "source {} is not a folder", source.display());
        anyhow::ensure!(backup_dir.is_dir(), "backup {} is not a folder", backup_dir.display());
        let source = std::path::absolute(source).with_context(|| format!("source {} is not found", source.display()))?;
        let backup_dir = std::path::absolute(backup_dir).with_context(|| format!("backup dir {} is not found", backup_dir.display()))?;
        self.backups.push(Backup {
            name: name.to_string(),
            source,
            backup_dir,
            last_backup: chrono::Local::now().to_rfc3339(),
        });
        self.save()
    }
    pub fn remove(&mut self, name: &str) -> anyhow::Result<()> {
        let before = self.backups.len();
        self.backups.retain(|b| b.name != name);
        anyhow::ensure!(self.backups.len() < before, "backups not found");
        self.save()
    }


}
