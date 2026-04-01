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

#[derive(Deserialize)]
pub struct UserCreateForm {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn create(
    _ctx: Context,
    store: Store,
    _state: State<AppState>,
    Json(form): Json<UserCreateForm>,
) -> impl IntoResponse {
    let cfg = store.get_register_config().await;

    // 验证邮箱不能为空
    if form.email.trim().is_empty() {
        return Data::fail("邮箱不能为空").into_response();
    }

    if !crate::home::auth::is_valid_username(form.username.as_str()) {
        return Data::fail("用户名格式错误，只能包含字母和数字").into_response();
    }
    
    if cfg.min_username > form.username.len() as i64 {
        return Data::fail(&format!("用户名长度不能小于 {} 位", cfg.min_username)).into_response();
    }
    
    if cfg.max_username < form.username.len() as i64 {
        return Data::fail(&format!("用户名长度不能超过 {} 位", cfg.max_username)).into_response();
    }

    if form.password.len() < cfg.min_password_len {
        return Data::fail(&format!("密码长度不能小于 {} 位", cfg.min_password_len)).into_response();
    }

    let email_exists = match store.check_email_exists(&form.email).await {
        Ok(exists) => exists,
        Err(e) => return Data::fail(&format!("系统错误: {}", e)).into_response(),
    };
    if email_exists {
        return Data::fail("该邮箱已被注册").into_response();
    }

    let username_exists = match store.check_username_exists(&form.username).await {
        Ok(exists) => exists,
        Err(e) => return Data::fail(&format!("系统错误: {}", e)).into_response(),
    };
    if username_exists {
        return Data::fail("该用户名已被注册").into_response();
    }

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(_) => return Data::fail("密码加密失败").into_response(),
    };

    // 使用默认语言和分数创建用户
    let language = "en"; // 或者是其他的默认语言配置，如果能从 store/context 拿最好，这里简单起见直接写死或者用空字符串
    let result = store
        .create_user_with_invite(
            &form.username,
            &password_hash,
            &form.email,
            language,
            cfg.initial_score,
            None,
        )
        .await;

    match result {
        Ok(_) => Data::done().into_response(),
        Err(e) => Data::fail(&format!("创建用户失败: {}", e)).into_response(),
    }
}
