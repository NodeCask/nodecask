use crate::common::AppState;
use crate::home::auth::hash_password;
use crate::moderator::{Data, ModContext as Context, PageExtractor};
use crate::store::Store;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UserSearch {
    pub username: Option<String>,
    pub active: Option<bool>,
    pub role: Option<String>,
}
#[axum::debug_handler(state = AppState)]
pub async fn list(
    _ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    PageExtractor(page): PageExtractor,
    Query(search): Query<UserSearch>,
) -> impl IntoResponse {
    match store
        .search_users(search.username, search.active, search.role, page)
        .await
    {
        Ok(users) => Data::ok(users).into_response(),
        Err(err) => Data::fail(&format!("查询失败: {}", err)).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UserStatusForm {
    pub action: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn update_status(
    _ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    Path(id): Path<i64>,
    Json(form): Json<UserStatusForm>,
) -> impl IntoResponse {
    if id == 1 {
        return Data::fail("禁止修改超级管理员账户状态").into_response();
    }
    let active = match form.action.as_str() {
        "enable" => true,
        "disable" => false,
        _ => return Data::fail("无效的操作").into_response(),
    };

    let result = store.update_user_status(id, active).await;

    match result {
        Ok(rows) => {
            if rows == 0 {
                Data::fail("用户未找到").into_response()
            } else {
                Data::done().into_response()
            }
        }
        Err(e) => Data::fail(&format!("更新失败: {}", e)).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UserRoleForm {
    pub role: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn update_role(
    _ctx: Context,
    store: Store,
    _state: State<AppState>,
    Path(id): Path<i64>,
    Json(form): Json<UserRoleForm>,
) -> impl IntoResponse {
    if id == 1 {
        return Data::fail("禁止修改超级管理员账户状态").into_response();
    }
    let valid_roles = vec!["user", "moderator", "administrator"];
    if !valid_roles.contains(&form.role.as_str()) {
        return Data::fail("无效的角色").into_response();
    }

    match store.update_user_role(id, &form.role).await {
        Ok(_) => Data::done().into_response(),
        Err(e) => Data::fail(&format!("更新角色失败: {}", e)).into_response(),
    }
}

#[derive(Deserialize)]
pub struct UserPasswordResetForm {
    pub password: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn reset_password(
    _ctx: Context,
    store: Store,
    _state: State<AppState>,
    Path(id): Path<i64>,
    Json(form): Json<UserPasswordResetForm>,
) -> impl IntoResponse {
    if id == 1 {
        return Data::fail("禁止修改超级管理员密码").into_response();
    }

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(e) => return Data::fail(&format!("密码加密失败: {}", e)).into_response(),
    };

    match store.user_password_reset(id, &password_hash).await {
        Ok(_) => Data::done().into_response(),
        Err(e) => Data::fail(&format!("重置密码失败: {}", e)).into_response(),
    }
}
