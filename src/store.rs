pub(crate) mod access_token;
pub(crate) mod email;
pub(crate) mod invite;
pub(crate) mod link;
pub(crate) mod node;
pub(crate) mod notifications;
mod page;
pub(crate) mod system;
pub(crate) mod topic;
pub(crate) mod user;

use askama::Template;
use crate::common::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct Store {
    pub(crate) pool: SqlitePool,
}

impl FromRequestParts<AppState> for Store {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> std::result::Result<Self, Self::Rejection> {
        Ok(state.store())
    }
}

#[derive(Debug, Deserialize)]
pub struct Page {
    pub p: u32,
    pub topics_per_page: u32,
    pub comment_per_page: u32,
}
impl Page {
    pub fn list(&self) -> PaginationQuery {
        PaginationQuery {
            p: self.p,
            per_page: self.topics_per_page,
        }
    }
    pub fn topic(&self) -> PaginationQuery {
        PaginationQuery {
            p: self.p,
            per_page: self.topics_per_page,
        }
    }
    pub fn comment(&self) -> PaginationQuery {
        PaginationQuery {
            p: self.p,
            per_page: self.comment_per_page,
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub p: u32,
    pub per_page: u32,
}
impl FromRequestParts<AppState> for Page {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 获取 query 字符串，如果没有 query 则为空字符串
        let query_str = parts.uri.query().unwrap_or("");

        #[derive(Deserialize)]
        struct PageParams{
            p:Option<u32>
        }
        let params: PageParams = serde_urlencoded::from_str(query_str)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid query params: {}", e)))?;

        let p = params.p.unwrap_or(1);
        let cfg = state.cfg.load();
        let topics_per_page = cfg.visit.topics_per_page;
        let comment_per_page= cfg.visit.comments_per_page;
        Ok(Page{
            p,
            topics_per_page,
            comment_per_page,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    after: Option<chrono::DateTime<Utc>>,
    before: Option<chrono::DateTime<Utc>>,
    amount: i64,
}
impl Default for RangeQuery {
    fn default() -> Self {
        Self { after: None, before: Some(chrono::Utc::now()), amount: 20 }
    }
}
impl FromRequestParts<AppState> for RangeQuery {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 获取 query 字符串，如果没有 query 则为空字符串
        let query_str = parts.uri.query().unwrap_or("");

        #[derive(Deserialize)]
        struct Params{
            after:Option<i64>,
            before:Option<i64>,
            amount:Option<i64>
        }
        let params: Params = serde_urlencoded::from_str(query_str)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid query params: {}", e)))?;

        let after = params.after
            .and_then(|t|chrono::DateTime::from_timestamp(t,0));
        let before = params.before
            .and_then(|t|chrono::DateTime::from_timestamp(t,0));
        let amount = params.amount.unwrap_or(20);
        Ok(RangeQuery {after, before, amount })
    }
}

impl From<u32> for PaginationQuery {
    fn from(v: u32) -> Self {
        Self {
            p: v,
            per_page: 20,
        }
    }
}
impl PaginationQuery {
    pub fn start(&self) -> u32 {
        (self.p - 1) * self.size()
    }
    pub fn current(&self) -> u32 {
        self.p
    }
    pub fn size(&self) -> u32 {
        self.per_page
    }
}
#[derive(Debug, serde::Serialize)]
pub struct Pagination<T> {
    pub data: Vec<T>,
    pub total: u32,
    pub per_page: u32,
    pub total_page: u32,
    pub current_page: u32,
}
impl<T> Default for Pagination<T> {
    fn default() -> Self {
        Self{
            data: vec![],
            total: 0,
            per_page: 20,
            total_page: 1,
            current_page: 1,
        }
    }
}
impl<T> Pagination<T> {
    pub fn new(page: PaginationQuery, total: u32, data: Vec<T>) -> Self {
        let per_page = page.size();
        let current_page = page.current();
        let total_page = if total % per_page == 0 {
            total / per_page
        } else {
            (total / per_page) + 1
        };
        Self {
            data,
            total,
            total_page,
            per_page,
            current_page,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
    pub fn buttons(&self) -> Vec<PageButton> {
        generate_pagination(self.current_page, self.total_page)
    }
    pub fn render(&self) -> String {
        let result = PaginationTemplate{buttons:self.buttons()}.render();
        match result {
            Ok(html) => {html}
            Err(err) => {
                format!("An error occurred during the rendering of the pagination: {}", err)
            }
        }
    }
}
pub enum PageButton {
    Link(u32),
    Current(u32),
    Span,
}

#[derive(Template)]
#[template(path = "common/pagination.html")]
struct PaginationTemplate {
    buttons: Vec<PageButton>,
}

// 生成分页效果：[1] ... [3] [4] [5] [6] *7* [8] [9] [10] [11] ... [100]
fn generate_pagination(current: u32, total: u32) -> Vec<PageButton> {
    // 边界检查：如果没有页面，返回空
    if total == 0 {
        return vec![];
    }

    // 确保 current 不会超过 total 或为 0
    let current = current.clamp(1, total);

    // 1. 收集需要显示的页码
    let mut pages = Vec::new();

    // 规则：总是显示第一页
    pages.push(1);

    // 规则：总是显示最后一页
    pages.push(total);

    // 规则：显示当前页面前后各 4 页
    // 使用 saturating_sub 防止 usize 溢出（虽然这里是 u32，但保持逻辑安全）
    let start = if current > 4 { current - 4 } else { 1 };
    let end = if current + 4 < total { current + 4 } else { total };

    for i in start..=end {
        pages.push(i);
    }

    // 2. 排序并去重
    pages.sort_unstable();
    pages.dedup();

    // 3. 构建 Button 数组并插入 Span
    let mut buttons = Vec::new();
    let mut prev = 0; // 用于记录上一个插入的页码

    for &p in &pages {
        // 如果这不是第一个元素，且与上一个元素不连续
        if prev > 0 && p - prev > 1 {
            buttons.push(PageButton::Span);
        }

        if p == current {
            buttons.push(PageButton::Current(p));
        } else {
            buttons.push(PageButton::Link(p));
        }

        prev = p;
    }

    buttons
}