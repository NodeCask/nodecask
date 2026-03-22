use crate::store::{Pagination, PaginationQuery, Store};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Notification {
    pub id: i64,
    pub user_id: i64,
    pub created_at: DateTime<Utc>,
    pub category: String,
    pub link_id: i64,
    pub content: String,
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationSearch {
    pub user_id: i64,
    pub category: Option<String>,
}

impl Store {
    pub async fn add_notification(
        &self,
        user_id: i64,
        category: &str,
        link_id: i64,
        content: &str,
        meta: Option<String>,
    ) -> sqlx::Result<i64> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            r#"INSERT INTO notification (user_id, category, link_id, content, meta, created_at)
               VALUES (?, ?, ?, ?, ?, datetime('now'))"#,
        )
        .bind(user_id)
        .bind(category)
        .bind(link_id)
        .bind(content)
        .bind(meta)
        .execute(&mut *tx)
        .await?;
        let _ = sqlx::query(
            r#"update user set unread_notifications = unread_notifications + 1 where id = ?"#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_notifications(
        &self,
        search: &NotificationSearch,
        page: PaginationQuery,
    ) -> sqlx::Result<Pagination<Notification>> {
        let total: (u32,) = sqlx::query_as(
            r#"SELECT COUNT(*)
               FROM notification
               WHERE user_id = ?
               AND (? IS NULL OR category = ?)
               "#,
        )
        .bind(search.user_id)
        .bind(&search.category)
        .bind(&search.category)
        .fetch_one(&self.pool)
        .await?;

        let data: Vec<Notification> = sqlx::query_as::<_, Notification>(
            r#"SELECT id, user_id, created_at, category, link_id, content, meta
               FROM notification
               WHERE user_id = ?
               AND (? IS NULL OR category = ?)
               ORDER BY created_at DESC
               LIMIT ?, 100"#,
        )
        .bind(search.user_id)
        .bind(&search.category)
        .bind(&search.category)
        .bind(page.start())
        .fetch_all(&self.pool)
        .await?;

        let total = total.0;
        let per_page = 100;
        let total_page = if total % per_page == 0 {
            total / per_page
        } else {
            (total / per_page) + 1
        };

        Ok(Pagination {
            data,
            total,
            per_page,
            total_page,
            current_page: page.current(),
        })
    }

    pub async fn get_unread_count(&self, user_id: i64) -> sqlx::Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"SELECT unread_notifications
               FROM user
               WHERE id = ?"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count.0)
    }

    pub async fn mark_all_as_read(&self, user_id: i64) -> sqlx::Result<u64> {
        let result = sqlx::query(
            r#"UPDATE user SET unread_notifications = 0
               WHERE id = ?"#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_all_notifications(&self, user_id: i64) -> sqlx::Result<u64> {
        let result = sqlx::query(r#"DELETE FROM notification WHERE user_id = ?"#)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}