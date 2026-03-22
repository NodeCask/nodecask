use chrono::Utc;
use crate::moderator::Moderator;
use crate::store::Store;
use crate::store::user::User;
use uuid::Uuid;

impl Store {
    /// 创建访问 token，有效期 90 天
    pub async fn get_moderator(
        &self,
        token: &str,
    ) -> sqlx::Result<Option<Moderator>> {
        sqlx::query_as::<_, Moderator>(
            r#"
            select u.id as uid,u.username from user u join access_token t on t.user_id = u.id where t.token = ? and t.expires_at > datetime('now') and t.category = 'admin'
            "#,
        )
            .bind(&token)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_bot(
        &self,
        token: &str,
    ) -> sqlx::Result<Option<Moderator>> {
        sqlx::query_as::<_, Moderator>(
            r#"
            select u.id as uid,u.username from user u join access_token t on t.user_id = u.id where t.token = ? and t.expires_at > datetime('now') and t.category = 'bot'
            "#,
        )
            .bind(&token)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_user_by_token(
        &self,
        token: &str,
        category: &str,
    ) -> sqlx::Result<Option<User>> {
        sqlx::query_as::<_, User>(
            r#"select u.id,
       u.username,
       u.password_hash,
       u.email,
       u.role,
       u.active,
       u.unread_notifications,
       u.credit_score,
       u.coins,
       u.bio,
       u.address,
       u.timezone,
       u.language,
       u.public_email,
       u.last_access_date,
       u.created_at,
       u.updated_at,
       u.avatar_timestamp
from user u
         join access_token t on t.user_id = u.id
where t.token = ?
  and t.expires_at > datetime('now')
  and t.category = ?
            "#,
        )
            .bind(token)
            .bind(category)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_access_token_with_days(
        &self,
        user_id: i64,
        description: &str,
        category: &str,
        days: i64,
    ) -> sqlx::Result<String> {
        let token = Uuid::new_v4().to_string();
        let expires_modifier = format!("+{} days", days);

        let _ = sqlx::query(
            r#"
            INSERT INTO access_token (token, description, created_at, expires_at, user_id, category)
            VALUES (?, ?, datetime('now'), datetime('now', ?), ?, ?)
            "#,
        )
        .bind(&token)
        .bind(description)
        .bind(&expires_modifier)
        .bind(user_id)
        .bind(category)
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    pub async fn create_access_token(
        &self,
        user_id: i64,
        description: &str,
        category: &str,
    ) -> sqlx::Result<String> {
        let token = Uuid::new_v4().to_string();

        // 计算过期时间：当前时间 + 90 天
        let _ = sqlx::query(
            r#"
            INSERT INTO access_token (token, description, created_at, expires_at, user_id, category)
            VALUES (?, ?, datetime('now'), datetime('now', '+90 days'), ?, ?)
            "#,
        )
        .bind(&token)
        .bind(description)
        .bind(user_id)
        .bind(category)
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    pub async fn list_access_tokens(
        &self,
        category: &str,
    ) -> sqlx::Result<Vec<AccessToken>> {
        sqlx::query_as::<_, AccessToken>(
            r#"
            select t.*, u.username from access_token t join user u on t.user_id = u.id where t.category = ? order by t.created_at desc
            "#,
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_access_token(&self, token: &str) -> sqlx::Result<()> {
        sqlx::query("delete from access_token where token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct AccessToken {
    pub token: String,
    pub description: String,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub user_id: i64,
    pub category: String,
    pub username: String,
}