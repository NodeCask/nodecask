use crate::home::auth::verify_password;
use crate::home::{PasswordChallengeCounter};
use crate::moderator::Data;
use crate::store::Store;
use axum::response::IntoResponse;
use axum::Json;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};
use crate::common::AppState;
use crate::store::user::User;

#[derive(Deserialize)]
pub struct AccessTokenRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct AccessTokenResponse {
    token: String,
    totp: bool,
    message: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn get_access_token(
    store: Store,
    Json(form): Json<AccessTokenRequest>,
) -> impl IntoResponse {
    let user = match store.get_user(&form.username).await {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Data::<AccessTokenResponse>::error("用户名或密码错误");
        }
        Err(err) => {
            return Data::<AccessTokenResponse>::error(&err.to_string());
        }
    };

    // Check rate limit
    let current_count = match check_login_limit(&store, user.id).await {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Verify password
    if !verify_password(&form.password, &user.password_hash).unwrap_or(false) {
        record_login_failure(&store, user.id, current_count).await;
        return Data::<AccessTokenResponse>::error("用户名或密码错误");
    }

    // Check permissions
    if let Err(err) = check_user_permissions(&user) {
        return err;
    }

    // Check if TOTP is enabled
    match store.get_totp_secret(user.id).await {
        Ok(None) => {}
        Ok(Some(str)) if str.is_empty() => {}
        Ok(Some(_)) => {
            let token = match store.get_token(user.id, "admin-totp").await {
                Ok(token) => token,
                Err(err) => return Data::<AccessTokenResponse>::error(&format!("系统错误: {}", err)),
            };
            return Data::ok(AccessTokenResponse {
                token,
                totp: true,
                message: "请执行 TOTP 验证".to_string(),
            });
        }
        Err(err) => return Data::<AccessTokenResponse>::error(&format!("系统错误: {}", err)),
    }

    // Clear failure counter and generate token
    finalize_login(store, user).await
}

#[derive(Deserialize)]
pub struct TotpChallengeRequest {
    username: String,
    code: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn totp_challenge(
    store: Store,
    Json(form): Json<TotpChallengeRequest>,
) -> impl IntoResponse {
    if form.username.is_empty() || form.code.is_empty() {
        return Data::<AccessTokenResponse>::error("验证失败");
    }
    let user = match store.get_user(&form.username).await {
        Ok(None) => {
            return Data::<AccessTokenResponse>::error("验证失败");
        }
        Ok(Some(user)) => user,
        Err(err) => {
            return Data::<AccessTokenResponse>::error(&format!("系统错误: {}", err));
        }
    };

    // Check rate limit
    let current_count = match check_login_limit(&store, user.id).await {
        Ok(count) => count,
        Err(resp) => return resp,
    };

    // Check permissions
    if let Err(err) = check_user_permissions(&user) {
        return err;
    }

    let secret_str = match store.get_totp_secret(user.id).await {
        Ok(None) => {
            return Data::<AccessTokenResponse>::error("验证失败");
        }
        Ok(Some(s)) => s,
        Err(err) => {
            return Data::<AccessTokenResponse>::error(&format!("系统错误: {}", err));
        }
    };

    let secret_bytes = match Secret::Encoded(secret_str).to_bytes() {
        Ok(b) => b,
        Err(_) => return Data::<AccessTokenResponse>::error("系统错误: Invalid Secret"),
    };

    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        None,
        user.email.clone(),
    ) {
        Ok(t) => t,
        Err(e) => return Data::<AccessTokenResponse>::error(&format!("系统错误: {}", e)),
    };

    match totp.check_current(&form.code) {
        Ok(true) => {}
        Ok(false) => {
            record_login_failure(&store, user.id, current_count).await;
            return Data::<AccessTokenResponse>::error("验证失败");
        }
        Err(err) => {
            return Data::<AccessTokenResponse>::error(&format!("系统错误: {}", err));
        }
    }

    finalize_login(store, user).await
}

// --- Helper Functions ---
async fn finalize_login(store: Store, user: User) -> Json<Data<AccessTokenResponse>> {
    if let Err(err) = store
        .set_user_attr::<PasswordChallengeCounter>(user.id, "login-challenge-counter", None)
        .await
    {
        warn!("Failed to clear user login failure counter: {}", err)
    }

    match store
        .create_access_token(user.id, "API Access Token", "admin")
        .await
    {
        Ok(token) => Data::ok(AccessTokenResponse {
            token,
            totp: false,
            message: "Token 创建成功，有效期 90 天".to_string(),
        }),
        Err(err) => {
            error!("Failed to create access token: {}", err);
            Data::error("服务器内部错误")
        }
    }
}

fn check_user_permissions(user: &User) -> Result<(), Json<Data<AccessTokenResponse>>> {
    // Check if user is active
    if !user.active {
        return Err(Data::error("用户已被禁止登录"));
    }

    // Check if user is administrator
    if user.role != "administrator" {
        return Err(Data::error("只有管理员才能访问此接口"));
    }
    Ok(())
}

async fn check_login_limit(
    store: &Store,
    user_id: i64,
) -> Result<i64, Json<Data<AccessTokenResponse>>> {
    match store
        .get_user_attr::<PasswordChallengeCounter>(user_id, "login-challenge-counter")
        .await
    {
        Ok(Some(PasswordChallengeCounter { count, expires })) => {
            if count >= 5 && expires > chrono::Utc::now() {
                return Err(Data::error("密码错误次数过多，请稍后再尝试"));
            }
            if expires > chrono::Utc::now() {
                Ok(count)
            } else {
                Ok(0)
            }
        }
        Ok(None) => Ok(0),
        Err(err) => Err(Data::error(&err.to_string())),
    }
}

async fn record_login_failure(store: &Store, user_id: i64, current_count: i64) {
    let c = PasswordChallengeCounter {
        count: current_count + 1,
        expires: chrono::Utc::now() + chrono::Duration::hours(1),
    };
    if let Err(err) = store
        .set_user_attr(user_id, "login-challenge-counter", Some(&c))
        .await
    {
        warn!("Failed to update user login failure counter: {}", err)
    }
}