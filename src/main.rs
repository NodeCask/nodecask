mod common;
mod daemon;
mod home;
mod markdown;
mod moderator;
mod store;
mod wizard;

use crate::daemon::config::GlobalConfigDaemon;
use crate::daemon::link_filter::LinkFilter;
use crate::daemon::notify::NotifyDaemon;
use crate::home::{router};
use log::{error, info};
use object_store::local::LocalFileSystem;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tower_sessions::cookie::time::Duration;
use tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use crate::common::{AppState};
use shadow_rs::shadow;

shadow!(build);

rust_i18n::i18n!("locales", fallback = "en_US");
#[macro_export]
macro_rules! t {
    ($ctx:expr, $key:expr) => {
        rust_i18n::t!($key, locale = crate::common::LocaleProvider::locale(&$ctx))
    };

    ($ctx:expr, $key:expr, $($args:tt)+) => {
        rust_i18n::t!($key, locale = crate::common::LocaleProvider::locale(&$ctx), $($args)+)
    };
}
#[macro_export]
macro_rules! t_owned {
    ($($arg:tt)+) => {
        $crate::t!($($arg)+).to_string()
    };
}
#[tokio::main]
async fn main() {
    colog::init();
    info!("version: {}", build::BUILD_TIMESTAMP);
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let port = std::env::var("BIND_PORT")
        .map(|x| {
            x.parse::<u16>()
                .expect("BIND_PORT is not a valid port number")
        })
        .unwrap_or(3000);
    let db_url = "sqlite://data/db.sqlite3";
    let pool = SqlitePool::connect(db_url)
        .await
        .expect("Failed to connect to database");

    let store = store::Store { pool: pool.clone() };
    // 检查数据库是否需要初始化
    wizard::wizard(&store).await;

    info!("Listening on http://localhost:{port}");
    let Ok(listener) = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .inspect_err(|err| error!("Failed to bind port: {}", err))
    else {
        return;
    };

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.unwrap();
    let _deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_mins(1)), // 一分钟清理一次过期 session
    );
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(30))); // 30 分钟不刷新网页就掉线

    let moderator = daemon::moderate::ModerateDaemon::new(store.clone()).await; // 引入后台审核进程
    let cfg = GlobalConfigDaemon::new(store.clone()).await;
    let notify = NotifyDaemon::new(store.clone()).await;
    let email = daemon::email::EmailSenderDaemon::new(store.clone()).await;
    let login_rewards = daemon::login_rewards::LoginRewardsDaemon::new(store.clone());

    // 加载帖子内容链接过滤器
    let link_filter = LinkFilter::new();
    let link_filter_cfg = store.get_link_filter().await;
    link_filter.update(link_filter_cfg.rules);

    let searcher = daemon::tantivy::SearchDaemon::new(store.clone(), "data/tantivy")
        .expect("Failed to init search daemon");
    let nsfw_detector =
        daemon::nsfw_detect::NSFWDetector::new().expect("Failed to init nsfw detector");

    let _ = std::fs::create_dir_all("data/store");
    let p = PathBuf::from("data/store");
    let fs = LocalFileSystem::new_with_prefix(&p).unwrap();
    let state = AppState {
        sender: tokio::sync::broadcast::channel(128).0,
        pool,
        moderator,
        cfg,
        notify,
        email,
        link_filter,
        login_rewards,
        searcher,
        nsfw_detector,
        fs: Arc::new(Box::new(fs)),
    };

    let app = router(state).layer(session_layer);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
    info!("ByeBye");
}

async fn shutdown_signal() {
    // 监听 Ctrl+C (SIGINT)
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    // 监听 UNIX 系统的 SIGTERM (通常用于 Docker/Kubernetes 停止容器)
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    // 如果不是 UNIX 系统，让 terminate 永远挂起（不执行）
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // 只要有任意一个信号触发，就结束
    tokio::select! {
        _ = ctrl_c => {
            println!("收到 Ctrl+C 信号，准备退出...");
        },
        _ = terminate => {
            println!("收到 SIGTERM 信号，准备退出...");
        },
    }
}
