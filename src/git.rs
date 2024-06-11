use std::{path::PathBuf, process::Stdio};

use anyhow::anyhow;
use lazy_regex::regex_captures;
use tokio::process::Command;
use url::Url;

pub struct Git {
    exe: String,
}

impl Default for Git {
    fn default() -> Self {
        Self {
            exe: "git".to_string(),
        }
    }
}

impl Git {
    pub async fn clone(
        &self,
        target: impl AsRef<str>,
        path: impl AsRef<str>,
    ) -> anyhow::Result<std::process::ExitStatus> {
        Command::new(&self.exe)
            .arg("clone")
            .arg("--")
            .arg(target.as_ref())
            .arg(path.as_ref())
            .stdout(Stdio::inherit())
            .spawn()?
            .wait()
            .await
            .map_err(anyhow::Error::new)
    }

    pub async fn init(&self, path: impl AsRef<str>) -> anyhow::Result<std::process::ExitStatus> {
        Command::new(&self.exe)
            .arg("init")
            .arg(path.as_ref())
            .stdout(Stdio::inherit())
            .spawn()?
            .wait()
            .await
            .map_err(anyhow::Error::new)
    } 
}

pub enum GitUrl {
    Ssh {
        user: Option<String>,
        host: String,
        port: Option<u16>,
        username: Option<String>,
        path: String,
    },
    Git {
        host: String,
        port: Option<u16>,
        username: Option<String>,
        path: String,
    },
    Http {
        https: bool,
        host: String,
        port: Option<u16>,
        path: String,
    },
    Ftp {
        ftps: bool,
        host: String,
        port: Option<u16>,
        path: String,
    },
    File {
        path: String,
    },
}

impl GitUrl {
    pub fn parse(url: impl AsRef<str>) -> anyhow::Result<Self> {
        let url = url.as_ref();
        if let Ok(url) = Url::parse(url) {
            match url.scheme().to_lowercase().as_str() {
                "ssh" => {
                    let user = url.username();
                    let user = if user.is_empty() {
                        None
                    } else {
                        Some(user.to_string())
                    };
                    let host = url.host_str().ok_or(anyhow!("empty host"))?.to_string();
                    let port = url.port();
                    let path = url.path().to_string();
                    Ok(GitUrl::Ssh {
                        user,
                        host,
                        port,
                        username: None,
                        path,
                    })
                }
                "git" => {
                    let host = url.host_str().ok_or(anyhow!("empty host"))?.to_string();
                    let port = url.port();
                    let path = url.path().to_string();
                    Ok(GitUrl::Git {
                        host,
                        port,
                        username: None,
                        path,
                    })
                }
                "http" | "https" => {
                    let https = url.scheme() == "https";
                    let host = url.host_str().ok_or(anyhow!("empty host"))?.to_string();
                    let port = url.port();
                    let path = url.path().to_string();
                    Ok(GitUrl::Http {
                        https,
                        host,
                        port,
                        path,
                    })
                }
                "ftp" | "ftps" => {
                    let ftps = url.scheme() == "ftps";
                    let host = url.host_str().ok_or(anyhow!("empty host"))?.to_string();
                    let port = url.port();
                    let path = url.path().to_string();
                    Ok(GitUrl::Ftp {
                        ftps,
                        host,
                        port,
                        path,
                    })
                }
                "file" => {
                    let path = url.path().to_string();
                    Ok(GitUrl::File { path })
                }
                schema => Err(anyhow!("invalid url schema: '{}'", schema)),
            }
        } else if let Some((_, user, host, username, path)) =
            regex_captures!(r#"([^@/]+@)?([^:/]+):([^/]+)?/(.+)"#, url)
        {
            let user = if user.is_empty() {
                None
            } else {
                Some(user.to_string())
            };
            let host = host.to_string();
            let port = None;
            let username = if username.is_empty() {
                None
            } else {
                Some(username.to_string())
            };
            let path = path.to_string();
            Ok(GitUrl::Ssh {
                user,
                host,
                port,
                username,
                path,
            })
        } else {
            let path = PathBuf::from(url).to_string_lossy().to_string();
            Ok(GitUrl::File { path })
        }
    }

    pub fn username(&self) -> String {
        match self {
            GitUrl::Ssh { username, .. } => username.to_owned().unwrap_or_default(),
            GitUrl::Git { username, .. } => username.to_owned().unwrap_or_default(),
            GitUrl::Http { .. } => "".to_string(),
            GitUrl::Ftp { .. } => "".to_string(),
            GitUrl::File { .. } => "".to_string(),
        }
    }

    pub fn host(&self) -> &str {
        match self {
            GitUrl::Ssh { host, .. } => host,
            GitUrl::Git { host, .. } => host,
            GitUrl::Http { host, .. } => host,
            GitUrl::Ftp { host, .. } => host,
            GitUrl::File { .. } => "local",
        }
    }

    pub fn path(&self) -> &str {
        match self {
            GitUrl::Ssh { path, .. } => path,
            GitUrl::Git { path, .. } => path,
            GitUrl::Http { path, .. } => path,
            GitUrl::Ftp { path, .. } => path,
            GitUrl::File { path, .. } => path,
        }
    }
}
