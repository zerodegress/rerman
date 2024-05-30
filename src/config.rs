#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Config {
    pub repo_dir: Option<String>,
    pub default_open_with: Option<String>,
}

impl Config {
    #[allow(dead_code)]
    pub fn is_valid_custom(&self) -> bool {
        if self.repo_dir.is_none() {
            return false;
        }
        true
    }
}

impl Config {}
