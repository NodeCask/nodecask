use crate::store::Store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub title: String,
    pub url: String,
    pub description: String,
    pub blank: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkCollection {
    pub title: String,
    pub links: Vec<Link>,
}

impl Store {
    // 查询底部链接列表
    pub async fn get_links(&self) -> Vec<LinkCollection> {
        self.get_cfg("site_bottom_links").await.unwrap_or_default()
    }
}
