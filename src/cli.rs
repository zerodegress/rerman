use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version = "snapshot", about = "A repository manager.", long_about = None)]
pub struct Cli {
    #[arg(long)]
    pub system: Option<bool>,
    #[arg(long)]
    pub user: Option<bool>,
    #[arg(long)]
    pub local: Option<bool>,
    #[arg(short, long)]
    pub config: Option<String>,
    #[command(subcommand)]
    pub commands: Commands,
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
    Debug {
        #[command(subcommand)]
        commands: DebugCommands,
    },
}

#[derive(Subcommand)]
pub enum DebugCommands {
    Locale,
    LocaleText { key: String },
}
