use std::{
    env::current_dir,
    path::{Path, PathBuf},
};

use anyhow::anyhow;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Config {
    pub repo_dir: Option<String>,
    pub default_open_with: Option<String>,
}

impl Config {
    pub fn is_valid_custom(&self) -> bool {
        if self.repo_dir.is_none() {
            return false;
        }
        true
    }
}

fn local_config_dir() -> anyhow::Result<PathBuf> {
    let current_dir = current_dir()?;
    let rerman_dir: PathBuf = [current_dir.as_ref() as &Path, ".rerman".as_ref()]
        .iter()
        .collect();
    if !rerman_dir.is_dir() {
        Err(anyhow!("local config dir not found"))?
    } else {
        Ok(rerman_dir)
    }
}

fn user_config_dir() -> anyhow::Result<PathBuf> {
    if cfg!(target_os = "linux") {
        let config_home_var = std::env::var("XDG_CONFIG_HOME")
            .or_else(|_| std::env::var("HOME").map(|var| format!("{}/.config", var)))?;
        let rerman_config_dir = format!("{}/rerman", config_home_var);
        let rerman_config_dir = PathBuf::from(&rerman_config_dir);
        if !rerman_config_dir.is_dir() {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} not found", rerman_config_dir.to_string_lossy()),
            ))?
        }
        Ok(rerman_config_dir)
    } else {
        Err(anyhow::Error::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "cannot found user config dir",
        )))
    }
}
fn sys_config_dir() -> anyhow::Result<PathBuf> {
    if cfg!(target_os = "linux") {
        let rerman_config_dir = PathBuf::from("/etc/rerman");
        if !rerman_config_dir.is_dir() {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} not found", rerman_config_dir.to_string_lossy()),
            ))?
        }
        Ok(rerman_config_dir)
    } else {
        Err(anyhow!("cannot found user config dir"))
    }
}

pub enum ConfigSetup {
    Local,
    User,
    System,
    Custom {
        config_dir: PathBuf,
        repo_dir: PathBuf,
    },
}

impl ConfigSetup {
    pub fn config_dir(&self) -> anyhow::Result<PathBuf> {
        match self {
            Self::Local => Ok(current_dir()?.join(".rerman").join("config")),
            Self::User => user_config_dir(),
            Self::System => sys_config_dir(),
            Self::Custom { config_dir, .. } => Ok(config_dir.to_owned()),
        }
    }

    pub fn initial_content(&self) -> String {
        match self {
            Self::Local { .. } => "".to_string(),
            Self::User => "".to_string(),
            Self::System => "".to_string(),
            Self::Custom { repo_dir, .. } => {
                let mut doc = toml_edit::DocumentMut::new();
                doc["repo_dir"] = toml_edit::value(repo_dir.to_string_lossy().to_string());
                doc.to_string()
            }
        }
    }
}

impl Config {}
