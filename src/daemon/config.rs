use crate::store::link::LinkCollection;
use crate::store::system::{InjectionConfig, VisitConfig, Website};
use crate::store::Store;
use arc_swap::ArcSwap;
use log::{info, warn};
use std::sync::Arc;

#[derive(Clone)]
pub struct GlobalConfig {
    pub(crate) website: Website,           // 网站基础信息
    pub(crate) links: Vec<LinkCollection>, // 网站底部导航链接
    pub(crate) injection: InjectionConfig, // 网页全局插入 HTML 代码
    pub(crate) visit: VisitConfig,         // 分页配置
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
