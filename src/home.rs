pub(crate) mod auth;
pub(crate) mod bot;
pub(crate) mod node;
pub(crate) mod page;
pub(crate) mod topic;
pub(crate) mod user;
pub(crate) mod debug;
pub(crate) mod public;

use crate::common::{AppState, Context, VContext};
use askama::Template;
use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::Deserialize;


use into_response_derive::TemplateResponse;

use crate::t;
#[derive(Template, TemplateResponse)]
#[template(path = "common/redirect.html")]
pub struct RedirectTemplate<'a> {
    pub tips: &'a str,
    pub url: &'a str,
    pub ctx: VContext,
}


use crate::moderator;
use crate::store::Store;
use serde::Serialize;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(public::index))
        .route("/robots.txt", get(public::robots))
        .route("/select-theme", get(public::select_theme))
        .route(
            "/register",
            get(auth::register_form).post(auth::register_post),
        )
        .route("/get-register-link", post(auth::register_get_link))
        .route("/login", get(auth::login_form).post(auth::login_post))
        .route(
            "/login-totp-challenge",
            get(auth::login_totp_challenge_page).post(auth::login_totp_challenge_post),
        )
        .route(
            "/forgot-password",
            get(auth::forgot_password_form).post(auth::forgot_password_post),
        )
        .route(
            "/reset-password",
            get(auth::reset_password_form).post(auth::reset_password_post),
        )
        .route("/logout", get(auth::logout_handler))
        .route("/error_tips", get(error_tips))
        .route("/search", get(topic::search))
        .route("/notifications", get(user::notifications))
        .route("/notifications/clear", post(user::notifications_clear))
        .route("/recent", get(topic::recent))
        .route("/statistics", get(node::statistics))
        .route("/recent/rss.xml", get(topic::rss_recent))
        .route("/nodes", get(node::nodes))
        .route("/nodes-new", get(node::nodes_new))
        .route("/go/{slug}", get(node::topics))
        .route("/go/{slug}/rss.xml", get(node::rss))
        .route(
            "/go/{slug}/new",
            get(topic::create_form).post(topic::create_post),
        )
        .route("/t/{id}", get(topic::read))
        .route("/t/{id}/edit", get(topic::edit_form).post(topic::edit_post))
        .route(
            "/t/{id}/delete",
            get(topic::delete_confirm).post(topic::delete_post),
        )
        .route(
            "/t/{id}/lock",
            get(topic::lock_confirm).post(topic::lock_post),
        )
        .route("/comment", post(topic::comment_post))
        .route("/comment/{id}/delete", post(topic::comment_delete))
        .route(
            "/super/t/{id}/pin",
            get(topic::manager::pin_confirm).post(topic::manager::pin_post),
        )
        .route(
            "/super/t/{id}/delete",
            get(topic::manager::delete_confirm).post(topic::manager::delete_post),
        )
        .route(
            "/super/t/{id}/move",
            get(topic::manager::move_confirm).post(topic::manager::move_post),
        )
        .route(
            "/super/comment/{id}/delete",
            get(topic::manager::comment_delete_confirm).post(topic::manager::comment_delete_post),
        )
        .route("/u/{username}", get(user::info))
        .route("/u/{username}/topics", get(user::topics))
        .route("/u/{username}/comments", get(user::comments))
        .route("/u/{username}/follow", post(user::follow))
        .route("/u/{username}/unfollow", post(user::unfollow))
        .route("/u/{username}/block", post(user::block))
        .route("/u/{username}/unblock", post(user::unblock))
        .route("/u/{uid}/avatar.png", get(user::avatar))
        .route("/user/center", get(user::center))
        .route("/user/checkin", post(user::checkin))
        .route(
            "/user/email/change",
            get(user::change_email_form).post(user::change_email_post),
        )
        .route("/user/email/verify", post(user::change_email_verify_post))
        .route(
            "/user/settings/password",
            get(user::change_password_form).post(user::change_password_post),
        )
        .route(
            "/user/settings/avatar",
            get(user::avatar_upload_form).post(user::avatar_upload_post),
        )
        .route(
            "/user/settings/profile",
            get(user::profile_form).post(user::profile_post),
        )
        .route(
            "/user/settings/totp",
            get(user::totp_settings_page).post(user::totp_settings_post),
        )
        .route("/get-access-token", post(moderator::auth::get_access_token))
        .route("/get-access-token/totp-challenge", post(moderator::auth::totp_challenge))
        .nest("/bot", bot::router())
        .nest("/debug", debug::router())
        .nest("/mod", moderator::router())
        .route("/public/{*path}", get(public::public_handler))
        .fallback(page::page) // 自定义页面
        .with_state(state)
}

#[axum::debug_handler(state = AppState)]
pub async fn error_tips(ctx: Context, _store: Store) -> impl IntoResponse {
    ctx.error("错误信息")
}


/// 密码错误 5 次之后，添加一个 1 小时的冷却时间
#[derive(Deserialize, Serialize, Debug)]
pub struct PasswordChallengeCounter {
    pub count: i64,
    pub expires: chrono::DateTime<chrono::Utc>,
}
