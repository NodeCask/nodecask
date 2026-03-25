use crate::store::link::LinkCollection;
use crate::store::system::{InjectionConfig, VisitConfig, Website};
use crate::store::Store;
use arc_swap::ArcSwap;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct GlobalConfig {
    pub(crate) website: Website,              // 网站基础信息
    pub(crate) links: Vec<LinkCollection>,    // 网站底部导航链接
    pub(crate) injection: InjectionConfig,    // 网页全局插入 HTML 代码
    pub(crate) visit: VisitConfig,            // 分页配置
    pub(crate) overview: (Overview, Instant), // 统计信息，还有更新时间
}

/// 网站统计信息
#[derive(Copy, Debug, Clone, Default, FromRow, Serialize, Deserialize)]
pub struct Overview {
    pub(crate) users: i64,    // 会员总数
    pub(crate) topics: i64,   // 帖子总数
    pub(crate) comments: i64, // 评论总数
}

#[derive(Clone)]
pub struct GlobalConfigDaemon {
    inner: Arc<ArcSwap<GlobalConfig>>,
    channel: tokio::sync::mpsc::Sender<ReloadInstruct>,
}

pub enum ReloadInstruct {
    Website,
    Links,
    Injection,
    Visit,
    Overview,
}

impl GlobalConfigDaemon {
    pub async fn new(store: Store) -> Self {
        let cfg: GlobalConfig = store.get_global_config().await;
        let cfg = Arc::new(ArcSwap::from_pointee(cfg));
        let cfg_cp = Arc::clone(&cfg);
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<ReloadInstruct>(1);
        tokio::spawn(async move {
            while let Some(t) = receiver.recv().await {
                let old = (**cfg.load()).clone();
                let neo: GlobalConfig = match t {
                    ReloadInstruct::Website => GlobalConfig {
                        website: store.get_website().await,
                        ..old
                    },
                    ReloadInstruct::Links => GlobalConfig {
                        links: store.get_links().await,
                        ..old
                    },
                    ReloadInstruct::Injection => GlobalConfig {
                        injection: store.get_injection_config().await,
                        ..old
                    },
                    ReloadInstruct::Visit => GlobalConfig {
                        visit: store.get_visit_config().await,
                        ..old
                    },
                    ReloadInstruct::Overview => GlobalConfig {
                        overview: (store.get_overview().await.unwrap_or_default(), Instant::now()),
                        ..old
                    }
                };
                cfg.store(Arc::new(neo));
            }
            info!("GlobalConfigDaemon exiting...");
        });
        Self {
            inner: cfg_cp,
            channel: sender,
        }
    }
    pub fn get(&self) -> GlobalConfig {
        (**self.inner.load()).clone()
    }
    pub fn load(&self) -> arc_swap::Guard<Arc<GlobalConfig>> {
        self.inner.load()
    }
    pub async fn reload(&self, t: ReloadInstruct) {
        if let Err(err) = self.channel.send(t).await {
            warn!("Failed to send reload task: {}", err);
        }
    }
}
