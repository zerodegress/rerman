use std::{env::current_dir, path::PathBuf, process::Stdio};

use anyhow::anyhow;
use clap::Parser;
use tabled::Tabled;
use unic_langid::{langid, LanguageIdentifier};

use crate::{
    cli::{Cli, Commands, DebugCommands},
    config::Config,
    git::{filter_git_paths_recursively, Git, GitUrl},
    i18n::I18N,
};

#[derive(Debug, Clone)]
pub enum RerSetup {
    System,
    User,
    Local,
    Custom { config_file: PathBuf },
}

pub struct Rer {
    cli: Cli,
    setup: RerSetup,
    config: Config,
    i18n: I18N,
    lang_id: LanguageIdentifier,
}

#[derive(Tabled, serde::Serialize)]
pub struct RepoTableItem {
    path: String,
    #[tabled(rename = "type")]
    #[serde(rename = "type")]
    ty: String,
    hostname: String,
}

impl Rer {
    fn default_open_with(&self) -> Option<String> {
        self.config.open_with.to_owned()
    }

    fn repo_dir(&self) -> anyhow::Result<PathBuf> {
        if let Some(ref repo_dir) = self.config.repo_dir {
            Ok(PathBuf::from(repo_dir))
        } else {
            match self.setup {
                RerSetup::Local => Ok(current_dir()?.join(".rerman").join("repositories")),
                RerSetup::User => {
                    let base_dirs = directories::BaseDirs::new().ok_or_else(|| {
                        anyhow!(
                            "{}",
                            self.i18n
                                .format_msg_or_log(&self.lang_id, "error-get-base-dirs", None)
                        )
                    })?;
                    Ok(base_dirs
                        .data_local_dir()
                        .join("rerman")
                        .join("repositories"))
                }
                RerSetup::System => {
                    if cfg!(target_os = "linux") {
                        Ok(PathBuf::from("/usr/share/rerman/repositories"))
                    } else {
                        panic!(
                            "{}",
                            self.i18n.format_msg_or_log(
                                &self.lang_id,
                                "error-not-supported-operation-for-os",
                                None
                            )
                        )
                    }
                }
                RerSetup::Custom { .. } => Ok(PathBuf::from(
                    self.config.repo_dir.as_ref().ok_or_else(|| {
                        anyhow!(
                            "{}",
                            self.i18n.format_msg_or_log(
                                &self.lang_id,
                                "error-no-repo-dir-specified",
                                None
                            )
                        )
                    })?,
                )),
            }
        }
    }

    fn path_of_repo(
        &self,
        ty: impl AsRef<str>,
        hostname: impl AsRef<str>,
        username: impl AsRef<str>,
        path: impl AsRef<str>,
    ) -> anyhow::Result<PathBuf> {
        Ok((self.repo_dir()?)
            .join(ty.as_ref())
            .join(hostname.as_ref())
            .join(username.as_ref())
            .join(path.as_ref()))
    }

    fn config_file(&self) -> anyhow::Result<PathBuf> {
        Ok(match self.setup {
            RerSetup::System => {
                if cfg!(target_os = "linux") {
                    PathBuf::from("/etc/rerman/config.toml")
                } else {
                    panic!(
                        "{}",
                        self.i18n.format_msg_or_log(
                            &self.lang_id,
                            "error-not-supported-system-setup-for-os",
                            None
                        )
                    )
                }
            }
            RerSetup::User => {
                let base_dirs = directories::BaseDirs::new().ok_or_else(|| {
                    anyhow!(
                        "{}",
                        self.i18n
                            .format_msg_or_log(&self.lang_id, "error-get-base-dirs", None)
                    )
                })?;
                base_dirs.config_dir().join("rerman").join("config.toml")
            }
            RerSetup::Local => (current_dir()?).join(".rerman").join("config.toml"),
            RerSetup::Custom { ref config_file } => config_file.to_owned(),
        })
    }

    pub async fn parse() -> anyhow::Result<Self> {
        let lang_id = sys_locale::get_locale()
            .unwrap_or("en-US".to_string())
            .parse()
            .unwrap_or(langid!("en-US"));
        let i18n = I18N::new();
        let cli = Cli::parse();
        let setup = if let Some(true) = cli.system {
            RerSetup::System
        } else if let Some(true) = cli.user {
            RerSetup::User
        } else if let Some(true) = cli.local {
            RerSetup::Local
        } else if let Some(ref config) = cli.config {
            RerSetup::Custom {
                config_file: PathBuf::from(config),
            }
        } else if std::env::var("RERMAN_LEVEL").is_ok() {
            match std::env::var("RERMAN_LEVEL").unwrap_or_default().as_str() {
                "system" => RerSetup::System,
                "user" => RerSetup::User,
                "local" => RerSetup::Local,
                _ => RerSetup::User,
            }
        } else {
            RerSetup::User
        };
        let config_file = match setup {
            RerSetup::System => {
                if cfg!(target_os = "linux") {
                    PathBuf::from("/etc/rerman/config.toml")
                } else {
                    panic!(
                        "{}",
                        i18n.format_msg_or_log(
                            &lang_id,
                            "error-not-supported-system-setup-for-os",
                            None
                        )
                    )
                }
            }
            RerSetup::User => {
                let base_dirs = directories::BaseDirs::new().ok_or_else(|| {
                    anyhow!(
                        "{}",
                        i18n.format_msg_or_log(&lang_id, "error-get-base-dirs", None)
                    )
                })?;
                base_dirs.config_dir().join("rerman").join("config.toml")
            }
            RerSetup::Local => (current_dir()?).join(".rerman").join("config.toml"),
            RerSetup::Custom { ref config_file } => config_file.to_owned(),
        };
        let config = tokio::fs::read(config_file).await.or_else(|_| {
            println!(
                "{}",
                i18n.format_msg_or_log(&lang_id, "error-read-config-file", None)
            );
            toml::to_string(&Config::default()).map(|v| v.as_bytes().to_vec())
        })?;
        let config: Config = toml::from_str(&String::from_utf8(config)?).unwrap_or_else(|_| {
            println!(
                "{}",
                i18n.format_msg_or_log(&lang_id, "error-read-config-file", None)
            );
            Config::default()
        });
        Ok(Rer {
            cli,
            setup,
            config,
            i18n,
            lang_id,
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        match &self.cli.commands {
            Commands::Clone { r#type: ty, target } => {
                match ty.as_str() {
                    "git" => {
                        let url = GitUrl::parse(target)?;
                        let git = Git::default();
                        git.clone(
                            target,
                            self.repo_dir()?
                                .join("git")
                                .join(url.host())
                                .join(url.username())
                                .join({
                                    let path = url.path();
                                    let path = path.strip_prefix('/').unwrap_or(path);
                                    let path = path.strip_suffix(".git").unwrap_or(path);
                                    path
                                })
                                .to_string_lossy(),
                        )
                        .await?;
                    }
                    _ => {
                        todo!("more repository type")
                    }
                }
                Ok(())
            }
            Commands::Setup => {
                let config_file = self.config_file()?;
                let config_dir = config_file.parent().ok_or_else(|| {
                    anyhow!(
                        "{}",
                        self.i18n.format_msg_or_log(
                            &self.lang_id,
                            "error-invalid-config-dir",
                            Some(vec![(
                                "dir".to_string(),
                                config_file.join("..").to_string_lossy().to_string()
                            )])
                        )
                    )
                })?;
                if config_dir.exists() && !config_dir.is_dir() {
                    Err(anyhow!(
                        "{}",
                        self.i18n.format_msg_or_log(
                            &self.lang_id,
                            "error-invalid-config-dir",
                            Some(vec![(
                                "dir".to_string(),
                                config_dir.to_string_lossy().to_string()
                            )])
                        )
                    ))?
                }
                if !config_dir.exists() {
                    tokio::fs::create_dir_all(config_dir).await?;
                }

                let repo_dir = self.repo_dir()?;
                if repo_dir.exists() && !repo_dir.is_dir() {
                    Err(anyhow!(
                        "{}",
                        self.i18n.format_msg_or_log(
                            &self.lang_id,
                            "error-invalid-repo-dir",
                            Some(vec![(
                                "dir".to_string(),
                                repo_dir.to_string_lossy().to_string()
                            )])
                        )
                    ))?
                }
                if !repo_dir.exists() {
                    tokio::fs::create_dir_all(repo_dir).await?;
                }

                tokio::fs::write(
                    &config_file,
                    include_str!("../assets/config.toml").as_bytes(),
                )
                .await?;
                println!(
                    "{}",
                    self.i18n.format_msg_or_log(
                        &self.lang_id,
                        "info-setup-completed",
                        Some(vec![(
                            "file".to_string(),
                            config_file.to_string_lossy().to_string()
                        )])
                    )
                );
                Ok(())
            }
            Commands::Open { with, target } => {
                let open_with = with.to_owned().ok_or(()).or_else(|_| {
                    self.default_open_with().ok_or(anyhow!(
                        "{}",
                        self.i18n.format_msg_or_log(
                            &self.lang_id,
                            "error-no-default-open-with",
                            None
                        )
                    ))
                })?;
                for type_dir in std::fs::read_dir(self.repo_dir()?)? {
                    for host_dir in std::fs::read_dir(type_dir?.path())? {
                        let target_dir = host_dir?.path().join(target);
                        if target_dir.exists() && target_dir.join(".git").exists() {
                            tokio::process::Command::new(open_with)
                                .arg(target_dir.to_string_lossy().to_string())
                                .stdout(Stdio::inherit())
                                .spawn()?
                                .wait()
                                .await?;
                            return Ok(());
                        }
                    }
                }
                Err(anyhow!(
                    "{}",
                    self.i18n
                        .format_msg_or_log(&self.lang_id, "error-target-not-found", None)
                ))
            }
            Commands::Config { edit, with } => {
                if *edit {
                    let with_editor = with
                        .to_owned()
                        .or_else(|| self.config.config_editor.to_owned())
                        .or_else(|| {
                            if cfg!(target_os = "linux") {
                                std::env::var("EDITOR").ok()
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| {
                            anyhow!(
                                "{}",
                                self.i18n.format_msg_or_log(
                                    &self.lang_id,
                                    "error-no-editor-specified",
                                    None
                                )
                            )
                        })?;
                    tokio::process::Command::new(with_editor)
                        .arg(self.config_file()?)
                        .stdout(Stdio::inherit())
                        .spawn()?
                        .wait()
                        .await?;
                    Ok(())
                } else {
                    Err(anyhow!("unspecified behaviour"))
                }
            }
            Commands::Create {
                r#type: ty,
                hostname,
                target,
            } => match ty.as_str() {
                "git" => {
                    Git::default()
                        .init(
                            self.path_of_repo(ty, hostname, "", target)?
                                .to_string_lossy(),
                        )
                        .await?;
                    Ok(())
                }
                _ => {
                    todo!("more repository type")
                }
            },
            Commands::List {
                filter_type,
                filter_hostname,
                filter_path,
                json,
            } => {
                let mut list = vec![];
                let repo_dir_path = self.repo_dir()?;
                for type_dir in std::fs::read_dir(self.repo_dir()?)? {
                    let type_dir_path = type_dir?.path();
                    let ty = type_dir_path
                        .strip_prefix(&repo_dir_path)?
                        .to_string_lossy()
                        .to_string();
                    if let Some(r#type) = filter_type {
                        if !ty.contains(r#type) {
                            continue;
                        }
                    }
                    for host_dir in std::fs::read_dir(&type_dir_path)? {
                        let host_dir_path = host_dir?.path();
                        let host = host_dir_path
                            .strip_prefix(&type_dir_path)?
                            .to_string_lossy()
                            .to_string();
                        if let Some(hostname) = filter_hostname {
                            if !host.contains(hostname) {
                                continue;
                            }
                        }
                        for repo_dir in filter_git_paths_recursively(&host_dir_path).await? {
                            let repo_path = repo_dir
                                .strip_prefix(&host_dir_path)?
                                .to_string_lossy()
                                .to_string();
                            if let Some(filter_path) = filter_path {
                                if !repo_path.contains(filter_path) {
                                    continue;
                                }
                            }

                            list.push(RepoTableItem {
                                path: repo_path.to_owned(),
                                ty: ty.to_owned(),
                                hostname: host.to_owned(),
                            });
                        }
                    }
                }
                if *json {
                    println!("{}", serde_json::to_string(&list)?);
                } else {
                    println!("{}", tabled::Table::new(list));
                }
                Ok(())
            }
            Commands::Debug { commands } => match commands {
                DebugCommands::Locale => {
                    println!(
                        "{}",
                        sys_locale::get_locale().unwrap_or("MISSING".to_string())
                    );
                    Ok(())
                }
                DebugCommands::LocaleText { key } => {
                    println!("{}", self.i18n.format_msg_or_log(&self.lang_id, key, None));
                    Ok(())
                }
            },
        }
    }
}
