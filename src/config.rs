#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Config {
    pub repo_dir: Option<String>,
    pub open_with: Option<String>,
    pub config_editor: Option<String>,
}
