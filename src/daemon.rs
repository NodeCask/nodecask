use std::sync::Arc;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use object_store::ObjectStore;
use crate::common::{AppState, GlobalContext};
use crate::daemon::email::EmailSenderDaemon;
use crate::daemon::link_filter::LinkFilter;
use crate::daemon::moderate::ModerateDaemon;
use crate::daemon::notify::NotifyDaemon;
use crate::daemon::nsfw_detect::NSFWDetector;
use crate::daemon::tantivy::SearchDaemon;

pub mod moderate;
pub mod config;
pub mod notify;
pub(crate) mod email;
pub(crate) mod link_filter;
pub(crate) mod login_rewards;
pub(crate) mod tantivy;
pub(crate) mod nsfw_detect;

impl FromRequestParts<AppState> for GlobalContext<Arc<Box<dyn ObjectStore>>> {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(GlobalContext(state.fs.clone()))
    }
}
impl FromRequestParts<AppState> for GlobalContext<ModerateDaemon> {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(GlobalContext(state.moderator.clone()))
    }
}
impl FromRequestParts<AppState> for GlobalContext<LinkFilter> {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(GlobalContext(state.link_filter.clone()))
    }
}
impl FromRequestParts<AppState> for GlobalContext<NotifyDaemon> {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(GlobalContext(state.notify.clone()))
    }
}
impl FromRequestParts<AppState> for GlobalContext<EmailSenderDaemon> {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(GlobalContext(state.email.clone()))
    }
}
impl FromRequestParts<AppState> for GlobalContext<SearchDaemon> {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(GlobalContext(state.searcher.clone()))
    }
}
impl FromRequestParts<AppState> for GlobalContext<NSFWDetector> {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(GlobalContext(state.nsfw_detector.clone()))
    }
}
