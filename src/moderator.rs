mod dashboard;
mod email;
mod invite;
pub(crate) mod node;
pub(crate) mod page;
mod system;
mod token;
pub(crate) mod topic;
pub(crate) mod user;
pub(crate) mod auth;
pub(crate) mod store;

use std::time::Duration;
use crate::common::AppState;
use crate::store::{PaginationQuery, Store};
use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::time::sleep;

#[derive(Debug, Deserialize, Serialize)]
pub struct Data<T: Serialize = ()> {
    code: u32,
    data: Option<T>,
    message: Option<String>,
}

impl<T: Serialize> Data<T> {
    pub fn new(code: u32, data: Option<T>, message: Option<String>) -> Json<Data<T>> {
        Json(Data {
            code,
            data,
            message,
        })
    }
    pub fn error(msg: &str) -> Json<Data<T>> {
        Json(Data {
            code: 100,
            data: None,
            message: Some(msg.to_string()),
        })
    }
    pub fn ok(data: T) -> Json<Data<T>> {
        Json(Data {
            code: 0,
            data: Some(data),
            message: Some("success".to_string()),
        })
    }
}

impl Data<()> {
    // 定义一个别名方法，比如叫 err 或 fail
    // 这个方法不需要推导 T，因为 T 已经被“锁死”为 ()
    pub fn fail(msg: &str) -> Json<Data<()>> {
        Json(Data {
            code: 100,
            data: None,
            message: Some(msg.to_string()),
        })
    }
    pub fn done() -> Json<Data<()>> {
        Json(Data {
            code: 0,
            data: None,
            message: Some("success".to_string()),
        })
    }
}

pub struct PageExtractor(pub PaginationQuery);
impl FromRequestParts<AppState> for PageExtractor {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 获取 query 字符串，如果没有 query 则为空字符串
        let query_str = parts.uri.query().unwrap_or("");

        #[derive(Deserialize)]
        struct PageParams {
            p: Option<u32>,
            per_page: Option<u32>,
        }
        let params: PageParams = serde_urlencoded::from_str(query_str)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid query params: {}", e)))?;

        let p = params.p.unwrap_or(1);
        let per_page = params.per_page.unwrap_or(20);
        Ok(PageExtractor(PaginationQuery {
            p,
            per_page,
        }))
    }
}

#[derive(Deserialize, Serialize, FromRow)]
pub struct Moderator {
    pub uid: i64,
    pub username: String,
}
pub struct ModContext {
    pub store: Store,
    pub moderator: Moderator,
    pub token: String,
}
impl FromRequestParts<AppState> for ModContext {
    type Rejection = Json<Data<()>>;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(token) = parts.headers.get("token") else {
            return Err(Data::new(99,None,Some("missing token".to_string())));
        };
        let token_str = token.to_str().unwrap_or_default().to_string();
        let store = state.store();
        let Ok(Some(moderator)) = store
            .get_moderator(&token_str)
            .await
        else {
            return Err(Data::new(99,None,Some("missing token".to_string())));
        };

        Ok(ModContext {
            store,
            moderator,
            token: token_str,
        })
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(dashboard::dashboard))
        .route("/me", get(dashboard::me))
        .route("/logout", post(dashboard::logout))
        .route("/emails", get(email::list))
        .route("/email-test", post(email::send_test))
        .route("/emails/{id}", get(email::get))
        .route("/invites", get(invite::list))
        .route("/invites/generate", post(invite::generate))
        .route("/invites/{id}/delete", post(invite::delete))
        .route("/invites/{id}/logs", get(invite::logs))
        .route("/nodes", get(node::list).post(node::create))
        .route("/nodes/{id}", post(node::update))
        .route("/nodes/{id}/delete", post(node::delete))
        .route("/pages", get(page::list).post(page::create))
        .route("/pages/{id}", post(page::update))
        .route("/pages/{id}/delete", post(page::delete))
        .route(
            "/settings",
            get(system::settings).post(system::settings_update),
        )
        .route("/users", get(user::list))
        .route("/users/{id}/status", post(user::update_status))
        .route("/users/{id}/role", post(user::update_role))
        .route("/users/{id}/reset-password", post(user::reset_password))
        .route("/topics", get(topic::list))
        .route("/topics-analysis", get(topic::analysis))
        .route("/topics/{id}", post(topic::update))
        .route("/topics/{id}/delete", post(topic::delete))
        .route("/tokens", get(token::list).post(token::create))
        .route("/tokens/{token}/delete", post(token::delete))
        .route("/store/delete", post(store::delete))
        .route("/store/upload", post(store::upload))
        .route("/store/list", post(store::list))
        .route("/store/rename", post(store::rename))
        .route("/store/move", post(store::move_file))
        .route("/store/download", post(store::download))
}

// 这是一个中间件函数
pub async fn delay_middleware(req: Request, next: Next) -> Response {
    // 模拟延迟 3 秒
    sleep(Duration::from_secs(3)).await;

    // 继续处理请求
    next.run(req).await
}