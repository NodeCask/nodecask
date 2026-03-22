use crate::daemon::config::GlobalConfigDaemon;
use crate::daemon::email::EmailSenderDaemon;
use crate::daemon::link_filter::LinkFilter;
use crate::daemon::notify::NotifyDaemon;
use crate::home::RedirectTemplate;
use crate::store::link::LinkCollection;
use crate::store::system::{InjectionConfig, Website};
use crate::store::user::{User, UserContinuousAccessDays};
use crate::store::Store;
use crate::{daemon, t};
use askama::Template;
use axum::extract::OptionalFromRequestParts;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use into_response_derive::TemplateResponse;
use log::{error, info, warn};
use object_store::ObjectStore;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::backtrace::Backtrace;
use std::cmp::min;
use std::error::Error;
use std::ops::Add;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::broadcast::error::{RecvError, SendError};
use tower_sessions::Session;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    Init,
    NewTopic {
        id: i64,
        title: String,
        username: String,
        node: String,
    },
    NewComment {
        id: i64,
        topic_id: String,
        topic_title: String,
        username: String,
        node: String,
        content: String,
    },
    TopicAt {
        id: i64,
        username: String,
        content: String,
    },
    CommentAt {
        id: i64,
        topic_id: i64,
        username: String,
        content: String,
    },
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) sender: tokio::sync::broadcast::Sender<Message>,
    pub(crate) pool: SqlitePool,
    pub(crate) moderator: daemon::moderate::ModerateDaemon,
    pub(crate) cfg: GlobalConfigDaemon,
    pub(crate) notify: NotifyDaemon,
    pub(crate) email: EmailSenderDaemon,
    pub(crate) link_filter: LinkFilter,
    pub(crate) login_rewards: daemon::login_rewards::LoginRewardsDaemon,
    pub(crate) searcher: daemon::tantivy::SearchDaemon,
    pub(crate) nsfw_detector: daemon::nsfw_detect::NSFWDetector,
    pub(crate) fs: Arc<Box<dyn ObjectStore>>,
}

impl AppState {
    pub fn store(&self) -> Store {
        Store {
            pool: self.pool.clone(),
        }
    }
    pub fn push(&self, msg: Message) {
        if let Err(err) = self.sender.send(msg) {
            warn!("unexpected error: {}", err);
        }
    }
    pub fn subscribe(&self) -> Subscriber {
        Subscriber(self.sender.subscribe())
    }
}
pub struct Broadcast(tokio::sync::broadcast::Sender<Message>);
impl Broadcast {
    pub fn send(&self, msg: Message) -> Result<usize, SendError<Message>> {
        self.0.send(msg)
    }
    pub fn send_silent(&self, msg: Message) {
        if let Err(e) = self.0.send(msg) {
            warn!("broadcast error: {}", e);
        }
    }
}
impl FromRequestParts<AppState> for Broadcast {
    type Rejection = ();

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Broadcast(state.sender.clone()))
    }
}

pub struct Subscriber(tokio::sync::broadcast::Receiver<Message>);
impl Subscriber {
    pub async fn recv(&mut self) -> Result<Message, RecvError> {
        self.0.recv().await
    }
}

impl FromRequestParts<AppState> for Subscriber {
    type Rejection = ();

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(state.subscribe())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CurrentUser {
    pub id: i64,
    pub username: String,
    pub is_moderator: bool,
    pub is_administrator: bool,
    pub unread_notifications: i64,
    pub timezone: chrono_tz::Tz,
    pub avatar_timestamp: i64,
    pub last_access_date: chrono::NaiveDate,
}
impl From<&User> for CurrentUser {
    fn from(value: &User) -> Self {
        let timezone = chrono_tz::Tz::from_str(value.timezone.as_str()).unwrap_or_default();
        Self {
            id: value.id,
            username: value.username.clone(),
            is_moderator: &value.role == "moderator" || &value.role == "administrator",
            is_administrator: &value.role == "administrator",
            unread_notifications: value.unread_notifications,
            timezone,
            avatar_timestamp: value.avatar_timestamp,
            last_access_date: value.last_access_date,
        }
    }
}
impl From<User> for CurrentUser {
    fn from(value: User) -> Self {
        let timezone = chrono_tz::Tz::from_str(value.timezone.as_str()).unwrap_or_default();
        Self {
            id: value.id,
            username: value.username,
            is_moderator: &value.role == "moderator" || &value.role == "administrator",
            is_administrator: &value.role == "administrator",
            unread_notifications: value.unread_notifications,
            timezone,
            avatar_timestamp: value.avatar_timestamp,
            last_access_date: value.last_access_date,
        }
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(u) = OptionalCurrentUser::from_request_parts(parts, state)
            .await
            .map_err(|err| err.into_response())?
        {
            Ok(u)
        } else {
            Err(Redirect::to("/login").into_response())
        }
    }
}
impl OptionalFromRequestParts<AppState> for CurrentUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Option<Self>, Self::Rejection> {
        let session = Session::from_request_parts(parts, state).await?;
        let mut current_user = session
            .get::<CurrentUser>("CurrentUser")
            .await
            .unwrap_or_default();
        let store = state.store();

        if current_user.is_none() {
            let jar = CookieJar::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to get cookie jar",
                    )
                })?;
            if let Some(cookie) = jar.get("remember_me") {
                let token = cookie.value();
                if let Ok(Some(user)) = store
                    .get_user_by_token(token, "remember_me")
                    .await
                    .inspect_err(|err| {
                        warn!("failed to get user by token: {}", err);
                    })
                {
                    let cu = CurrentUser::from(user);
                    let _ = session.insert("CurrentUser", &cu).await;
                    current_user = Some(cu);
                }
            }
        }

        let today = Utc::now().date_naive();
        if let Some(user) = &mut current_user
            && user.last_access_date < today
        {
            if let Ok(Some(UserContinuousAccessDays {
                last_access_date,
                access_days,
                continuous_access_days,
                ..
            })) = store.get_user_access_stats(user.id).await
            {
                let continuous_access_days = if last_access_date.add(chrono::Days::new(1)) == today
                {
                    continuous_access_days + 1
                } else {
                    1
                };
                if let Err(e) = store
                    .update_user_access_stats(
                        user.id,
                        today,
                        access_days + 1,
                        continuous_access_days,
                    )
                    .await
                {
                    error!("update user access stats error: {}", e);
                }
                user.last_access_date = today;
                let _ = session.insert("CurrentUser", &user).await;

                // 触发执行用户连续登录脚本
                match state.login_rewards.login(user.id).await {
                    Ok(rewards) => {
                        info!(
                            "User ({}) login reward: credit +{}, coins +{}",
                            &user.username, rewards.credit, rewards.coins
                        );
                    }
                    Err(err) => {
                        info!(
                            "Failed to query user ({}) login reward: {}",
                            &user.username, err
                        );
                    }
                }
            }
        }

        let unread_notifications = if let Some(u) = &current_user {
            store.get_unread_count(u.id).await.unwrap_or_default()
        } else {
            0
        };
        let current_user = current_user.map(move |u| CurrentUser {
            unread_notifications,
            ..u
        });
        Ok(current_user)
    }
}

pub type OptionalCurrentUser = Option<CurrentUser>;

pub struct GlobalContext<T>(pub T);

/// Context used in askama template
#[derive(Debug, Clone)]
pub struct VContext {
    pub title: String,
    pub current_user: OptionalCurrentUser,
    pub website: Website,
    pub links: Vec<LinkCollection>,
    pub injection: InjectionConfig,
    pub timezone: chrono_tz::Tz,
    pub theme: String,
    pub language: String,
}
impl VContext {
    pub fn is_moderator(&self) -> bool {
        self.current_user
            .as_ref()
            .map(|u| u.is_moderator)
            .unwrap_or(false)
    }
    pub fn t(&self, time: &chrono::DateTime<Utc>) -> String {
        time.with_timezone(&self.timezone)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    }
    pub fn date(&self, time: &chrono::DateTime<Utc>) -> String {
        time.with_timezone(&self.timezone)
            .format("%Y-%m-%d")
            .to_string()
    }
    pub fn t_ago(&self, time: &chrono::DateTime<Utc>) -> String {
        let now = Utc::now().with_timezone(&self.timezone);
        let target = time.with_timezone(&self.timezone);

        // 计算时间差
        let duration = now.signed_duration_since(target);

        // 1. 如果是未来时间（比如时钟差异），或者超过 7 天，直接显示绝对时间
        // 你可以根据需求调整这个 7 天的阈值
        if duration.num_days() > 7 || duration.num_seconds() < 0 {
            return target.format("%Y-%m-%d %H:%M").to_string();
        }

        // 2. 获取日历日期的差异（用于判断昨天、前天）
        // 比如：昨晚 23:00 和 今天 01:00 只差 2 小时，但也属于“昨天”
        let today_date = now.date_naive();
        let target_date = target.date_naive();
        let days_diff = (today_date - target_date).num_days();

        match days_diff {
            0 => {
                // 是今天
                let secs = duration.num_seconds();
                if secs < 60 {
                    "刚刚".to_string()
                } else if secs < 3600 {
                    format!("{}分钟前", duration.num_minutes())
                } else {
                    format!("{}小时前", duration.num_hours())
                }
            }
            1 => {
                // 是昨天
                format!("昨天 {}", target.format("%H:%M"))
            }
            2 => {
                // 是前天
                format!("前天 {}", target.format("%H:%M"))
            }
            _ => {
                // 3到7天之间
                format!("{}天前", days_diff)
            }
        }
    }
}
impl LocaleProvider for VContext {
    fn locale(&self) -> &str {
        self.language.as_str()
    }
}
pub struct Context {
    pub title: String,
    pub theme: String,
    pub session: Session,
    pub current_user: OptionalCurrentUser,
    pub website: Website,
    pub links: Vec<LinkCollection>,
    pub injection: InjectionConfig,
    pub timezone: chrono_tz::Tz,
    pub language: String,
}
impl Context {
    pub fn err<T: Into<WebError>>(&self) -> impl FnOnce(T) -> WebError {
        self.err_with_title("系统异常")
    }
    pub fn err_with_title<T: Into<WebError>>(
        &self,
        title: impl AsRef<str>,
    ) -> impl FnOnce(T) -> WebError {
        let v_ctx = self.context(String::from(title.as_ref()));
        move |err| err.into().with_context(v_ctx)
    }
    pub fn get_uid(&self) -> Option<i64> {
        self.current_user.as_ref().map(|u| u.id.clone())
    }
    pub fn is_login(&self) -> bool {
        self.current_user.is_some()
    }
    pub fn is_moderator(&self) -> bool {
        self.current_user
            .as_ref()
            .map(|u| u.is_moderator)
            .unwrap_or_default()
    }
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.session.get(key).await.unwrap_or_else(|err| {
            error!("get session error: {}", err);
            None
        })
    }
    pub async fn set<T: Serialize + DeserializeOwned>(&self, key: &str, value: T) {
        if let Err(err) = self.session.insert(key, value).await {
            error!("set session error: {}", err);
        }
    }

    pub fn redirect<'a>(&self, tips: &'a str, url: &'a str) -> RedirectTemplate<'a> {
        RedirectTemplate {
            tips,
            url,
            ctx: self.context(self.title.clone()),
        }
    }
    pub fn error(&self, message: impl AsRef<str>) -> ErrorTemplate {
        ErrorTemplate::error(message.as_ref()).with_context(self.context(self.title.clone()))
    }
    pub fn info(&self, message: impl AsRef<str>) -> ErrorTemplate {
        ErrorTemplate::info(message.as_ref()).with_context(self.context(self.title.clone()))
    }
    pub fn success(&self, message: impl AsRef<str>) -> ErrorTemplate {
        ErrorTemplate::success(message.as_ref()).with_context(self.context(self.title.clone()))
    }
    pub fn not_found(&self) -> crate::home::page::NotFoundTemplate {
        crate::home::page::NotFoundTemplate {
            ctx: self.context("页面未找到".to_string()),
        }
    }
    pub fn internal_error(&self) -> crate::home::page::InternalErrorTemplate {
        crate::home::page::InternalErrorTemplate {
            ctx: self.context("服务器内部错误".to_string()),
        }
    }

    pub fn context(&self, title: String) -> VContext {
        VContext {
            title,
            current_user: self.current_user.clone(),
            website: self.website.clone(),
            links: self.links.clone(),
            injection: self.injection.clone(),
            timezone: self.timezone.clone(),
            theme: format!("theme-{}", self.theme),
            language: self.language.clone(),
        }
    }
}
pub trait LocaleProvider {
    fn locale(&self) -> &str;
}
impl<T: LocaleProvider> LocaleProvider for &T {
    fn locale(&self) -> &str {
        (*self).locale()
    }
}

impl LocaleProvider for Context {
    fn locale(&self) -> &str {
        self.language.as_str()
    }
}
impl FromRequestParts<AppState> for Context {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state).await?;
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "加载 CookieJar 失败"))?;
        let current_user = OptionalCurrentUser::from_request_parts(parts, state).await?;
        let cfg = state.cfg.get();
        let timezone = current_user
            .as_ref()
            .map(|u| u.timezone.clone())
            .unwrap_or_default();
        let theme = jar
            .get("theme")
            .map(|c| c.value())
            .unwrap_or("light")
            .to_owned();
        let language = jar.get("language").map(|c| c.value().to_string());
        let language = match language {
            Some(n) => n,
            None => {
                let headers = HeaderMap::from_request_parts(parts, state)
                    .await
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "读取 Headers 失败"))?;
                headers
                    .get("Accept-Language")
                    .and_then(|v| v.to_str().ok().map(priority_language))
                    .unwrap_or("en_US")
                    .to_string()
            }
        };
        Ok(Context {
            title: "Hello World".to_string(),
            theme,
            session,
            current_user,
            website: cfg.website,
            links: cfg.links,
            injection: cfg.injection,
            timezone,
            language,
        })
    }
}
fn priority_language(s: &str) -> &'static str {
    if s.is_empty() {
        return "en_US";
    }
    let us = min(
        s.find("en-US").unwrap_or(1000),
        s.find("en-GB").unwrap_or(1000),
    );
    let zh = min(
        s.find("zh-CN").unwrap_or(1000),
        s.find("zh-TW").unwrap_or(1000),
    );
    if zh > us { "en_US" } else { "zh_CN" }
}

#[derive(Template, TemplateResponse)]
#[template(path = "common/error.html")]
pub struct ErrorTemplate {
    pub level: &'static str,
    pub message: String,
    pub return_url: Option<String>,
    pub ctx: VContext,
}
impl ErrorTemplate {
    pub fn error(message: &str) -> ErrorTemplate {
        ErrorTemplate {
            level: "error",
            message: message.to_string(),
            return_url: None,
            ctx: VContext::default(), // Placeholder, needs with_context
        }
    }
    pub fn info(message: &str) -> ErrorTemplate {
        ErrorTemplate {
            level: "info",
            message: message.to_string(),
            return_url: None,
            ctx: VContext::default(),
        }
    }
    pub fn success(message: &str) -> ErrorTemplate {
        ErrorTemplate {
            level: "success",
            message: message.to_string(),
            return_url: None,
            ctx: VContext::default(),
        }
    }
    pub fn with_return_url(mut self, url: &str) -> Self {
        self.return_url = Some(url.to_string());
        self
    }
    pub fn with_context(mut self, ctx: VContext) -> Self {
        self.ctx = ctx;
        self
    }
}
impl Default for VContext {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            current_user: None,
            website: Default::default(),
            links: vec![],
            injection: Default::default(),
            timezone: Default::default(),
            theme: "theme-light".to_string(),
            language: "en_US".to_string(),
        }
    }
}

pub enum WebError {
    DB(Option<VContext>, sqlx::Error),
    Other(Option<VContext>, String),
}
impl WebError {
    fn with_context(self, ctx: VContext) -> WebError {
        match self {
            WebError::DB(_, err) => WebError::DB(Some(ctx), err),
            WebError::Other(_, err) => WebError::Other(Some(ctx), err),
        }
    }
}

pub type WebResponse = Result<Response, WebError>;

impl<E: Error + 'static> From<E> for WebError {
    fn from(err: E) -> Self {
        let boxed: Box<dyn Error + 'static> = Box::new(err);
        if boxed.is::<sqlx::Error>() {
            return match boxed.downcast::<sqlx::Error>() {
                Ok(sql_err) => {
                    let bt = Backtrace::capture();
                    info!("sqlx error: {:?}. Backtrace: {}", sql_err, bt);
                    WebError::DB(None, *sql_err)
                }
                Err(err) => {
                    WebError::Other(None, err.to_string())
                }
            }
        }
        WebError::Other(None, boxed.to_string())
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        match self {
            WebError::DB(ctx, err) => ErrorTemplate {
                level: "error",
                message: format!("数据库读写异常: {}", err.to_string()),
                return_url: None,
                ctx: ctx.unwrap_or_default(),
            }
            .into_response(),
            WebError::Other(ctx, err) => ErrorTemplate {
                level: "error",
                message: err,
                return_url: None,
                ctx: ctx.unwrap_or_default(),
            }
            .into_response(),
        }
    }
}
