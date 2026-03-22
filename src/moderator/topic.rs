use crate::common::AppState;
use crate::moderator::{Data, ModContext as Context, PageExtractor};
use crate::store::Store;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Deserialize)]
pub struct TopicSearch {
    pub query: Option<String>,
    pub node_id: Option<i64>,
}

pub async fn analysis(
    store: Store,
    State(_state): State<AppState>,
    Query(_search): Query<TopicSearch>,
) -> impl IntoResponse {
    #[derive(FromRow, Serialize)]
    struct Analysis {
        total_topics: i64,
        total_comments: i64,
        new_topics: i64,
        new_comments: i64,
    }
    let result = sqlx::query_as::<_, Analysis>(
        r#"
SELECT
  (SELECT count(*) FROM topic) AS total_topics,
  (SELECT count(*) FROM comment) AS total_comments,
  (SELECT count(*) FROM topic WHERE created_at >= datetime('now', '-1 day')) AS new_topics,
  (SELECT count(*) FROM comment WHERE created_at >= datetime('now', '-1 day')) AS new_comments;
    "#,
    )
    .fetch_one(&store.pool)
    .await;
    match result {
        Ok(data) => Data::ok(data),
        Err(err) => Data::error(&format!("系统异常:{}", err.to_string())),
    }
}
#[axum::debug_handler(state = AppState)]
pub async fn list(
    _ctx: Context,
    store: Store,
    State(_state): State<AppState>,
    PageExtractor(page): PageExtractor,
    Query(search): Query<TopicSearch>,
) -> impl IntoResponse {
    match store
        .topics(
            &crate::store::topic::TopicSearch {
                sort: Some("latest".to_string()),
                node_id: search.node_id,
                q: search.query,
                ..Default::default()
            },
            page,
        )
        .await
    {
        Ok(result) => Data::ok(result).into_response(),
        Err(err) => Data::fail(&format!("系统异常:{}", err.to_string())).into_response(),
    }
}

#[derive(Deserialize)]
pub struct TopicUpdateForm {
    pub action: String,
}

#[axum::debug_handler(state = AppState)]
pub async fn update(
    _ctx: Context,
    _store: Store,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(form): Json<TopicUpdateForm>,
) -> impl IntoResponse {
    let result = match form.action.as_str() {
        "pin" => {
            sqlx::query("UPDATE topic SET is_pinned = 1 WHERE id = ?")
                .bind(id)
                .execute(&state.pool)
                .await
        }
        "unpin" => {
            sqlx::query("UPDATE topic SET is_pinned = 0 WHERE id = ?")
                .bind(id)
                .execute(&state.pool)
                .await
        }
        "lock" => {
            sqlx::query("UPDATE topic SET is_locked = 1 WHERE id = ?")
                .bind(id)
                .execute(&state.pool)
                .await
        }
        "unlock" => {
            sqlx::query("UPDATE topic SET is_locked = 0 WHERE id = ?")
                .bind(id)
                .execute(&state.pool)
                .await
        }
        _ => return Data::fail("无效的操作").into_response(),
    };

    match result {
        Ok(r) => {
            if r.rows_affected() == 0 {
                Data::fail("帖子未找到").into_response()
            } else {
                Data::done().into_response()
            }
        }
        Err(e) => Data::fail(&format!("更新失败: {}", e)).into_response(),
    }
}

#[axum::debug_handler(state = AppState)]
pub async fn delete(
    _ctx: Context,
    _store: Store,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM topic WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) => {
            if r.rows_affected() == 0 {
                Data::fail("帖子未找到").into_response()
            } else {
                Data::done().into_response()
            }
        }
        Err(e) => Data::fail(&format!("删除失败: {}", e)).into_response(),
    }
}
