#[derive(Debug, Clone)]
pub struct Plugin {
    pub slug: String,
    pub name: String,
    pub active_installs: i64,
    pub last_updated: Option<String>,
    pub page: i32,
}

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub slug: String,
    pub success: bool,
    pub message: String,
    pub version: Option<String>,
}
