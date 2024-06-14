use std::{env::current_dir, path::PathBuf, process::Stdio};

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use tabled::Tabled;

use crate::{
    config::Config,
    git::{filter_git_paths_recursively, Git, GitUrl},
};

#[derive(Parser)]
#[command(version = "snapshot", about = "A repository manager.", long_about = None)]
pub struct Cli {
    #[arg(long, default_value = "false")]
    system: bool,
    #[arg(long, default_value = "false")]
    user: bool,
    #[arg(long, default_value = "false")]
    local: bool,
    #[arg(short, long)]
    config: Option<String>,
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Clone {
        #[arg(long, default_value = "git")]
        r#type: String,
        target: String,
    },
    Setup,
    Open {
        #[arg(long)]
        with: Option<String>,
        target: String,
    },
    Config {
        #[arg(long)]
        edit: bool,
        #[arg(long)]
        with: Option<String>,
    },
    Create {
        #[arg(long, default_value = "git")]
        r#type: String,
        #[arg(long, default_value = "localhost")]
        hostname: String,
        target: String,
    },
    List {
        #[arg(long)]
        filter_type: Option<String>,
        #[arg(long)]
        filter_hostname: Option<String>,
        #[arg(long)]
        filter_path: Option<String>,
        #[arg(long, default_value = "false")]
        json: bool,
    },
}

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
}

#[derive(Tabled, serde::Serialize)]
pub struct RepoTableItem {
    path: String,
    #[tabled(rename = "type")]
    ty: String,
    hostname: String,
}
impl Rer {
    fn default_open_with(&self) -> Option<String> {
        self.config.default_open_with.to_owned()
    }

    fn config_file(&self) -> anyhow::Result<PathBuf> {
        Ok(match self.setup {
            RerSetup::System => {
                if cfg!(target_os = "linux") {
                    PathBuf::from("/etc/rerman.toml")
                } else {
                    panic!("not supported os")
                }
            }
            RerSetup::User => {
                if cfg!(target_os = "linux") {
                    PathBuf::from(
                        std::env::var("XDG_CONFIG_HOME")
                            .unwrap_or(std::env::var("HOME")? + "/.config"),
                    )
                    .join("rerman.toml")
                } else {
                    panic!("not supported os")
                }
            }
            RerSetup::Local => (current_dir()?).join(".rerman").join("rerman.toml"),
            RerSetup::Custom { ref config_file } => config_file.to_owned(),
        })
    }

    fn repo_dir(&self) -> anyhow::Result<PathBuf> {
        if let Some(ref repo_dir) = self.config.repo_dir {
            Ok(PathBuf::from(repo_dir))
        } else {
            match self.setup {
                RerSetup::Local => Ok(current_dir()?.join(".rerman").join("repositories")),
                RerSetup::User => {
                    if cfg!(target_os = "linux") {
                        Ok(PathBuf::from(std::env::var("XDG_DATA_HOME").unwrap_or(
                            std::env::var("HOME")? + "/.local/share/rerman/repositories",
                        )))
                    } else {
                        panic!("not supported os")
                    }
                }
                RerSetup::System => {
                    if cfg!(target_os = "linux") {
                        Ok(PathBuf::from("/usr/share/rerman/repositories"))
                    } else {
                        panic!("not supported os")
                    }
                }
                RerSetup::Custom { .. } => Ok(PathBuf::from(
                    self.config
                        .repo_dir
                        .as_ref()
                        .ok_or_else(|| anyhow!("no repositories dir specified"))?,
                )),
            }
        }
    }

    fn path_of(
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

    pub async fn parse() -> anyhow::Result<Self> {
        let cli = Cli::parse();
        let setup = if cli.system {
            RerSetup::System
        } else if cli.user {
            RerSetup::User
        } else if cli.local {
            RerSetup::Local
        } else if let Some(ref config) = cli.config {
            RerSetup::Custom {
                config_file: PathBuf::from(config),
            }
        } else {
            RerSetup::User
        };
        let config_file = match setup {
            RerSetup::System => {
                if cfg!(target_os = "linux") {
                    PathBuf::from("/etc/rerman.toml")
                } else {
                    panic!("not supported os")
                }
            }
            RerSetup::User => {
                if cfg!(target_os = "linux") {
                    PathBuf::from(
                        std::env::var("XDG_CONFIG_HOME")
                            .unwrap_or(std::env::var("HOME")? + "/.config"),
                    )
                    .join("rerman.toml")
                } else {
                    panic!("not supported os")
                }
            }
            RerSetup::Local => (current_dir()?).join(".rerman").join("rerman.toml"),
            RerSetup::Custom { ref config_file } => config_file.to_owned(),
        };
        let config = tokio::fs::read(config_file).await.or_else(|_| {
            println!("config file read failed...use default");
            toml::to_string(&Config::default()).map(|v| v.as_bytes().to_vec())
        })?;
        let config: Config = toml::from_str(&String::from_utf8(config)?).unwrap_or_else(|_| {
            println!("config file read failed...use default");
            Config::default()
        });
        Ok(Rer { cli, setup, config })
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
                                .join(
                                    url.path()
                                        .strip_prefix('/')
                                        .map(|x| x.strip_suffix(".git").unwrap_or(x))
                                        .unwrap_or(url.path()),
                                )
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
                let config_dir = config_file
                    .parent()
                    .ok_or_else(|| anyhow!("config dir not valid"))?;
                if config_dir.exists() && !config_dir.is_dir() {
                    Err(anyhow!(
                        "config dir '{}' is not a valid directory",
                        config_dir.to_string_lossy()
                    ))?
                }
                if !config_dir.exists() {
                    tokio::fs::create_dir_all(config_dir).await?;
                }

                let repo_dir = self.repo_dir()?;
                if repo_dir.exists() && !repo_dir.is_dir() {
                    Err(anyhow!(
                        "repo dir '{}' is not a valid directory",
                        repo_dir.to_string_lossy()
                    ))?
                }
                if !repo_dir.exists() {
                    tokio::fs::create_dir_all(repo_dir).await?;
                }

                tokio::fs::write(
                    &config_file,
                    format!(
                        "# repo_dir = \"{}\"\n# default_open_with = \"code\"",
                        self.repo_dir()?.to_string_lossy()
                    )
                    .as_bytes(),
                )
                .await?;
                println!("setup completed: {}", config_file.to_string_lossy());
                Ok(())
            }
            Commands::Open { with, target } => {
                let open_with = with.to_owned().ok_or(()).or_else(|_| {
                    self.default_open_with()
                        .ok_or(anyhow!("no default open with specified"))
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
                Err(anyhow!("target not found"))
            }
            Commands::Config { edit, with } => {
                if *edit {
                    let with_editor = with
                        .to_owned()
                        .or_else(|| self.config.default_config_editor.to_owned())
                        .or_else(|| {
                            if cfg!(target_os = "linux") {
                                std::env::var("EDITOR").ok()
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| anyhow!("no editor specified"))?;
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
                        .init(self.path_of(ty, hostname, "", target)?.to_string_lossy())
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
        }
    }
}
