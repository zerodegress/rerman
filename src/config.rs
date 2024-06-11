#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Config {
    pub repo_dir: Option<String>,
    pub default_open_with: Option<String>,
    pub default_config_editor: Option<String>,
}
