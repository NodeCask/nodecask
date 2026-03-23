use crate::common::{Context, CurrentUser, GlobalContext, OptionalCurrentUser, WebResponse};
use crate::daemon::email::{EmailSenderDaemon, Mail};
use crate::home::{AppState, PasswordChallengeCounter, RedirectTemplate};
use crate::moderator::Moderator;
use crate::store::system::RegisterConfig;
use crate::store::user::User;
use crate::store::Store;
use crate::{t, t_owned};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use askama::Template;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum::Form;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use into_response_derive::TemplateResponse;
use log::{error, warn};
use serde::Deserialize;
use sqlx::types::time::OffsetDateTime;
use tantivy::time::Duration;
use totp_rs::{Algorithm, Secret, TOTP};
use tower_sessions::Session;
use url::Url;

#[derive(Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub token: String,
    #[serde(alias = "cf-turnstile-response")]
    pub turnstile_response: Option<String>,
    pub invite_code: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub token: String,
    pub remember_me: Option<String>,
    #[serde(alias = "cf-turnstile-response")]
    pub turnstile_response: Option<String>,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/auth/login.html")]
pub struct AuthLoginTemplate {
    pub token: String,
    pub tips: String,
    pub username: String,
    pub password: String,
    pub turnstile: Option<(String, String)>,
    pub ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/auth/register0.html")]
pub struct AuthRegisterZeroTemplate {
    pub token: String,
    pub email: String,
    pub tips: String,
    pub invite_code: String,
    pub turnstile: Option<(String, String)>,
    pub ctx: crate::common::VContext,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/auth/register.html")]
pub struct AuthRegisterTemplate {
    pub username: String,
    pub password: String,
    pub email: String,
    pub invite_code: String,
    pub tips: String,
    pub token: String,
    pub register_config: RegisterConfig,
    pub turnstile: Option<(String, String)>,
    pub ctx: crate::common::VContext,
}
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!(e))?
        .to_string();
    Ok(password_hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|e| anyhow::anyhow!(e))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
#[derive(Debug, Deserialize)]
pub struct RegisterFormQuery {
    token: Option<String>,
    email: Option<String>,
    invite_code: Option<String>,
}
#[axum::debug_handler(state = AppState)]
pub async fn register_form(
    ctx: Context,
    store: Store,
    Query(q): Query<RegisterFormQuery>,
    _user: OptionalCurrentUser,
) -> WebResponse {
    let cfg = store.get_register_config().await;
    if !cfg.enable {
        return ctx.error(t!(ctx, "auth.register_disabled")).into();
    }
    let cfg = store.get_register_config().await;
    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile = turnstile_cfg.get_site_key("register");
    if cfg.email_verify {
        if q.token.clone().filter(|s| !s.is_empty()).is_none() {
            let token = store
                .get_token(0, "register#get-email-token")
                .await
                .map_err(ctx.err())?;
            return AuthRegisterZeroTemplate {
                token,
                invite_code: q.invite_code.unwrap_or_default(),
                email: "".to_string(),
                tips: "".to_string(),
                ctx: ctx.context(t_owned!(ctx, "auth.register")),
                turnstile,
            }
            .into();
        }
        let y = store
            .is_token_exists(
                0,
                &format!("register#{}", q.email.clone().unwrap_or_default()),
                q.token.unwrap_or_default().as_str(),
            )
            .await
            .unwrap_or_default();
        if !y {
            // token 已过期，需要重新生成
            return ctx.error(t!(ctx, "auth.register_link_expired")).into();
        }
    }

    let token = store.get_token(0, "register").await.map_err(ctx.err())?;
    AuthRegisterTemplate {
        username: "".to_string(),
        password: "".to_string(),
        email: q.email.unwrap_or_default(),
        invite_code: q.invite_code.unwrap_or_default(),
        token,
        tips: "".to_string(),
        register_config: cfg,
        turnstile,
        ctx: ctx.context(t_owned!(ctx, "auth.register")),
    }
    .into()
}

#[derive(Deserialize)]
pub struct GetRegisterLinkForm {
    pub token: String,
    pub email: String,
    #[serde(alias = "cf-turnstile-response")]
    pub turnstile_response: Option<String>,
    pub invite_code: Option<String>,
}
#[derive(Template)]
#[template(path = "email/register_link.html")]
pub struct RegisterLinkTemplate {
    pub url: String,
}
/// 发送注册链接到邮箱
#[axum::debug_handler(state = AppState)]
pub async fn register_get_link(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    _session: Session,
    GlobalContext(sender): GlobalContext<EmailSenderDaemon>,
    Form(form): Form<GetRegisterLinkForm>,
) -> WebResponse {
    let cfg = store.get_register_config().await;
    if !cfg.enable {
        return ctx.error(t!(ctx, "auth.register_disabled")).into();
    }
    if !cfg.email_verify {
        return ctx.error(t!(ctx, "auth.email_verify_disabled")).into();
    }
    if !store
        .verify_token(0, "register#get-email-token", &form.token)
        .await
        .map_err(ctx.err())?
    {
        return ctx
            .redirect(&t!(ctx, "auth.form_expired"), "/register")
            .into();
    }

    // CF Turnstile 人机校验
    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile = turnstile_cfg.get_site_key("register");
    if turnstile.is_some() {
        let token = store
            .get_token(0, "register#get-email-token")
            .await
            .map_err(ctx.err())?;
        let Some(turnstile_response) = form.turnstile_response.filter(|k| !k.is_empty()) else {
            return AuthRegisterZeroTemplate {
                token,
                turnstile,
                email: form.email.clone(),
                invite_code: form.invite_code.unwrap_or_default(),
                tips: t_owned!(ctx, "auth.captcha_required"),
                ctx: ctx.context(t_owned!(ctx, "auth.register")),
            }
            .into();
        };

        match turnstile_cfg.validate("register", &turnstile_response).await {
            Ok(true) => {}
            Ok(false) => {
                return AuthRegisterZeroTemplate {
                    token,
                    turnstile,
                    email: form.email.clone(),
                    invite_code: form.invite_code.unwrap_or_default(),
                    tips: t_owned!(ctx, "auth.captcha_failed"),
                    ctx: ctx.context(t_owned!(ctx, "auth.register")),
                }
                .into();
            }
            Err(err) => {
                return AuthRegisterZeroTemplate {
                    token,
                    turnstile,
                    email: form.email.clone(),
                    invite_code: form.invite_code.unwrap_or_default(),
                    tips: t_owned!(ctx, "auth.captcha_error", err = err),
                    ctx: ctx.context(t_owned!(ctx, "auth.register")),
                }
                .into();
            }
        }
    }

    let email_exists = store
        .check_email_exists(&form.email)
        .await
        .map_err(ctx.err())?;
    if email_exists {
        let token = store
            .get_token(0, "register#get-email-token")
            .await
            .map_err(ctx.err())?;
        return AuthRegisterZeroTemplate {
            token,
            turnstile,
            email: form.email.clone(),
            invite_code: form.invite_code.unwrap_or_default(),
            tips: t_owned!(ctx, "auth.email_already_registered"),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
        }
        .into();
    }
    let token = store
        .get_token(0, &format!("register#{}", &form.email))
        .await
        .map_err(ctx.err())?;

    let Ok(u) = Url::parse(&ctx.website.domain) else {
        error!(
            "Invalid website domain configuration: {}",
            &ctx.website.domain
        );
        return ctx.error(t!(ctx, "auth.invalid_domain_config")).into();
    };
    let mut u = u.join("/register").map_err(ctx.err())?;
    u.query_pairs_mut()
        .append_pair("token", &token)
        .append_pair("email", &form.email);
    if let Some(code) = form.invite_code.as_ref() {
        u.query_pairs_mut().append_pair("invite_code", code);
    }
    let msg = RegisterLinkTemplate { url: u.to_string() }
        .render()
        .unwrap_or_default();
    if msg.is_empty() {
        return ctx.internal_error().into();
    }

    let id = store
        .add_email_queue(0, "", &form.email, &t!(ctx, "auth.register_link"), &msg)
        .await
        .map_err(ctx.err())?;

    sender.send(Mail(id)).await;

    ctx.success(&t!(ctx, "auth.register_link_sent")).into()
}
#[axum::debug_handler(state = AppState)]
pub async fn register_post(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    session: Session,
    Query(q): Query<RegisterFormQuery>,
    Form(form): Form<RegisterForm>,
) -> WebResponse {
    let cfg = store.get_register_config().await;
    if !cfg.enable {
        return ctx.error(t!(ctx, "auth.register_disabled")).into();
    }
    // CF Turnstile 人机校验
    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile_site_key = turnstile_cfg.get_site_key("register");
    let placeholder = AuthRegisterTemplate {
        username: form.username.clone(),
        password: form.password.clone(),
        email: form.email.clone(),
        invite_code: form.invite_code.clone().unwrap_or_default(),
        tips: "".to_string(),
        token: store.get_token(0, "register").await.map_err(ctx.err())?,
        register_config: cfg.clone(),
        turnstile: turnstile_site_key.clone(),
        ctx: ctx.context("__placeholder_title__".to_string()),
    };
    if !store
        .verify_token(0, "register", form.token.as_str())
        .await
        .map_err(ctx.err())?
    {
        return AuthRegisterTemplate {
            tips: t_owned!(ctx, "auth.page_expired"),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
            ..placeholder
        }
        .into();
    }
    if cfg.email_verify {
        if q.email.is_none()
            || q.token.is_none()
            || !store
                .is_token_exists(
                    0,
                    &format!("register#{}", &q.email.clone().unwrap_or_default()),
                    &q.token.unwrap_or_default(),
                )
                .await
                .unwrap_or_default()
        {
            return ctx.error(t!(ctx, "auth.register_link_expired")).into();
        }
        if q.email.unwrap_or_default() != form.email {
            return ctx.error(t!(ctx, "auth.email_mismatch")).into();
        }
    }

    // 验证邮箱不能为空
    if form.email.trim().is_empty() {
        return AuthRegisterTemplate {
            tips: t_owned!(ctx, "auth.email_required"),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
            ..placeholder
        }
        .into();
    }

    if !is_valid_username(form.username.as_str()) {
        return AuthRegisterTemplate {
            tips: t_owned!(ctx, "auth.username_format_error"),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
            ..placeholder
        }
        .into();
    }
    if cfg.min_username > form.username.len() as i64 {
        return AuthRegisterTemplate {
            tips: t_owned!(ctx, "auth.username_min_length", min = cfg.min_username),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
            ..placeholder
        }
        .into();
    }
    if cfg.max_username < form.username.len() as i64 {
        return AuthRegisterTemplate {
            tips: t_owned!(ctx, "auth.username_max_length", max = cfg.max_username),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
            ..placeholder
        }
        .into();
    }

    if form.password.len() < cfg.min_password_len {
        return AuthRegisterTemplate {
            tips: t_owned!(ctx, "auth.password_min_length", min = cfg.min_password_len),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
            ..placeholder
        }
        .into();
    }

    let invite_code = if cfg.invite_code_required {
        let Some(invite_code) = form
            .invite_code
            .clone()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
        else {
            return AuthRegisterTemplate {
                tips: t_owned!(ctx, "auth.invite_code_required"),
                ctx: ctx.context(t_owned!(ctx, "auth.register")),
                ..placeholder
            }
            .into();
        };
        Some(invite_code)
    } else {
        None
    };

    let email_exists = store
        .check_email_exists(&form.email)
        .await
        .map_err(ctx.err())?;
    if email_exists {
        return AuthRegisterTemplate {
            tips: t_owned!(ctx, "auth.email_already_registered"),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
            ..placeholder
        }
        .into();
    }
    let username_exists = store
        .check_username_exists(&form.username)
        .await
        .map_err(ctx.err())?;
    if username_exists {
        return AuthRegisterTemplate {
            tips: t_owned!(ctx, "auth.username_taken"),
            ctx: ctx.context(t_owned!(ctx, "auth.register")),
            ..placeholder
        }
        .into();
    }
    if let Some(code) = invite_code.as_ref() {
        if !store.valid_invite_code(code).await? {
            return AuthRegisterTemplate {
                tips: t_owned!(ctx, "auth.invalid_invite_code"),
                ctx: ctx.context(t_owned!(ctx, "auth.register")),
                ..placeholder
            }
            .into();
        }
    }

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Error hashing password: {}", e);
            return ctx.internal_error().into();
        }
    };

    // CF 人机验证放到最后检测，因为需要请求外站接口
    if turnstile_site_key.is_some() {
        let Some(turnstile_response) = form.turnstile_response.filter(|k| !k.is_empty()) else {
            return AuthRegisterTemplate {
                tips: t_owned!(ctx, "auth.captcha_required"),
                ctx: ctx.context(t_owned!(ctx, "auth.register")),
                ..placeholder
            }
            .into();
        };

        match turnstile_cfg.validate("register", &turnstile_response).await {
            Ok(true) => {}
            Ok(false) => {
                return AuthRegisterTemplate {
                    tips: t_owned!(ctx, "auth.captcha_failed"),
                    ctx: ctx.context(t_owned!(ctx, "auth.register")),
                    ..placeholder
                }
                .into();
            }
            Err(err) => {
                return AuthRegisterTemplate {
                    tips: t_owned!(ctx, "auth.captcha_error", err = err),
                    ctx: ctx.context(t_owned!(ctx, "auth.register")),
                    ..placeholder
                }
                .into();
            }
        }
    }

    let result = store
        .create_user_with_invite(
            &form.username,
            &password_hash,
            &form.email,
            ctx.language.as_str(),
            cfg.initial_score,
            invite_code,
        )
        .await;

    match result {
        Ok(_) => {
            if cfg.email_verify {
                let _ = store
                    .remove_token(0, &format!("register#{}", &form.email))
                    .await;
            }
            let Some(user) = store.get_user(&form.username).await.map_err(ctx.err())? else {
                return ctx.error("系统异常").into();
            };
            session
                .insert("CurrentUser", &CurrentUser::from(&user))
                .await
                .map_err(ctx.err())?;
            RedirectTemplate {
                tips: &t!(ctx, "auth.register_success"),
                url: "/",
                ctx: ctx.context(t_owned!(ctx, "auth.register_success")),
            }
            .into()
        }
        Err(e) => {
            error!("Error creating user: {}", e);
            AuthRegisterTemplate {
                tips: e.to_string(),
                ctx: ctx.context(t_owned!(ctx, "auth.register")),
                ..placeholder
            }
            .into()
        }
    }
}
pub fn is_valid_username(username: &str) -> bool {
    // 1. 使用 '_' 分割字符串
    // 2. 使用 all() 检查所有片段是否都满足条件
    username.split('_').all(|segment| {
        // 条件 A: 片段不能为空
        // (这也隐含处理了开头/结尾是下划线，或者连续下划线的情况，因为 split 会产生空字符串)
        !segment.is_empty() &&
            // 条件 B: 片段中的所有字符必须是字母或数字
            segment.chars().all(char::is_alphanumeric)
    })
}

#[axum::debug_handler(state = AppState)]
pub async fn login_form(ctx: Context, store: Store, _user: OptionalCurrentUser) -> WebResponse {
    let cfg = store.get_turnstile_config().await;
    AuthLoginTemplate {
        token: store.get_token(0, "login").await.map_err(ctx.err())?,
        tips: "".to_string(),
        username: "".to_string(),
        password: "".to_string(),
        ctx: ctx.context(t_owned!(ctx, "auth.login")),
        turnstile: cfg.get_site_key("login"),
    }
    .into()
}

#[axum::debug_handler(state = AppState)]
pub async fn login_post(
    ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> WebResponse {
    let cfg = store.get_turnstile_config().await;
    let turnstile_site_key = cfg.get_site_key("login");
    // 如果开启了 cf turnstile，先进行人机校验
    let placeholder = AuthLoginTemplate {
        token: store.get_token(0, "login").await.map_err(ctx.err())?,
        tips: "".to_string(),
        username: form.username.clone(),
        password: form.password.clone(),
        ctx: ctx.context("".to_string()),
        turnstile: turnstile_site_key.clone(),
    };
    if !store
        .verify_token(0, "login", form.token.as_str())
        .await
        .map_err(ctx.err())?
    {
        return AuthLoginTemplate {
            tips: t_owned!(ctx, "auth.page_expired"),
            ctx: ctx.context(t_owned!(ctx, "auth.login")),
            ..placeholder
        }
        .into();
    }
    if turnstile_site_key.is_some() {
        let Some(turnstile_response) = form.turnstile_response.filter(|k| !k.is_empty()) else {
            return AuthLoginTemplate {
                tips: t_owned!(ctx, "auth.captcha_required"),
                ctx: ctx.context(t_owned!(ctx, "auth.login")),
                ..placeholder
            }
            .into();
        };

        match cfg.validate("login", &turnstile_response).await {
            Ok(true) => {}
            Ok(false) => {
                return AuthLoginTemplate {
                    tips: t_owned!(ctx, "auth.captcha_failed"),
                    ctx: ctx.context(t_owned!(ctx, "auth.login")),
                    ..placeholder
                }
                .into();
            }
            Err(err) => {
                return AuthLoginTemplate {
                    tips: t_owned!(ctx, "auth.captcha_error", err = err),
                    ctx: ctx.context(t_owned!(ctx, "auth.login")),
                    ..placeholder
                }
                .into();
            }
        }
    }

    let user = if form.username.contains("@") {
        store.get_user_by_email(&form.username).await
    } else {
        store.get_user(&form.username).await
    };
    let Some(user) = user.map_err(ctx.err())? else {
        return AuthLoginTemplate {
            tips: t_owned!(ctx, "auth.invalid_credentials"),
            ctx: ctx.context(t_owned!(ctx, "auth.login")),
            ..placeholder
        }
        .into();
    };

    let counter = match store
        .get_user_attr::<PasswordChallengeCounter>(user.id, "login-challenge-counter")
        .await
    {
        Ok(Some(PasswordChallengeCounter { count, expires })) => {
            if count >= 5 && expires > chrono::Utc::now() {
                return AuthLoginTemplate {
                    tips: t_owned!(ctx, "auth.too_many_attempts"),
                    ctx: ctx.context(t_owned!(ctx, "auth.login")),
                    ..placeholder
                }
                .into();
            }
            if expires > chrono::Utc::now() {
                count
            } else {
                0
            }
        }
        Ok(None) => 0,
        Err(err) => {
            return AuthLoginTemplate {
                tips: err.to_string(),
                ctx: ctx.context(t_owned!(ctx, "auth.login")),
                ..placeholder
            }
            .into();
        }
    };

    if !verify_password(&form.password, &user.password_hash).unwrap_or(false) {
        let c = PasswordChallengeCounter {
            count: counter + 1,
            expires: chrono::Utc::now() + chrono::Duration::hours(1),
        };
        if let Err(err) = store
            .set_user_attr(user.id, "login-challenge-counter", Some(&c))
            .await
        {
            warn!("Failed to update user login failure counter: {}", err)
        }
        return AuthLoginTemplate {
            tips: t_owned!(ctx, "auth.invalid_credentials"),
            ctx: ctx.context(t_owned!(ctx, "auth.login")),
            ..placeholder
        }
        .into();
    };

    if !user.active {
        return AuthLoginTemplate {
            tips: t_owned!(ctx, "auth.user_banned"),
            ctx: ctx.context(t_owned!(ctx, "auth.login")),
            ..placeholder
        }
        .into();
    }

    // 清空登录计数器
    if let Err(err) = store
        .set_user_attr::<PasswordChallengeCounter>(user.id, "login-challenge-counter", None)
        .await
    {
        warn!("Failed to update user login failure counter: {}", err)
    }
    let totp_secret = store
        .get_totp_secret(user.id)
        .await
        .map_err(ctx.err())?
        .filter(|s| !s.is_empty());
    if totp_secret.is_some() {
        let token = store
            .get_token(user.id, "login_totp")
            .await
            .map_err(ctx.err())?;
        let mut url = Url::parse("http://localhost")?.join("/login-totp-challenge")?;
        url.query_pairs_mut()
            .append_pair("token", &token)
            .append_pair("username", &user.username);

        if let Some(remember) = form.remember_me {
            url.query_pairs_mut().append_pair("remember_me", &remember);
        }
        let path = url.path();
        let query = url.query().unwrap_or_default();
        return Ok(Redirect::to(&format!("{}?{}", path, query)).into_response());
    }

    login_after(ctx, store, user, jar, form.remember_me.is_some()).await
}
async fn theme_jar(store: &Store, uid: i64, jar: CookieJar) -> CookieJar {
    #[derive(Deserialize)]
    struct PreferredTheme {
        theme: String,
    }
    if let Some(theme) = store
        .get_user_attr::<PreferredTheme>(uid, "preferred-theme")
        .await
        .unwrap_or_default()
    {
        let e: OffsetDateTime = OffsetDateTime::now_utc() + Duration::days(30);
        return jar.add(Cookie::build(("theme", theme.theme)).expires(e));
    }
    jar
}
#[derive(Template, TemplateResponse)]
#[template(path = "home/auth/login-totp.html")]
struct LoginTotpTemplate {
    tips: String,
    token: String,
    username: String,
    remember_me: Option<String>,
    ctx: crate::common::VContext,
}

#[derive(Deserialize)]
pub struct LoginTotpForm {
    pub token: String,
    pub username: String,
    pub code: String,
    pub remember_me: Option<String>,
}
#[derive(Deserialize)]
pub struct LoginTotpQuery {
    pub token: String,
    pub username: String,
    pub remember_me: Option<String>,
}

#[axum::debug_handler(state = AppState)]
pub async fn login_totp_challenge_page(
    ctx: Context,
    Query(q): Query<LoginTotpQuery>,
) -> WebResponse {
    LoginTotpTemplate {
        tips: "".to_string(),
        token: q.token,
        username: q.username,
        remember_me: q.remember_me,
        ctx: ctx.context(t_owned!(ctx, "auth.two_factor")),
    }
    .into()
}

#[axum::debug_handler(state = AppState)]
pub async fn login_totp_challenge_post(
    ctx: Context,
    store: Store,
    jar: CookieJar,
    Form(form): Form<LoginTotpForm>,
) -> WebResponse {
    let Some(user) = store.get_user(&form.username).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "auth.user_not_found")).into();
    };
    let totp_secret = store
        .get_totp_secret(user.id)
        .await
        .map_err(ctx.err())?
        .filter(|s| !s.is_empty());

    // Verify token
    match store
        .verify_token(user.id, "login_totp", &form.token)
        .await
        .map_err(ctx.err())?
    {
        true => {}
        false => {
            return LoginTotpTemplate {
                tips: t_owned!(ctx, "auth.page_expired"),
                token: form.token,
                username: form.username,
                remember_me: form.remember_me,
                ctx: ctx.context(t_owned!(ctx, "auth.two_factor")),
            }
            .into();
        }
    }

    // Verify TOTP
    let Some(ref secret_str) = totp_secret else {
        return ctx.error(t!(ctx, "auth.totp_not_enabled")).into();
    };

    let secret_bytes = Secret::Encoded(secret_str.to_string()).to_bytes().unwrap();
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
        return LoginTotpTemplate {
            tips: t_owned!(ctx, "auth.totp_invalid"),
            token: form.token,
            username: form.username,
            remember_me: form.remember_me,
            ctx: ctx.context(t_owned!(ctx, "auth.two_factor")),
        }
        .into();
    }
    // Success login
    login_after(ctx, store, user, jar, form.remember_me.is_some()).await
}
async fn login_after(
    ctx: Context,
    store: Store,
    user: User,
    jar: CookieJar,
    remember_me: bool,
) -> WebResponse {
    ctx.set("CurrentUser", CurrentUser::from(&user)).await;
    if &user.role == "moderator" {
        ctx.set(
            "Moderator",
            Moderator {
                uid: user.id,
                username: user.username.clone(),
            },
        )
        .await;
    }
    let template = RedirectTemplate {
        tips: &t!(ctx, "auth.login_success"),
        url: "/",
        ctx: ctx.context(t_owned!(ctx, "auth.login_success")),
    };

    let e: OffsetDateTime = OffsetDateTime::now_utc() + Duration::days(30);
    let jar = if !user.language.is_empty() {
        jar.add(Cookie::build(("language", user.language.clone())).expires(e))
    } else {
        jar
    };

    let jar = if remember_me {
        let remember_me_token = store
            .create_access_token_with_days(user.id, "remember_me", "remember_me", 30)
            .await
            .map_err(ctx.err())?;
        jar.add(Cookie::build(("remember_me", remember_me_token)).expires(e))
    } else {
        jar
    };

    Ok((theme_jar(&store, user.id, jar).await, template).into_response())
}
#[axum::debug_handler(state = AppState)]
pub async fn logout_handler(
    ctx: Context,
    _store: Store,
    session: Session,
    jar: CookieJar,
    current_user: OptionalCurrentUser,
) -> WebResponse {
    if current_user.is_none() {
        return Ok((jar, Redirect::to("/")).into_response());
    }
    session.clear().await;
    let jar = jar.remove(Cookie::from("remember_me"));
    let resp = RedirectTemplate {
        tips: &t!(ctx, "auth.logout_success"),
        url: "/",
        ctx: ctx.context(t_owned!(ctx, "auth.logout_success")),
    };
    Ok((jar, resp).into_response())
}
#[derive(Deserialize)]
pub struct ForgotPasswordForm {
    pub email: String,
    token: String,
    #[serde(alias = "cf-turnstile-response")]
    pub turnstile_response: Option<String>,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/auth/forgot_password.html")]
pub struct ForgotPasswordTemplate {
    pub email: String,
    pub tips: String,
    pub token: String,
    pub turnstile: Option<(String, String)>,
    pub ctx: crate::common::VContext,
}

#[axum::debug_handler(state = AppState)]
pub async fn forgot_password_form(
    ctx: Context,
    store: Store,
    _user: OptionalCurrentUser,
) -> WebResponse {
    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile = turnstile_cfg.get_site_key("password-reset");
    let t = ForgotPasswordTemplate {
        email: "".to_string(),
        tips: "".to_string(),
        token: store.get_token(0, "password-reset-request").await?,
        turnstile,
        ctx: ctx.context(t_owned!(ctx, "auth.forgot_password")),
    };
    t.into()
}

#[derive(Template)]
#[template(path = "email/reset_password_link.html")]
pub struct ResetPasswordLinkTemplate {
    pub url: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn forgot_password_post(
    ctx: Context,
    store: Store,
    GlobalContext(sender): GlobalContext<EmailSenderDaemon>,
    Form(form): Form<ForgotPasswordForm>,
) -> WebResponse {
    let turnstile_cfg = store.get_turnstile_config().await;
    let turnstile = turnstile_cfg.get_site_key("password-reset");
    let placeholder = ForgotPasswordTemplate {
        email: form.email.clone(),
        tips: "".to_string(),
        token: store.get_token(0, "password-reset-request").await?,
        turnstile: turnstile.clone(),
        ctx: ctx.context(t_owned!(ctx, "auth.forgot_password")),
    };
    let email = form.email.trim();
    if email.is_empty() {
        return ForgotPasswordTemplate {
            tips: t_owned!(ctx, "auth.email_required"),
            ..placeholder
        }.into();
    }
    match store
        .verify_token(0, "password-reset-request", form.token.as_str())
        .await
        .map_err(ctx.err())?
    {
        true => {}
        false => {
            return ForgotPasswordTemplate {
                tips: t_owned!(ctx, "auth.form_expired"),
                ..placeholder
            }
                .into();
        }
    }
    if turnstile.is_some() {
        let Some(turnstile_response) = form.turnstile_response.clone().filter(|k| !k.is_empty())
        else {
            return ForgotPasswordTemplate {
                tips: t_owned!(ctx, "auth.captcha_required"),
                ..placeholder
            }.into();
        };

        match turnstile_cfg.validate("password-reset", &turnstile_response).await {
            Ok(true) => {}
            Ok(false) => {
                return ForgotPasswordTemplate {
                    tips: t_owned!(ctx, "auth.captcha_failed"),
                    ..placeholder
                }
                    .into();
            }
            Err(err) => {
                return ForgotPasswordTemplate {
                    tips: t_owned!(ctx, "auth.captcha_error", err = err),
                    ..placeholder
                }.into();
            }
        }
    }

    let Some(user) = store.get_user_by_email(email).await.map_err(ctx.err())? else {
        // 为了安全起见，即使用户不存在也提示发送成功，防止枚举用户名
        // 但为了方便调试，暂时还是如果不存在就啥也不干，或者提示发送成功
        // 这里模仿 register_get_link 的逻辑，如果有错误就提示，但这里为了安全最好模糊处理
        // 不过 register_get_link 也是明确提示 "邮箱已经注册"，所以这里可以明确提示 "邮箱未注册" 吗？
        // 还是保持一致性，如果 register 泄露了，这里也没必要藏着掖着。
        // 不过 register 泄露是因为要防止重复注册。这里是忘记密码。
        // 让我们还是提示 "如果邮箱存在，重置链接已发送" 吧。
        return ctx
            .success(&t!(ctx, "auth.reset_link_sent_if_exists"))
            .into();
    };

    let token = store
        .get_token(user.id, "password-reset")
        .await
        .map_err(ctx.err())?;

    let Ok(u) = Url::parse(&ctx.website.domain) else {
        error!(
            "Invalid website domain configuration: {}",
            &ctx.website.domain
        );
        return ctx.internal_error().into();
    };
    let mut u = u.join("/reset-password")?;
    u.query_pairs_mut()
        .append_pair("token", &token)
        .append_pair("email", email);

    let msg = ResetPasswordLinkTemplate { url: u.to_string() }
        .render()
        .unwrap_or_default();

    if msg.is_empty() {
        return ctx.internal_error().into();
    }

    match store
        .add_email_queue(user.id, "", email, &t!(ctx, "auth.reset_password"), &msg)
        .await
    {
        Ok(id) => {
            sender.send(Mail(id)).await;
        }
        Err(err) => {
            error!("Failed to add email to queue: {:?}", err);
            return ctx.internal_error().into();
        }
    }

    ctx.success(&t!(ctx, "auth.reset_link_sent")).into()
}

#[derive(Deserialize)]
pub struct ResetPasswordQuery {
    pub token: String,
    pub email: String,
}

#[derive(Template, TemplateResponse)]
#[template(path = "home/auth/reset_password.html")]
pub struct ResetPasswordTemplate {
    pub token: String,
    pub email: String,
    pub tips: String,
    pub ctx: crate::common::VContext,
}

#[axum::debug_handler(state = AppState)]
pub async fn reset_password_form(
    ctx: Context,
    store: Store,
    Query(q): Query<ResetPasswordQuery>,
) -> WebResponse {
    let Some(user) = store.get_user_by_email(&q.email).await.map_err(ctx.err())? else {
        return ctx.error(t!(ctx, "auth.invalid_request")).into();
    };

    match store
        .is_token_exists(user.id, "password-reset", &q.token)
        .await
        .map_err(ctx.err())?
    {
        true => {}
        false => {
            return ctx.error(t!(ctx, "auth.link_expired_or_invalid")).into();
        }
    }

    let t = ResetPasswordTemplate {
        token: q.token,
        email: q.email,
        tips: "".to_string(),
        ctx: ctx.context(t_owned!(ctx, "auth.reset_password")),
    };
    t.into()
}

#[derive(Deserialize)]
pub struct ResetPasswordForm {
    pub token: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn reset_password_post(
    ctx: Context,
    store: Store,
    Form(form): Form<ResetPasswordForm>,
) -> WebResponse {
    if form.password != form.confirm_password {
        let t = ResetPasswordTemplate {
            token: form.token,
            email: form.email,
            tips: t_owned!(ctx, "auth.passwords_mismatch"),
            ctx: ctx.context(t_owned!(ctx, "auth.reset_password")),
        };
        return t.into();
    }

    let Some(user) = store
        .get_user_by_email(&form.email)
        .await
        .map_err(ctx.err())?
    else {
        return ctx.error(t!(ctx, "auth.user_not_found")).into();
    };

    match store
        .verify_token(user.id, "password-reset", &form.token)
        .await
        .map_err(ctx.err())?
    {
        true => {}
        false => {
            return ctx.error(t!(ctx, "auth.link_expired_or_invalid")).into();
        }
    }

    let password_hash = match hash_password(&form.password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Error hashing password: {}", e);
            return ctx.internal_error().into();
        }
    };

    match store.user_password_reset(user.id, &password_hash).await {
        Ok(_) => RedirectTemplate {
            tips: &t!(ctx, "auth.password_reset_success"),
            url: "/login",
            ctx: ctx.context(t_owned!(ctx, "auth.password_reset_success")),
        }
        .into(),
        Err(e) => {
            error!("Error resetting password: {}", e);
            ctx.error(t!(ctx, "auth.password_reset_failed")).into()
        }
    }
}
