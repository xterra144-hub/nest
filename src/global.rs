use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub projects: Vec<std::path::PathBuf>
}

impl GlobalConfig {
    fn file() -> std::path::PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join("nest").join("config.toml")
    }
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::file();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&text)?)
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::file();
        if let  Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
    /// регистрирует папку по абсолютному пути если её ещё нет 
    pub fn register(dir: &std::path::Path) -> anyhow::Result<(bool)> {
        let abs = std::fs::canonicalize(&dir)?;
        let mut conf = Self::load()?;
        if conf.projects.contains(&abs) {
            return Ok(false)
        }
        conf.projects.push(abs);
        conf.save()?;
        Ok(true) // true добавлено, false уже было 
    }
}
