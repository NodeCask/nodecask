use crate::store::{Pagination, PaginationQuery, Store};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EmailIndex {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub user_id: i64,
    pub email_from: String,
    pub email_to: String,
    pub email_subject: String,
    pub result: String,
}
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EmailDetail {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub user_id: i64,
    pub email_from: String,
    pub email_to: String,
    pub email_subject: String,
    pub email_body: String,
    pub result: String,
}

impl Store {
    pub async fn list_emails(&self, q: PaginationQuery) -> sqlx::Result<Pagination<EmailIndex>> {
        let total: i64 = sqlx::query_scalar("SELECT count(*) FROM email_queue")
            .fetch_one(&self.pool)
            .await?;

        let data = sqlx::query_as::<_, EmailIndex>(
            "SELECT id, created_at, user_id, email_from, email_to, email_subject, result FROM email_queue ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(q.size())
        .bind(q.start())
        .fetch_all(&self.pool)
        .await?;

        Ok(Pagination::new(q, total as u32, data))
    }

    pub async fn get_email(&self, id: i64) -> sqlx::Result<Option<EmailDetail>> {
        sqlx::query_as::<_, EmailDetail>(
            r#"SELECT id,
       created_at,
       user_id,
       email_from,
       email_to,
       email_subject,
       email_body,
       result
FROM email_queue
WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn add_email_queue(
        &self,
        user_id: i64,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> sqlx::Result<i64> {
        let id = sqlx::query(
            r#"
            INSERT INTO email_queue (created_at, user_id, email_from, email_to, email_subject, email_body, result)
            VALUES (?, ?, ?, ?, ?, ?, 'pending')
            RETURNING id
            "#,
        ).bind(Utc::now())
        .bind(user_id)
        .bind(from)
        .bind(to)
        .bind(subject)
        .bind(body)
        .execute(&self.pool)
        .await?;
        Ok(id.last_insert_rowid())
    }

    pub async fn mark_email_done(&self, id: i64, from: &str, state: &str) -> sqlx::Result<()> {
        let _ = sqlx::query(r#"update email_queue set email_from = ?, result = ? where id = ?"#)
            .bind(from)
            .bind(state)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn count_by_email(&self, to: &str, since: DateTime<Utc>) -> sqlx::Result<i64> {
        // language=sql
        let n = sqlx::query_scalar(
            r#"select count(*) from email_queue where result != 'pending' and email_to = ? and created_at > ?"#)
            .bind(to)
            .bind(since)
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }
}
