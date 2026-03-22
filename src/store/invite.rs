use crate::store::{Pagination, PaginationQuery, Store};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InviteCode {
    pub id: i64,
    pub code: String,
    pub quota: i32,
    pub used_count: i32,
    pub created_at: DateTime<Utc>,
    pub expired_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InviteUsageLog {
    pub id: i64,
    pub invitation_id: i64,
    pub user_id: i64,
    pub username: String, // Joined from users table
    pub used_at: DateTime<Utc>,
}

impl Store {
    pub async fn create_invite_codes(&self, count: usize, quota: i32, expired_at:Option<DateTime<Utc>>) -> sqlx::Result<Vec<String>> {
        let mut codes = Vec::new();
        for _ in 0..count {
            let code = Uuid::new_v4().to_string();
            
            sqlx::query("INSERT INTO invitation_code (code, quota, expired_at) VALUES (?, ?, ?)")
                .bind(&code)
                .bind(quota)
                .bind(expired_at)
                .execute(&self.pool)
                .await?;
            codes.push(code);
        }
        Ok(codes)
    }

    pub async fn get_invite_codes(&self, page: PaginationQuery) -> sqlx::Result<Pagination<InviteCode>> {
        let total: (u32,) = sqlx::query_as("SELECT count(*) FROM invitation_code")
            .fetch_one(&self.pool)
            .await?;
            
        let data = sqlx::query_as::<_, InviteCode>(
            "SELECT * FROM invitation_code ORDER BY created_at DESC LIMIT ?, ?"
        )
        .bind(page.start())
        .bind(page.size())
        .fetch_all(&self.pool)
        .await?;
        
        Ok(Pagination::new(page, total.0, data))
    }
    
    pub async fn delete_invite_code(&self, id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM invitation_code WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    
    pub async fn get_invite_usage_logs(&self, invitation_id: i64) -> sqlx::Result<Vec<InviteUsageLog>> {
        let logs = sqlx::query_as::<_, InviteUsageLog>(
            r#"
            SELECT l.id, l.invitation_id, l.user_id, l.used_at, u.username
            FROM invitation_usage l
            JOIN user u ON l.user_id = u.id
            WHERE l.invitation_id = ?
            ORDER BY l.used_at DESC
            "#
        )
        .bind(invitation_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(logs)
    }
    
}