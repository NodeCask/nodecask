use crate::daemon::config;
use crate::common::AppState;
use crate::moderator::{Data, ModContext as Context};
use crate::store::Store;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use crate::store::system::UserLoginRewardsScript;

#[axum::debug_handler(state = AppState)]
pub async fn settings(
    _ctx: Context,
    store: Store,
    Query(q): Query<SysConfigQuery>,
) -> impl IntoResponse {
    if q.name.is_empty() {
        return Data::fail("配置名称不能为空").into_response();
    }

    match store.get_cfg::<serde_json::Value>(&q.name).await {
        Some(value) => Data::ok(value).into_response(),
        None => Data::ok(serde_json::Value::Null).into_response(),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn settings_update(
    _ctx: Context,
    store: Store,
    State(state): State<AppState>,
    Json(mut data): Json<SysConfig>,
) -> impl IntoResponse {
    if data.name.is_empty() {
        return Data::fail("配置名称不能为空").into_response();
    }
    if data.name.as_str() == "login_rewards_script" {
        let script_config: UserLoginRewardsScript =
            serde_json::from_value(data.value.clone()).unwrap_or_default();
        if let Err(e) = crate::daemon::login_rewards::is_script_valid(&script_config.script) {
            return Data::fail(&format!("脚本验证失败: {}", e)).into_response();
        }
    }
    if data.name.as_str() == "injection_config" {
        let mut config: crate::store::system::InjectionConfig =
            serde_json::from_value(data.value.clone()).unwrap_or_default();
        match grass::from_string(&config.style, &grass::Options::default()) {
            Ok(css) => {
                config.style_compiled = css;
                match serde_json::to_value(config) {
                    Ok(value) => {
                        data.value = value;
                    }
                    Err(err) => {
                        return Data::fail(&format!("序列化失败: {}", err)).into_response();
                    }
                }
            }
            Err(e) => {
                return Data::fail(&format!("SCSS 编译失败: {}", e)).into_response();
            }
        }
    }

    match store.set_cfg(&data.name, Some(&data.value)).await {
        Ok(_) => {
            // 更新配置之后，需要提醒配置 daemon 重新加载新的数据
            match data.name.as_str() {
                "site_bottom_links" => {
                    state.cfg.reload(config::ReloadInstruct::Links).await;
                }
                "site_info" => {
                    state.cfg.reload(config::ReloadInstruct::Website).await;
                }
                "visit_config" => {
                    state.cfg.reload(config::ReloadInstruct::Visit).await;
                }
                "smtp_config" => {
                    state.email.reload().await;
                }
                "llm_moderator" => {
                    state.moderator.reload().await;
                }
                "link_filter" => {
                    let link_filter_cfg = store.get_link_filter().await;
                    state.link_filter.update(link_filter_cfg.rules);
                }
                "login_rewards_script" => {
                    state.login_rewards.reload();
                }
                "injection_config" => {
                    state.cfg.reload(config::ReloadInstruct::Injection).await;
                }
                _ => {}
            }
            Data::done().into_response()
        }
        Err(e) => Data::fail(&format!("更新配置失败: {}", e)).into_response(),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SysConfig {
    name: String,
    value: serde_json::Value,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SysConfigQuery {
    name: String,
}
