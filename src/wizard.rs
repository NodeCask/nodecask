use crate::store::link::{Link, LinkCollection};
use crate::store::system::{UserLoginRewardsScript, Website};
use crate::store::Store;
use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{Form, Router};
use chrono::{DateTime, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use axum::http::HeaderMap;
use into_response_derive::TemplateResponse;
use tokio::sync::mpsc::Sender;
use crate::home::auth::hash_password;

const WIZARD_CONFIG_NAME: &str = "wizard";

#[derive(FromRow, Serialize, Deserialize, Default)]
struct WizardConfig {
    date: DateTime<Utc>,
    hostname: String,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/wizard.html")]
struct WizardTemplate {
    tips: String,
    success: bool,
    // Form fields for repopulation
    site_name: String,
    site_domain: String,
    site_desc: String,
    admin_username: String,
    admin_password: String,
    admin_email: String,
    smtp_host: String,
    smtp_port: String,
    smtp_user: String,
    smtp_pass: String,
    smtp_from_name: String,
    smtp_from_email: String,
    smtp_tls: bool,
}

impl Default for WizardTemplate {
    fn default() -> Self {
        Self {
            tips: "".to_string(),
            success: false,
            site_name: "NodeCask".to_string(),
            site_domain: "".to_string(),
            site_desc: "一个轻量级的论坛社区".to_string(),
            admin_username: "admin".to_string(),
            admin_password: "".to_string(),
            admin_email: "".to_string(),
            smtp_host: "".to_string(),
            smtp_port: "465".to_string(),
            smtp_user: "".to_string(),
            smtp_pass: "".to_string(),
            smtp_from_name: "".to_string(),
            smtp_from_email: "".to_string(),
            smtp_tls: true,
        }
    }
}

impl From<WizardForm> for WizardTemplate {
    fn from(form: WizardForm) -> Self {
        Self {
            tips: "".to_string(),
            success: false,
            site_name: form.site_name,
            site_domain: form.site_domain,
            site_desc: form.site_desc,
            admin_username: form.admin_username,
            admin_password: form.admin_password,
            admin_email: form.admin_email,
            smtp_host: form.smtp_host.unwrap_or_default(),
            smtp_port: form.smtp_port.unwrap_or_default(),
            smtp_user: form.smtp_user.unwrap_or_default(),
            smtp_pass: form.smtp_pass.unwrap_or_default(),
            smtp_from_name: form.smtp_from_name.unwrap_or_default(),
            smtp_from_email: form.smtp_from_email.unwrap_or_default(),
            smtp_tls: form.smtp_tls,
        }
    }
}

#[derive(Deserialize, Clone, Default)]
struct WizardForm {
    // Website
    site_name: String,
    site_domain: String,
    site_desc: String,
    // Admin
    admin_username: String,
    admin_password: String,
    admin_email: String,
    // SMTP
    smtp_host: Option<String>,
    smtp_port: Option<String>,
    smtp_user: Option<String>,
    smtp_pass: Option<String>,
    smtp_from_name: Option<String>,
    smtp_from_email: Option<String>,
    #[serde(default)]
    smtp_tls: bool,
}

#[derive(Serialize)]
struct SmtpConfig {
    from_name: String,
    from_mail: String,
    hostname: String,
    port: u16,
    tls_implicit: bool,
    username: String,
    password: String,
}

#[derive(Clone)]
struct WizardState {
    time: DateTime<Utc>,
    store: Store,
    shutdown_tx: Arc<Sender<()>>,
    exiting: Arc<AtomicBool>,
}

pub async fn wizard(store: &Store) {
    let cfg: Option<WizardConfig> = store.get_cfg(WIZARD_CONFIG_NAME).await;
    if cfg.is_some() {
        info!("Not first run, skipping installation wizard");
        return;
    }

    info!("Starting installation wizard...");

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let state = WizardState {
        time: Utc::now(),
        store: store.clone(),
        shutdown_tx: Arc::new(tx),
        exiting: Arc::new(AtomicBool::new(false)),
    };

    let app = Router::new()
        .route("/", get(wizard_page).post(install))
        .route("/public/{*path}", get(crate::home::public::public_handler))
        .with_state(state);

    let port = std::env::var("BIND_PORT")
        .map(|x| {
            x.parse::<u16>()
                .expect("BIND_PORT is not a valid port number")
        })
        .unwrap_or(3000);

    info!("Wizard listening on http://0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            rx.recv().await;
            info!("Wizard completed, shutting down...");
        })
        .await
        .unwrap();
    // 等待 2s 等待端口释放
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}

async fn wizard_page(headers: HeaderMap) -> impl IntoResponse {
    let host = headers.get("Host")
        .and_then(|host| host.to_str().ok())
        .unwrap_or("localhost")
        .to_owned();
    let t = WizardTemplate {
        site_domain: format!("https://{}", host),
        ..WizardTemplate::default()
    };
    match t.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => Html(format!("Template error: {}", err)).into_response(),
    }
}

async fn install(
    State(state): State<WizardState>,
    Form(form): Form<WizardForm>,
) -> impl IntoResponse {
    let mut template = WizardTemplate::from(form.clone());
    if state.exiting.load(Ordering::Relaxed) {
        template.success = true;
        template.tips = "程序已经安装完成，请等待刷新。".to_string();
        return template.into_response();
    }
    if state.time + std::time::Duration::from_hours(1) < Utc::now() {
        template.success = true;
        template.tips = "安装流程超时，请重新启动程序。".to_string();
        return template.into_response();
    }

    // Basic Validation
    if form.site_name.trim().is_empty() {
        template.tips = "网站名称不能为空".to_string();
        return template.into_response();
    }
    if form.site_domain.trim().is_empty() {
        template.tips = "网站域名不能为空".to_string();
        return template.into_response();
    }
    if form.admin_username.trim().is_empty() {
        template.tips = "管理员用户名不能为空".to_string();
        return template.into_response();
    }
    if form.admin_password.is_empty() {
        template.tips = "管理员密码不能为空".to_string();
        return template.into_response();
    }
    if form.admin_email.trim().is_empty() {
        template.tips = "管理员邮箱不能为空".to_string();
        return template.into_response();
    }

    // 1. Save Website Config
    let website = Website {
        name: form.site_name,
        domain: form.site_domain,
        description: form.site_desc,
        ..Default::default()
    };
    if let Err(e) = state.store.set_cfg("site_info", Some(&website)).await {
        template.tips = format!("保存网站配置失败: {}", e);
        return template.into_response();
    }

    // 2. Create Admin User
    let password_hash = match hash_password(&form.admin_password) {
        Ok(h) => h,
        Err(e) => {
            template.tips = format!("密码加密失败: {}", e);
            return template.into_response();
        }
    };

    if let Err(e) = state
        .store
        .create_user_with_invite(
            &form.admin_username,
            &password_hash,
            &form.admin_email,
            "zh_CN",
            999999, // High initial score/credits
            None
        )
        .await
    {
        template.tips = format!("创建管理员用户失败: {}", e);
        return template.into_response();
    }

    // Set admin role
    if let Ok(Some(user)) = state.store.get_user(&form.admin_username).await {
        if let Err(e) = state.store.update_user_role(user.id, "administrator").await {
            template.tips = format!("设置管理员权限失败: {}", e);
            return template.into_response();
        }
    }

    // 3. Save SMTP Config (if provided)
    let smtp_host = form.smtp_host.filter(|s| !s.is_empty());
    let smtp_port = form
        .smtp_port
        .filter(|s| !s.is_empty())
        .and_then(|p| p.parse::<u16>().ok());

    if let (Some(host), Some(port)) = (smtp_host, smtp_port) {
        let smtp = SmtpConfig {
            hostname: host,
            port,
            username: form.smtp_user.unwrap_or_default(),
            password: form.smtp_pass.unwrap_or_default(),
            from_name: form.smtp_from_name.unwrap_or_default(),
            from_mail: form.smtp_from_email.unwrap_or_default(),
            tls_implicit: form.smtp_tls,
        };
        if let Err(e) = state.store.set_cfg("smtp_config", Some(&smtp)).await {
            warn!("Error saving SMTP config: {}", e);
        }
    }

    // 4. Default Data
    nodes(&state.store).await;
    topics(&state.store).await;
    pages(&state.store).await;
    links(&state.store).await;
    user_rewards_script(&state.store).await;

    // 5. Mark Wizard Complete
    let cfg = WizardConfig {
        date: Utc::now(),
        hostname: sysinfo::System::name().unwrap_or("unknown".to_string()),
    };
    if let Err(e) = state.store.set_cfg(WIZARD_CONFIG_NAME, Some(&cfg)).await {
        template.tips = format!("保存安装状态失败: {}", e);
        return template.into_response();
    }

    // 6. Signal Shutdown (delayed)
    let tx = state.shutdown_tx.clone();
    tokio::spawn(async move {
        state.exiting.store(true, Ordering::Relaxed);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let _ = tx.send(()).await;
    });

    template.success = true;
    template.tips = "安装成功！系统将在 2 秒后重启，请稍后刷新页面。".to_string();
    template.into_response()
}

// 添加默认节点
async fn nodes(store: &Store) {
    if let Err(err) = store
        .create_node(
            "回收站",
            "trashed",
            "查看系统删除帖子",
            false,
            "",
            "",
            "",
            "",
            true,
            false,
            true,
            true,
            0,
            0,
        )
        .await
    {
        warn!("Failed to create trash node: {}", err);
    }
    if let Err(err) = store
        .create_node(
            "未分类",
            "unclassified",
            "没有归属的帖子存在在这个节点。",
            false,
            "",
            "",
            "",
            "",
            true,
            false,
            true,
            true,
            0,
            0,
        )
        .await
    {
        warn!("Failed to create unclassified node: {}", err);
    }
    async fn create(store: &Store, name: &str, slug: &str, description: &str) {
        info!("Creating node: {} ({})", name, slug);
        if let Err(err) = store
            .create_node(
                name,
                slug,
                description,
                true,
                "",
                "",
                "",
                "",
                false,
                false,
                false,
                false,
                0,
                0,
            )
            .await
        {
            warn!("failed to create node {}, error: {}", name, err);
        }
    }

    create(store, "日常", "daily", "").await;
    create(store, "分享", "share", "").await;
    create(store, "科技", "tech", "").await;
    create(store, "金融", "finance", "").await;
    create(store, "旅行", "travel", "").await;
    create(store, "游戏", "game", "").await;
}

// 新增一些默认页面
async fn pages(store: &Store) {
    if let Err(err) = store
        .create_page(
            "about",
            "About Us",
            "",
            "html",
            include_str!("../templates/pages/about.html"),
        )
        .await
    {
        warn!("Failed to create about page: {}", err);
    }
    if let Err(err) = store
        .create_page(
            "help",
            "Help",
            "",
            "html",
            include_str!("../templates/pages/help.html"),
        )
        .await
    {
        warn!("Failed to create help page: {}", err);
    }
    if let Err(err) = store
        .create_page(
            "rules",
            "Rules",
            "",
            "html",
            include_str!("../templates/pages/rules.html"),
        )
        .await
    {
        warn!("Failed to create rules page: {}", err);
    }
    if let Err(err) = store
        .create_page(
            "privacy",
            "Privacy",
            "",
            "html",
            include_str!("../templates/pages/privacy.html"),
        )
        .await
    {
        warn!("Failed to create privacy page: {}", err);
    }
}

// 新增一些底部链接

async fn links(store: &Store) {
    let links: Vec<LinkCollection> = vec![
        LinkCollection {
            title: "站内导航".to_string(),
            links: vec![
                Link {
                    title: "关于我们".to_string(),
                    url: "/about".to_string(),
                    description: "".to_string(),
                    blank: false,
                },
                Link {
                    title: "隐私政策".to_string(),
                    url: "/privacy".to_string(),
                    description: "".to_string(),
                    blank: false,
                },
                Link {
                    title: "网站规则".to_string(),
                    url: "/rules".to_string(),
                    description: "".to_string(),
                    blank: false,
                },
                Link {
                    title: "帮助信息".to_string(),
                    url: "/help".to_string(),
                    description: "".to_string(),
                    blank: false,
                },
            ],
        },
        LinkCollection {
            title: "友情链接".to_string(),
            links: vec![Link {
                title: "Github".to_string(),
                url: "https://github.com".to_string(),
                description: "".to_string(),
                blank: true,
            }],
        },
    ];
    if let Err(err) = store.set_cfg("site_bottom_links", Some(&links)).await {
        warn!("Failed to set site navigation links: {}", err);
    }
}

async fn topics(store: &Store) {
    let Ok(Some(node)) = store.get_node("daily").await else {
        warn!("Error querying node");
        return;
    };
    let content = "如果你看到这个帖子，说明程序安装完成！".to_string();
    if let Err(err) = store
        .new_topic(
            1,
            node.id,
            "第一个帖子",
            &content,
            &content,
            &content,
        )
        .await
    {
        warn!("Failed to create topic: {}", err)
    }
}

const USER_LOGIN_REWARDS_SCRIPT: &str = include_str!("../assets/rewards.js");
async fn user_rewards_script(store: &Store) {
    let cfg = UserLoginRewardsScript {
        script: USER_LOGIN_REWARDS_SCRIPT.to_string(),
    };
    if let Err(err) = store.set_cfg("login_rewards_script", Some(&cfg)).await {
        warn!("Failed to set login rewards script: {}", err)
    }
}
