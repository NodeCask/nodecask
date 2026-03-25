use std::time::Instant;
use anyhow::anyhow;
use crate::daemon::config::{GlobalConfig, Overview};
use crate::store::Store;
use log::{error, warn};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::store::topic::TopicDisplay;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Website {
    pub name: String,        // 网站名称
    pub logo: String,        // logo 图片地址
    pub nickname: String,    // 网站名称缩写
    pub domain: String,      // 网站域名
    pub description: String, // 网站描述
    pub keyword: String,     // 关键词
    pub copyright: String,   // 版权声明
    pub about: String,       // 关于我们部分
}
impl Default for Website {
    fn default() -> Self {
        Self {
            name: "NodeCask".to_string(),
            logo: "/public/logo.png".to_string(),
            nickname: "NodeCask".to_string(),
            domain: "".to_string(),
            description: "".to_string(),
            keyword: "".to_string(),
            copyright: "".to_string(),
            about: "".to_string(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VisitConfig {
    pub topics_per_page: u32,    // 每页展示帖子数量
    pub comments_per_page: u32,  // 帖子内每页展示回复数量
}
impl Default for VisitConfig {
    fn default() -> Self {
        Self {
            topics_per_page: 20,
            comments_per_page: 100,
        }
    }
}
impl Store {
    pub async fn get_cfg<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        if key.is_empty() {
            return None;
        }
        let res = sqlx::query_as::<_, (String,)>(
            r#"
        SELECT value
        FROM system_config
        WHERE key = ?
    "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or_else(|err| {
            error!("failed to fetch configuration (key: {}): {}", key, err);
            None
        });
        res.and_then(|(v,)| serde_json::from_str::<T>(v.as_ref()).ok())
    }

    pub async fn set_cfg<T: Serialize>(&self, key: &str, value: Option<&T>) -> sqlx::Result<()> {
        match value {
            Some(value) => {
                let value = serde_json::to_string(value)
                    .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
                let _ = sqlx::query(
                    r#"INSERT INTO system_config (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = ? "#,
                )
                    .bind(key)
                    .bind(&value)
                    .bind(&value)
                    .execute(&self.pool)
                    .await?;
            }
            None => {
                let _ = sqlx::query(r#" delete FROM system_config WHERE key = ? "#)
                    .bind(key)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_token(&self, uid: i64, key: &str) -> sqlx::Result<String> {
        let result = Uuid::new_v4().to_string();
        let _ = sqlx::query("insert into token (action, user_id, token) values (?, ?, ?)")
            .bind(key)
            .bind(uid)
            .bind(&result)
            .execute(&self.pool)
            .await?;
        if result.starts_with("a") {
            let _ = self.clean_token().await;
        }
        Ok(result)
    }
    pub async fn clean_token(&self) -> sqlx::Result<()> {
        let _ = sqlx::query("delete from token where created_at < datetime('now', '-1 hour')")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn remove_token(&self, uid: i64, key: &str) -> sqlx::Result<()> {
        let _ = sqlx::query("delete from token where user_id = ? and action= ?")
            .bind(uid)
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn is_token_exists(&self, uid: i64, key: &str, token: &str) -> sqlx::Result<bool> {
        let result = sqlx::query_as::<_, (bool,)>(
            "select count(*) from token where user_id = ? and action= ? and token = ?",
        )
        .bind(uid)
        .bind(key)
        .bind(token)
        .fetch_optional(&self.pool)
        .await?
        .map(|r| r.0)
        .unwrap_or_default();
        Ok(result)
    }
    pub async fn verify_token(&self, uid: i64, key: &str, token: &str) -> sqlx::Result<bool> {
        let result = sqlx::query("delete from token where token = ? and user_id = ? and action=?")
            .bind(token)
            .bind(uid)
            .bind(key)
            .execute(&self.pool)
            .await?;
        let r = result.rows_affected() == 1;
        if result.rows_affected() == 0 {
            let _ = sqlx::query("delete from token where token = ? ")
                .bind(token)
                .execute(&self.pool)
                .await;
        }
        Ok(r)
    }

    pub async fn get_website(&self) -> Website {
        self.get_cfg("site_info").await.unwrap_or_default()
    }
    pub async fn get_visit_config(&self) -> VisitConfig {
        self.get_cfg("visit_config").await.unwrap_or_default()
    }
    pub async fn get_overview(&self) -> sqlx::Result<Overview> {
        sqlx::query_as::<_, Overview>(
            r#" select
    (select count(*) from user) as users,
    (select count(*) from topic) as topics,
    (select count(*) from comment) as comments
            "#
        ).fetch_one(&self.pool).await
    }
    pub async fn get_link_filter(&self) -> LinkFilterConfig {
        self.get_cfg("link_filter").await.unwrap_or_default()
    }

    pub async fn get_register_config(&self) -> RegisterConfig {
        self.get_cfg("register_config").await.unwrap_or_default()
    }

    pub async fn get_post_config(&self) -> PostConfig {
        self.get_cfg("post_config").await.unwrap_or_default()
    }

    pub async fn get_injection_config(&self) -> InjectionConfig {
        self.get_cfg("injection_config").await.unwrap_or_default()
    }
    pub async fn get_login_rewards_script(&self) -> UserLoginRewardsScript {
        self.get_cfg("login_rewards_script").await.unwrap_or_default()
    }

    pub async fn get_turnstile_config(&self) -> TurnstileConfig {
        self.get_cfg("cloudflare_turnstile").await.unwrap_or_default()
    }

    pub async fn get_global_config(&self) -> GlobalConfig {
        GlobalConfig {
            website: self.get_website().await,
            links: self.get_links().await,
            injection: self.get_injection_config().await,
            visit: self.get_visit_config().await,
            overview: (self.get_overview().await.unwrap_or_default(), Instant::now()),
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RegisterConfig {
    pub enable: bool,
    pub email_verify: bool,
    pub min_username: i64,
    pub max_username: i64,
    pub min_password_len: usize,
    pub terms: String,
    pub invite_code_required: bool,
    pub initial_score: i64,
    pub reserved_username: Vec<String>, // 保留用户名
    pub reserved_prefix: Vec<String>, // 保留前缀
    pub reserved_suffix: Vec<String>, // 保留后缀
}

impl Default for RegisterConfig {
    fn default() -> Self {
        Self {
            enable: false,
            email_verify: false,
            min_username: 3,
            max_username: 20,
            min_password_len: 6,
            terms: "".to_string(),
            invite_code_required: false,
            initial_score: 100,
            reserved_username: vec![],
            reserved_prefix: vec![],
            reserved_suffix: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PostConfig {
    pub sensitive_words: Vec<String>,
    pub min_reg_age_secs: i64,
    pub min_title_length: i64,
    pub max_title_length: i64,
    pub min_content_length: i64,
    pub max_content_length: i64,
    pub min_reply_length: i64,
    pub max_reply_length: i64,
}

impl Default for PostConfig {
    fn default() -> Self {
        Self {
            sensitive_words: vec!["来点色图".to_string()],
            min_reg_age_secs: 0,
            min_title_length: 3,
            max_title_length: 100,
            min_content_length: 0,
            max_content_length: 1000,
            min_reply_length: 1,
            max_reply_length: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InjectionConfig {
    pub style: String,
    pub style_compiled: String,
    pub html_head: String,
    pub html_body: String,
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            style: "".to_string(),
            style_compiled: "".to_string(),
            html_head: "".to_string(),
            html_body: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkFilterConfig {
    pub rules: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UserLoginRewardsScript {
    pub script: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TurnstileConfig {
    pub enable: bool,
    pub site_key:String,
    pub secret_key:String,
    pub hooks: Vec<String>
}

impl TurnstileConfig {
    pub fn get_site_key(&self, hook: &str) -> Option<(String, String)> {
        if !self.enable {
            return None;
        }
        if self.site_key.is_empty() || self.secret_key.is_empty() || self.hooks.is_empty() {
            return None;
        }
        if !self.hooks.contains(&hook.to_string()) {
            return None;
        }
        Some((hook.to_string(), self.site_key.clone()))
    }

    pub async fn validate(&self, action: &str, response: &str) -> anyhow::Result<bool> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "secret": self.secret_key,
            "response": response,
        });
        let response = client
            .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&payload)
            .send()
            .await?;
        #[derive(Deserialize)]
        struct Response {
            success: bool,
            #[serde(alias = "error-codes")]
            error_codes: Vec<String>,
            hostname: Option<String>,
            action: Option<String>,
            challenge_ts: Option<String>,
            cdata: Option<String>,
        }
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed: {}", error_text));
        }
        let response: Response = response.json().await?;
        if !response.success {
            warn!("Cloudflare Turnstile verification failed: {}", response.error_codes.join(", "));
        }
        if !response.action.map(|act| act.as_str() == action).unwrap_or(false) {
            return Ok(false);
        }
        Ok(response.success)
    }
}