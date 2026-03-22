use crate::common::{Context, CurrentUser, GlobalContext, OptionalCurrentUser, WebResponse};
use crate::daemon::email::{EmailSenderDaemon, Mail};
use crate::daemon::nsfw_detect::NSFWDetector;
use crate::home::auth::{hash_password, verify_password};
use crate::home::{AppState, RedirectTemplate};
use crate::store::notifications::{Notification, NotificationSearch};
use crate::store::topic::{TopicIndex, TopicSearch, UserCommentDisplay};
use crate::store::user::{User, UserDetail};
use crate::store::{Page, Pagination, Store};
use crate::{t, t_owned};
use anyhow::anyhow;
use askama::Template;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::{Multipart, Query};
use axum::extract::{Path, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::response::Response;
use axum::response::{IntoResponse, Redirect};
use axum::Form;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use base64::engine::general_purpose;
use base64::Engine;
use into_response_derive::TemplateResponse;
use log::{error, info, warn};
use regex::Regex;
use serde::Deserialize;
use sqlx::types::time::OffsetDateTime;
use std::io::Cursor;
use std::ops::Add;
use std::sync::OnceLock;
use tantivy::time::Duration;
use totp_rs::{Algorithm, Secret, TOTP};
use tower_sessions::Session;

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/info.html")]
struct UserInfoTemplate {
    ctx: crate::common::VContext,
    profile_user: User,
    topics: Vec<TopicIndex>,
    relation: Option<String>,
    is_self: bool,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/settings/avatar.html")]
struct AvatarUploadTemplate {
    tips: String,
    username: String,
    timestamp:i64,
    ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/center.html")]
struct UserCenterTemplate {
    topics: Vec<TopicIndex>,
    user: UserDetail,
    checked_in_today: bool,
    ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/settings/password.html")]
struct ChangePasswordTemplate {
    tips: String,
    ctx: crate::common::VContext,
}

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    pub old_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/notifications.html")]
struct NotificationsTemplate {
    ctx: crate::common::VContext,
    notifications: Pagination<Notification>,
}
#[axum::debug_handler(state = AppState)]
pub async fn info(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    Path(username): Path<String>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(profile_user) = store.get_user(username.as_str()).await.map_err(ctx.err())? else {
        return Ok((
            StatusCode::NOT_FOUND,
            ctx.error(t!(ctx, "user.user_not_found")),
        )
            .into_response());
    };
    let topics = store
        .topics(
            &TopicSearch {
                user_id: Some(profile_user.id),
                viewer_id: None,
                sort: Some("latest".to_string()),
                ..Default::default()
            },
            1u32.into(),
        )
        .await
        .map_err(ctx.err())?;

    let relation = if let Some(u) = &user {
        store
            .get_relation(u.id, profile_user.id)
            .await
            .unwrap_or_default()
    } else {
        None
    };

    let is_self = user
        .as_ref()
        .map(|u| u.id == profile_user.id)
        .unwrap_or(false);

    let t = UserInfoTemplate {
        ctx: ctx.context(profile_user.username.clone()),
        profile_user,
        topics: topics.data,
        relation,
        is_self,
    };
    t.into()
}

#[derive(Deserialize)]
pub struct UserCenterUpdateForm {
    pub bio: Option<String>,
    pub address: Option<String>,
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub public_email: Option<String>,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/settings/profile.html")]
struct ProfileSettingsTemplate {
    ctx: crate::common::VContext,
    user: User,
    tips: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn profile_form(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(current_user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let Some(user) = store
        .get_user(&current_user.username)
        .await
        .map_err(ctx.err())?
    else {
        return ctx.error(t!(ctx, "user.user_not_found")).into();
    };

    let t = ProfileSettingsTemplate {
        ctx: ctx.context(t_owned!(ctx, "user.profile_settings")),
        user,
        tips: "".to_string(),
    };
    t.into()
}
#[axum::debug_handler(state = AppState)]
pub async fn profile_post(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    jar: CookieJar,
    user: OptionalCurrentUser,
    Form(form): Form<UserCenterUpdateForm>,
) -> WebResponse {
    let Some(user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let public_email = form.public_email.is_some();

    let language = form.language.clone().unwrap_or("en_US".to_string());
    match store
        .update_user_profile(
            user.id,
            form.bio.unwrap_or_default(),
            form.address.unwrap_or_default(),
            form.timezone.unwrap_or("UTC".to_string()),
            form.language.unwrap_or("en_US".to_string()),
            public_email,
        )
        .await
    {
        Ok(_) => {
            // 用户更新设定之后，需要重新更新当前 session，让时区、语言偏好这些生效
            if let Some(neo_user) = store.get_user(&user.username).await.map_err(ctx.err())? {
                ctx.set("CurrentUser", CurrentUser::from(&neo_user)).await;
            }
            let e: OffsetDateTime = OffsetDateTime::now_utc() + Duration::days(30);
            Ok((
                jar.add(Cookie::build(("language", language)).path("/").expires(e)),
                RedirectTemplate {
                    tips: &t!(ctx, "user.update_success"),
                    url: "/user/center",
                    ctx: ctx.context(t_owned!(ctx, "user.update_success")),
                },
            )
                .into_response())
        }
        Err(e) => {
            error!("Error updating user profile: {}", e);
            ctx.error(t!(ctx, "user.update_profile_error")).into()
        }
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn center(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };
    let topics = store
        .topics(
            &TopicSearch {
                user_id: Some(user.id),
                viewer_id: Some(user.id),
                sort: Some("latest".to_string()),
                ..Default::default()
            },
            1u32.into(),
        )
        .await
        .map_err(ctx.err())?;
    let user_detail = store.get_user_detail(user.id).await?;

    let today = chrono::Local::now().date_naive();
    let status = store.get_user_access_stats(user.id).await.unwrap_or(None);
    let checked_in_today = status
        .map(|s| s.last_checkin_date == today)
        .unwrap_or(false);

    let t = UserCenterTemplate {
        topics: topics.data,
        user: user_detail,
        checked_in_today,
        ctx: ctx.context(t_owned!(ctx, "user.user_center")),
    };
    t.into()
}

#[axum::debug_handler(state = AppState)]
pub async fn change_password_form(ctx: Context, user: OptionalCurrentUser) -> WebResponse {
    if user.is_none() {
        return Ok(Redirect::to("/login").into_response());
    }

    let t = ChangePasswordTemplate {
        tips: "".to_string(),
        ctx: ctx.context(t_owned!(ctx, "user.change_password")),
    };
    t.into()
}

#[axum::debug_handler(state = AppState)]
pub async fn change_password_post(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    user: OptionalCurrentUser,
    Form(form): Form<ChangePasswordForm>,
) -> WebResponse {
    let Some(current_user_display) = user else {
        return Ok(Redirect::to("/login").into_response());
    };

    if form.new_password != form.confirm_password {
        return ChangePasswordTemplate {
            tips: t_owned!(ctx, "user.passwords_mismatch"),
            ctx: ctx.context(t_owned!(ctx, "user.change_password")),
        }
        .into();
    }

    // Fetch full user to get the password hash
    let user_option = store
        .get_user(&current_user_display.username)
        .await
        .map_err(ctx.err())?;
    let Some(user) = user_option else {
        return ChangePasswordTemplate {
            tips: t_owned!(ctx, "user.user_not_found"),
            ctx: ctx.context(t_owned!(ctx, "user.change_password")),
        }
        .into();
    };

    if !verify_password(&form.old_password, &user.password_hash).unwrap_or(false) {
        return ChangePasswordTemplate {
            tips: t_owned!(ctx, "user.current_password_wrong"),
            ctx: ctx.context(t_owned!(ctx, "user.change_password")),
        }
        .into();
    }

    let password_hash = match hash_password(&form.new_password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Error hashing password: {}", e);
            return ChangePasswordTemplate {
                tips: t_owned!(ctx, "user.system_error"),
                ctx: ctx.context(t_owned!(ctx, "user.change_password")),
            }
            .into();
        }
    };

    match store.user_password_reset(user.id, &password_hash).await {
        Ok(_) => RedirectTemplate {
            tips: &t!(ctx, "user.password_changed"),
            url: "/user/center",
            ctx: ctx.context(t_owned!(ctx, "user.password_changed")),
        }
        .into(),
        Err(e) => {
            error!("Error resetting password: {}", e);
            ChangePasswordTemplate {
                tips: t_owned!(ctx, "user.password_change_failed"),
                ctx: ctx.context(t_owned!(ctx, "user.change_password")),
            }
            .into()
        }
    }
}

const AVATAR_PNG: &'static [u8] = include_bytes!("../../assets/avatar.png");

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AvatarTimestampQuery {
    t:Option<i64>
}
pub async fn avatar(
    Path(username): Path<String>,
    State(_state): State<AppState>,
    store: Store,
    Query(query): Query<AvatarTimestampQuery>,
) -> Response {
    let result = store.get_user_avatar_data(&username).await;
    let max_age = if query.t.unwrap_or_default() > 0 { 86400 } else { 60 };
    let cache_control = format!("public, max-age={}", max_age);

    if let Ok(Some(data)) = result {
        return (
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, cache_control.as_str()),
            ],
            Body::from(data),
        )
            .into_response();
    }
    // 尝试从数据库加载默认头像
    #[derive(Deserialize)]
    struct AvatarConfig {
        initial_avatar: String,
    }
    if let Some(cfg) = store.get_cfg::<AvatarConfig>("register_config").await
        && !cfg.initial_avatar.is_empty()
    {
        if let Ok((mime, data)) = get_image_from_data_url(&cfg.initial_avatar) {
            return (
                [
                    (header::CONTENT_TYPE, mime.as_str()),
                    (header::CACHE_CONTROL, cache_control.as_str()),
                ],
                Body::from(data),
            )
                .into_response();
        }
    }

    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, cache_control.as_str()),
        ],
        Body::from(AVATAR_PNG),
    )
        .into_response()
}
static DATA_URL_REGEX: OnceLock<Regex> = OnceLock::new();
fn get_image_from_data_url(data_url: &str) -> anyhow::Result<(String, Vec<u8>)> {
    let re = DATA_URL_REGEX.get_or_init(|| Regex::new(r"^data:image/(\w+);base64,(.+)$").unwrap());

    if let Some(caps) = re.captures(data_url) {
        let extension = &caps[1];
        let payload = &caps[2];
        let image_bytes = general_purpose::STANDARD.decode(payload)?;
        return Ok((format!("image/{}", extension), image_bytes));
    }
    Err(anyhow!("无效的 Data URL"))
}

#[axum::debug_handler(state = AppState)]
pub async fn avatar_upload_form(ctx: Context, user: OptionalCurrentUser) -> WebResponse {
    let Some(user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let t = AvatarUploadTemplate {
        tips: "".to_string(),
        username: user.username,
        timestamp: chrono::Utc::now().timestamp(),
        ctx: ctx.context(t_owned!(ctx, "user.upload_avatar")),
    };
    t.into()
}

#[axum::debug_handler(state = AppState)]
pub async fn avatar_upload_post(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    GlobalContext(detector): GlobalContext<NSFWDetector>,
    user: OptionalCurrentUser,
    mut multipart: Multipart,
) -> WebResponse {
    let Some(user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };

    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name() == Some("avatar") {
            let data: Bytes = field.bytes().await.unwrap_or_default();

            if data.len() > 1024 * 1024 * 2 {
                let t = AvatarUploadTemplate {
                    tips: t_owned!(ctx, "user.avatar_size_limit"),
                    username: user.username,
                    timestamp: chrono::Utc::now().timestamp(),
                    ctx: ctx.context("上传头像".to_string()),
                };
                return t.into();
            }

            if data.is_empty() {
                let t = AvatarUploadTemplate {
                    tips: t_owned!(ctx, "user.select_file"),
                    username: user.username,
                    timestamp: chrono::Utc::now().timestamp(),
                    ctx: ctx.context("上传头像".to_string()),
                };
                return t.into();
            }

            // Resize and convert to PNG
            let img = match image::load_from_memory(&data) {
                Ok(img) => img,
                Err(e) => {
                    error!("Load image error: {}", e);
                    let t = AvatarUploadTemplate {
                        tips: t_owned!(ctx, "user.invalid_image"),
                        username: user.username,
                        timestamp: chrono::Utc::now().timestamp(),
                        ctx: ctx.context("上传头像".to_string()),
                    };
                    return t.into();
                }
            };
            match detector.detect(img.clone()).await {
                Ok(label) => {
                    if label.is_hentai() || label.is_porn() {
                        let t = AvatarUploadTemplate {
                            tips: t_owned!(ctx, "user.nsfw_image"),
                            username: user.username,
                            timestamp: chrono::Utc::now().timestamp(),
                            ctx: ctx.context("上传头像".to_string()),
                        };
                        return t.into();
                    }
                }
                Err(err) => {
                    let t = AvatarUploadTemplate {
                        tips: format!("{}: {}", t!(ctx, "user.image_detect_error"), err),
                        username: user.username,
                        timestamp: chrono::Utc::now().timestamp(),
                        ctx: ctx.context("上传头像".to_string()),
                    };
                    return t.into();
                }
            }

            let resized = img.resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
            let mut buffer = Cursor::new(Vec::new());
            if let Err(e) = resized.write_to(&mut buffer, image::ImageFormat::Png) {
                error!("Write image error: {}", e);
                let t = AvatarUploadTemplate {
                    tips: t_owned!(ctx, "user.image_process_error"),
                    username: user.username,
                    timestamp: chrono::Utc::now().timestamp(),
                    ctx: ctx.context("上传头像".to_string()),
                };
                return t.into();
            }
            let data = buffer.into_inner();

            let result = store.update_user_avatar(user.id, data).await;

            return match result {
                Ok(_) => {
                    let neo_user = CurrentUser {
                        avatar_timestamp: chrono::Utc::now().timestamp(),
                        ..user
                    };
                    ctx.set("CurrentUser", neo_user).await;
                    return ctx
                        .success(t!(ctx, "user.avatar_updated").as_ref())
                        .with_return_url("/user/center")
                        .into();
                }
                Err(e) => {
                    error!("Upload avatar error: {}", e);
                    let t = AvatarUploadTemplate {
                        tips: t_owned!(ctx, "user.upload_failed"),
                        username: user.username,
                        timestamp: chrono::Utc::now().timestamp(),
                        ctx: ctx.context("上传头像".to_string()),
                    };
                    t.into()
                }
            };
        }
    }

    let t = AvatarUploadTemplate {
        tips: t_owned!(ctx, "user.file_not_found"),
        username: user.username,
        timestamp: chrono::Utc::now().timestamp(),
        ctx: ctx.context("上传头像".to_string()),
    };
    t.into()
}

#[axum::debug_handler(state = AppState)]
pub async fn follow(
    ctx: Context,
    store: Store,
    Path(username): Path<String>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(current_user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };
    let Some(target) = store.get_user(&username).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "user.user_not_found")).into();
    };
    if let Err(e) = store.follow_user(current_user.id, target.id).await {
        error!("Follow user error: {}", e);
        return ctx.error(t!(ctx, "user.operation_failed")).into();
    }
    Ok(Redirect::to(&format!("/u/{}", username)).into_response())
}

#[axum::debug_handler(state = AppState)]
pub async fn unfollow(
    ctx: Context,
    store: Store,
    Path(username): Path<String>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(current_user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };
    let Some(target) = store.get_user(&username).await.map_err(ctx.err())? else {
        return ctx.error("User not found").into();
    };
    if let Err(e) = store.unfollow_user(current_user.id, target.id).await {
        error!("Unfollow user error: {}", e);
        return ctx.error("Operation failed").into();
    }
    Ok(Redirect::to(&format!("/u/{}", username)).into_response())
}

#[axum::debug_handler(state = AppState)]
pub async fn block(
    ctx: Context,
    store: Store,
    Path(username): Path<String>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(current_user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };
    let Some(target) = store.get_user(&username).await.map_err(ctx.err())? else {
        return ctx.error("User not found").into();
    };
    if let Err(e) = store.block_user(current_user.id, target.id).await {
        error!("Block user error: {}", e);
        return ctx.error("Operation failed").into();
    }
    Ok(Redirect::to(&format!("/u/{}", username)).into_response())
}

#[axum::debug_handler(state = AppState)]
pub async fn unblock(
    ctx: Context,
    store: Store,
    Path(username): Path<String>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(current_user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };
    let Some(target) = store.get_user(&username).await.map_err(ctx.err())? else {
        return ctx.error("User not found").into();
    };
    if let Err(e) = store.unblock_user(current_user.id, target.id).await {
        error!("Unblock user error: {}", e);
        return ctx.error("Operation failed").into();
    }
    Ok(Redirect::to(&format!("/u/{}", username)).into_response())
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/topics.html")]
struct UserTopicsTemplate {
    username: String,
    current_user: OptionalCurrentUser,
    articles: Pagination<TopicIndex>,
    ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/comments.html")]
struct UserCommentsTemplate {
    username: String,
    current_user: OptionalCurrentUser,
    comments: Pagination<UserCommentDisplay>,
    ctx: crate::common::VContext,
}

#[axum::debug_handler(state = AppState)]
pub async fn topics(
    ctx: Context,
    store: Store,
    Path(username): Path<String>,
    page: Page,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(target_user) = store.get_user(&username).await.map_err(ctx.err())? else {
        return ctx.error("User not found").into();
    };

    let search = TopicSearch {
        user_id: Some(target_user.id),
        viewer_id: None,
        sort: Some("latest".to_string()),
        ..Default::default()
    };

    match store.topics(&search, page.topic()).await {
        Ok(articles) => {
            let t = UserTopicsTemplate {
                current_user: user,
                username: target_user.username.clone(),
                articles,
                ctx: ctx.context(target_user.username),
            };
            t.into()
        }
        Err(err) => ctx.error(&format!("failed to get topics: {}", err)).into(),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn comments(
    ctx: Context,
    store: Store,
    Path(username): Path<String>,
    page: Page,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(target_user) = store.get_user(&username).await.map_err(ctx.err())? else {
        return ctx.error("User not found").into();
    };

    match store.user_comments(target_user.id, page.list()).await {
        Ok(comments) => {
            let t = UserCommentsTemplate {
                current_user: user,
                username: target_user.username.clone(),
                comments,
                ctx: ctx.context(format!(
                    "{} {}",
                    target_user.username,
                    t!(ctx, "user.recent_comments")
                )),
            };
            t.into()
        }
        Err(err) => ctx
            .error(&format!("failed to get comments: {}", err))
            .into(),
    }
}
#[axum::debug_handler(state = AppState)]
pub async fn notifications(
    ctx: Context,
    store: Store,
    user: CurrentUser,
    session: Session,
    page: Page,
) -> WebResponse {
    // 首先标记所有消息为已读，从 ctx 里面读取未读消息
    if ctx
        .current_user
        .as_ref()
        .map(|u| u.unread_notifications)
        .unwrap_or_default()
        > 0
    {
        if let Err(e) = store.mark_all_as_read(user.id).await {
            log::error!("Failed to mark notifications as read: {}", e);
            return ctx.error(t!(ctx, "user.update_unread_error")).into();
        }
        let neo = CurrentUser {
            unread_notifications: 0,
            ..user.clone()
        };
        if let Err(e) = session.insert("CurrentUser", &neo).await {
            log::error!("failed to update session: {}", e);
            return ctx.internal_error().into();
        }
    }

    // 获取用户的消息
    let search = NotificationSearch {
        user_id: user.id,
        category: None,
    };

    match store.get_notifications(&search, page.list()).await {
        Ok(pagination) => {
            // 渲染消息页面
            let template = NotificationsTemplate {
                ctx: ctx.context(t_owned!(ctx, "user.notifications")),
                notifications: pagination,
            };
            template.into()
        }
        Err(e) => {
            log::error!("Failed to get notifications: {}", e);
            ctx.error(t!(ctx, "user.get_notifications_error")).into()
        }
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn notifications_clear(ctx: Context, store: Store, user: CurrentUser) -> WebResponse {
    if let Err(e) = store.delete_all_notifications(user.id).await {
        log::error!("Failed to clear notifications: {}", e);
        return ctx.error(t!(ctx, "user.clear_notifications_error")).into();
    }
    RedirectTemplate {
        tips: &t!(ctx, "user.notifications_cleared"),
        url: "/notifications",
        ctx: ctx.context(t_owned!(ctx, "user.notifications_cleared")),
    }
    .into()
}

#[derive(Template)]
#[template(path = "email/verification.html")]
pub struct EmailVerificationTemplate {
    pub code: String,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/settings/email.html")]
struct ChangeEmailTemplate {
    tips: String,
    email: String,
    ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/settings/email_verify.html")]
struct ChangeEmailVerifyTemplate {
    tips: String,
    email: String,
    ctx: crate::common::VContext,
}

#[derive(Deserialize)]
pub struct ChangeEmailForm {
    pub new_email: String,
}

#[derive(Deserialize)]
pub struct ChangeEmailVerifyForm {
    pub email: String,
    pub code: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn change_email_form(ctx: Context, user: OptionalCurrentUser) -> WebResponse {
    if user.is_none() {
        return Ok(Redirect::to("/login").into_response());
    }
    let t = ChangeEmailTemplate {
        tips: "".to_string(),
        email: "".to_string(),
        ctx: ctx.context(t_owned!(ctx, "user.change_email")),
    };
    t.into()
}

#[axum::debug_handler(state = AppState)]
pub async fn change_email_post(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    user: CurrentUser,
    GlobalContext(sender): GlobalContext<EmailSenderDaemon>,
    Form(form): Form<ChangeEmailForm>,
) -> WebResponse {
    if form.new_email.trim().is_empty() {
        return ChangeEmailTemplate {
            tips: t_owned!(ctx, "user.email_required"),
            email: form.new_email,
            ctx: ctx.context("修改邮箱".to_string()),
        }
        .into();
    }

    let code = store
        .get_token(user.id, format!("new_email#{}", form.new_email).as_str())
        .await
        .map_err(ctx.err())?;

    let Ok(email_body) = EmailVerificationTemplate { code }.render() else {
        return ctx.internal_error().into();
    };
    let result = store
        .add_email_queue(
            user.id,
            "",
            &form.new_email,
            &t!(ctx, "user.email_change_reminder"),
            email_body.as_str(),
        )
        .await;
    match result {
        Ok(id) => {
            sender.send(Mail(id)).await;
        }
        Err(err) => {
            return ctx.error(err.to_string().as_str()).into();
        }
    }

    let t = ChangeEmailVerifyTemplate {
        tips: "".to_string(),
        email: form.new_email,
        ctx: ctx.context(t_owned!(ctx, "user.verify_new_email")),
    };
    t.into()
}

#[axum::debug_handler(state = AppState)]
pub async fn change_email_verify_post(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    user: CurrentUser,
    Form(form): Form<ChangeEmailVerifyForm>,
) -> WebResponse {
    if form.code.trim().is_empty() {
        return ChangeEmailVerifyTemplate {
            tips: t_owned!(ctx, "user.code_required"),
            email: form.email,
            ctx: ctx.context("验证新邮箱".to_string()),
        }
        .into();
    }

    info!(
        "Email verification: {} - {}",
        format!("new_email#{}", &form.email).as_str(),
        &form.code
    );
    let result = store
        .verify_token(
            user.id,
            format!("new_email#{}", &form.email).as_str(),
            &form.code,
        )
        .await;
    info!("Verification result: {:?}", &result);
    match result {
        Ok(true) => {}
        Ok(false) => {
            return ChangeEmailVerifyTemplate {
                tips: t_owned!(ctx, "user.code_invalid_or_expired"),
                email: form.email,
                ctx: ctx.context("验证新邮箱".to_string()),
            }
            .into();
        }
        Err(err) => {
            return ctx.error(err.to_string().as_str()).into();
        }
    }

    if let Err(e) = store.update_user_email(user.id, &form.email).await {
        error!("Update email error: {}", e);
        return ChangeEmailVerifyTemplate {
            tips: t_owned!(ctx, "user.update_failed"),
            email: form.email,
            ctx: ctx.context("验证新邮箱".to_string()),
        }
        .into();
    }
    RedirectTemplate {
        tips: &t!(ctx, "user.email_changed"),
        url: "/user/center",
        ctx: ctx.context(t_owned!(ctx, "user.email_changed")),
    }
    .into()
}

#[axum::debug_handler(state = AppState)]
pub async fn checkin(
    ctx: Context,
    store: Store,
    State(state): State<AppState>,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let today = chrono::Local::now().date_naive();

    // Check status
    let Some(status) = store
        .get_user_access_stats(user.id)
        .await
        .map_err(ctx.err())?
    else {
        return ctx.error(t!(ctx, "user.member_info_not_found")).into();
    };

    if status.last_checkin_date == today {
        return ctx.error(t!(ctx, "user.already_checked_in")).into();
    }
    let continuous_checkin_days = if status.last_checkin_date.add(chrono::Days::new(1)) == today {
        status.continuous_checkin_days + 1
    } else {
        1
    };
    if let Err(err) = store
        .update_user_check_stats(
            user.id,
            today,
            status.checkin_days + 1,
            continuous_checkin_days,
        )
        .await
    {
        return ctx
            .error(&format!(
                "{}: {}",
                t!(ctx, "user.update_checkin_error"),
                err
            ))
            .into();
    }
    match state.login_rewards.checkin(user.id).await {
        Ok(rewards) => {
            let msg = t_owned!(
                ctx,
                "user.checkin_success",
                coins = rewards.coins,
                days = continuous_checkin_days
            );
            ctx.success(&msg).with_return_url("/user/center").into()
        }
        Err(err) => ctx
            .error(&format!("{}: {}", t!(ctx, "user.update_reward_error"), err))
            .into(),
    }
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/user/settings/totp.html")]
struct TotpSettingsTemplate {
    tips: String,
    ctx: crate::common::VContext,
    is_bound: bool,
    qr_code: Option<String>,
    secret: Option<String>,
}

#[derive(Deserialize)]
pub struct TotpSettingsForm {
    pub code: String,
    pub secret: Option<String>,
    pub action: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn totp_settings_page(
    ctx: Context,
    store: Store,
    user: OptionalCurrentUser,
) -> WebResponse {
    let Some(current_user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let totp_secret = store
        .get_totp_secret(current_user.id)
        .await
        .map_err(ctx.err())?
        .filter(|s| !s.is_empty());
    let Some(user) = store
        .get_user(&current_user.username)
        .await
        .map_err(ctx.err())?
    else {
        return ctx.error("系统异常").into();
    };
    let is_bound = totp_secret.is_some();

    let (qr_code, secret) = if !is_bound {
        let secret = Secret::generate_secret();
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret.to_bytes().map_err(ctx.err())?,
            Some(ctx.website.nickname.clone()),
            user.email.clone(),
        )
            .map_err(ctx.err())?;
        let qr = totp.get_qr_base64().unwrap_or_default();
        (Some(qr), Some(secret.to_encoded().to_string()))
    } else {
        (None, None)
    };

    TotpSettingsTemplate {
        tips: "".to_string(),
        ctx: ctx.context(t_owned!(ctx, "user.two_factor")),
        is_bound,
        qr_code,
        secret,
    }
    .into()
}

#[axum::debug_handler(state = AppState)]
pub async fn totp_settings_post(
    ctx: Context,
    store: Store,
    user: OptionalCurrentUser,
    Form(form): Form<TotpSettingsForm>,
) -> WebResponse {
    let Some(current_user) = user else {
        return Ok(Redirect::to("/login").into_response());
    };

    let user = store
        .get_user(&current_user.username)
        .await
        .map_err(ctx.err())?
        .unwrap();
    let totp_secret = store
        .get_totp_secret(current_user.id)
        .await
        .map_err(ctx.err())?
        .filter(|s| !s.is_empty());

    if form.action == "bind" {
        if totp_secret.is_some() {
            return ctx.error(t!(ctx, "user.totp_already_bound")).into();
        }
        let Some(secret_str) = form.secret else {
            return ctx.error(t!(ctx, "user.param_error")).into();
        };

        let secret_bytes = match Secret::Encoded(secret_str.clone()).to_bytes() {
            Ok(s) => s,
            Err(_) => return ctx.error(t!(ctx, "user.secret_format_error")).into(),
        };

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            Some(ctx.website.nickname.clone()),
            user.email.clone(),
        ).map_err(ctx.err())?;

        if !totp.check_current(&form.code).unwrap_or(false) {
            let Ok(qr) = totp.get_qr_base64().inspect_err(|err| {
                warn!("create QR code failed: {:?}", err);
            }) else {
                return ctx.error("创建二维码失败").into();
            };
            return TotpSettingsTemplate {
                tips: t_owned!(ctx, "user.code_wrong"),
                ctx: ctx.context("两步验证".to_string()),
                is_bound: false,
                qr_code: Some(qr),
                secret: Some(secret_str),
            }
            .into();
        }

        store
            .update_totp_secret(user.id, Some(secret_str))
            .await
            .map_err(ctx.err())?;

        return RedirectTemplate {
            tips: &t!(ctx, "user.bind_success"),
            url: "/user/settings/totp",
            ctx: ctx.context(t_owned!(ctx, "user.bind_success")),
        }
        .into();
    } else if form.action == "unbind" {
        // Verify code before unbind? Usually good practice, but not strictly required by task.
        // Let's require it.
        let Some(secret_str) = totp_secret.clone() else {
            return ctx.error(t!(ctx, "user.totp_not_bound")).into();
        };
        let secret_bytes = Secret::Encoded(secret_str).to_bytes().map_err(ctx.err())?;
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            None,
            user.email.clone(),
        ).map_err(ctx.err())?;

        if !totp.check_current(&form.code).unwrap_or(false) {
            return TotpSettingsTemplate {
                tips: "验证码错误".to_string(),
                ctx: ctx.context("两步验证".to_string()),
                is_bound: true,
                qr_code: None,
                secret: None,
            }
            .into();
        }

        store
            .update_totp_secret(user.id, None)
            .await
            .map_err(ctx.err())?;

        return RedirectTemplate {
            tips: &t!(ctx, "user.unbind_success"),
            url: "/user/settings/totp",
            ctx: ctx.context(t_owned!(ctx, "user.unbind_success")),
        }
        .into();
    }

    ctx.error(t!(ctx, "user.unknown_action")).into()
}
