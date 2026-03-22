use crate::common::AppState;
use crate::moderator::{Data, ModContext as Context, PageExtractor};
use crate::store::Store;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct GenerateParams {
    count: usize,
    quota: i32,
    expired_at: Option<DateTime<Utc>>,
}

#[axum::debug_handler(state = AppState)]
pub async fn list(
    _ctx: Context,
    store: Store,
    PageExtractor(page): PageExtractor,
) -> impl IntoResponse {
    match store.get_invite_codes(page).await {
        Ok(data) => Data::ok(data).into_response(),
        Err(e) => Data::<()>::error(&e.to_string()).into_response(),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn generate(
    _ctx: Context,
    store: Store,
    Json(params): Json<GenerateParams>,
) -> impl IntoResponse {
    match store.create_invite_codes(params.count, params.quota, params.expired_at).await {
        Ok(codes) => Data::ok(codes).into_response(),
        Err(e) => Data::<()>::error(&e.to_string()).into_response(),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn delete(
    _ctx: Context,
    store: Store,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match store.delete_invite_code(id).await {
        Ok(_) => Data::done().into_response(),
        Err(e) => Data::<()>::error(&e.to_string()).into_response(),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn logs(
    _ctx: Context,
    store: Store,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match store.get_invite_usage_logs(id).await {
        Ok(logs) => Data::ok(logs).into_response(),
        Err(e) => Data::<()>::error(&e.to_string()).into_response(),
    }
}
