use crate::store::{Pagination, PaginationQuery};
use crate::store::Store;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt::{Display, Formatter};

// 会员连续登录信息
#[derive(Debug, Clone, FromRow, Default, Serialize, Deserialize)]
pub struct UserContinuousAccessDays {
    pub last_access_date: chrono::NaiveDate,
    pub access_days: i64,
    pub continuous_access_days: i64,
    pub last_checkin_date: chrono::NaiveDate,
    pub checkin_days: i64,
    pub continuous_checkin_days: i64,
}
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserIndex {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub active: bool,
    pub credit_score: i64,
    pub coins: i64,
    pub topics: u64,
    pub replies: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_access_date: chrono::NaiveDate,
    pub avatar_timestamp: i64,
}
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub email: String,
    pub role: String,
    pub active: bool,
    pub unread_notifications: i64,
    pub credit_score: i64,
    pub coins: i64,
    pub bio: String,
    pub address: String,
    pub timezone: String,
    pub language: String,
    pub public_email: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_access_date: chrono::NaiveDate,
    pub avatar_timestamp: i64,
}

#[derive(Debug, FromRow)]
pub struct UserDetail {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub bio: String,
    pub address: String,
    pub timezone: String,
    pub language: String,
    pub public_email: bool,
    pub topics: u64,
    pub replies: u64,
    pub coins: i64,
    pub credit_score: i64,
    pub avatar_timestamp: i64,
    pub created_at: DateTime<Utc>,
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(uid: {})", self.username, self.id)
    }
}
impl Store {
    pub async fn search_users(
        &self,
        username: Option<String>,
        active: Option<bool>,
        role: Option<String>,
        page: PaginationQuery,
    ) -> sqlx::Result<Pagination<UserIndex>> {
        // language=sql
        let total: u32 = sqlx::query_scalar(
            r#"
       SELECT count(*) FROM user
       where (? is null or username like '%'||?||'%' )
       and (? is null or active = ?)
       and (? is null or role = ?)
        "#,
        )
            .bind(username.clone())
            .bind(username.clone())
            .bind(active)
            .bind(active)
            .bind(role.clone())
            .bind(role.clone())
            .fetch_one(&self.pool)
            .await?;

        let users = sqlx::query_as::<_, UserIndex>(
            r#"
       SELECT id, username, email, role, active, credit_score, coins, last_access_date, created_at, updated_at, avatar_timestamp,
                (select count(*) from topic where user_id = user.id) as topics,
                (select count(*) from comment where user_id = user.id) as replies
       FROM user
       where (? is null or username like '%'||?||'%' )
       and (? is null or active = ?)
       and (? is null or role = ?)
        ORDER BY created_at DESC
       limit ?,?
        "#,
        )
        .bind(username.clone())
        .bind(username.clone())
        .bind(active)
        .bind(active)
        .bind(role.clone())
        .bind(role.clone())
        .bind(page.start())
        .bind(page.size())
        .fetch_all(&self.pool)
        .await?;

        Ok(Pagination::new(page, total, users))
    }

    pub async fn update_user_status(&self, id: i64, active: bool) -> sqlx::Result<u64> {
        let result = sqlx::query("UPDATE user SET active = ? WHERE id = ?")
            .bind(active)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn get_user(&self, username: &str) -> sqlx::Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT id, username, password_hash, email, role, active, unread_notifications, credit_score, coins, bio, address, timezone, language, public_email, last_access_date, created_at, updated_at, avatar_timestamp FROM user WHERE username = ?")
            .bind(username.to_string())
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_user_by_email(&self, email: &str) -> sqlx::Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT id, username, password_hash, email, role, active, unread_notifications, credit_score, coins, bio, address, timezone, language, public_email, last_access_date, created_at, updated_at, avatar_timestamp FROM user WHERE email = ?")
            .bind(email.to_string())
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn check_email_exists(&self, email: &str) -> sqlx::Result<bool> {
        let n: bool = sqlx::query_scalar(r#" select count(*) from user where email = ? "#)
            .bind(email)
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }

    pub async fn check_username_exists(&self, username: &str) -> sqlx::Result<bool> {
        let n: (bool,) = sqlx::query_as(r#" select count(*) from user where username = ? "#)
            .bind(username)
            .fetch_one(&self.pool)
            .await?;
        Ok(n.0)
    }

    pub async fn get_user_count(&self) -> sqlx::Result<i64> {
        let n: (i64,) = sqlx::query_as("SELECT count(*) FROM user")
            .fetch_one(&self.pool)
            .await?;
        Ok(n.0)
    }

    pub async fn valid_invite_code(
        &self,
        invite_code: &str,
    ) -> sqlx::Result<bool> {
        // language=sql
        let invite: Option<bool> = sqlx::query_scalar(
            "SELECT used_count < quota FROM invitation_code WHERE code = ? ",
        )
            .bind(invite_code)
            .fetch_optional(&self.pool)
            .await?;
        Ok(invite.unwrap_or(false))
    }
    pub async fn create_user_with_invite(
        &self,
        username: &str,
        password_hash: &str,
        email: &str,
        language: &str,
        initial_score: i64,
        invite_code: Option<String>,
    ) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;

        let invite_id = if let Some(invite_code) = &invite_code {
            // language=sql
            let invite: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM invitation_code WHERE code = ? AND used_count < quota",
            )
            .bind(invite_code)
            .fetch_optional(&mut *tx)
            .await?;

            match invite {
                Some(id) => Some(id),
                None => return Err(sqlx::Error::RowNotFound),
            }
        } else {
            None
        };

        // 2. Create user
        let user_id = sqlx::query(r#"INSERT INTO user (username, password_hash, email,language, role, credit_score ) VALUES (?, ?, ?, ?, 'user', ? )"#)
            .bind(username)
            .bind(password_hash)
            .bind(email)
            .bind(language)
            .bind(initial_score)
            .execute(&mut *tx)
            .await?
            .last_insert_rowid();

        // 更新最后访问日期和最后打卡日期，触发用户登录奖励
        let yesterday = Utc::now().date_naive() - chrono::Duration::days(2);
        sqlx::query("UPDATE user SET last_access_date= ? , last_checkin_date= ? WHERE id = ?")
            .bind(&yesterday)
            .bind(&yesterday)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        if let Some(invite_id) = invite_id {
            // 3. Update invite code usage
            sqlx::query("UPDATE invitation_code SET used_count = used_count + 1 WHERE id = ?")
                .bind(invite_id)
                .execute(&mut *tx)
                .await?;

            // 4. Log usage
            sqlx::query("INSERT INTO invitation_usage (invitation_id, user_id) VALUES (?, ?)")
                .bind(invite_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_user_email(&self, id: i64, email: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE user SET email = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(email)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_totp_secret(&self, id: i64) -> sqlx::Result<Option<String>> {
        // language=sql
        let res:Option<String> = sqlx::query_scalar("select totp_secret from user WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(res.filter(|s| !s.is_empty()))
    }
    pub async fn update_totp_secret(&self, id: i64, secret: Option<String>) -> sqlx::Result<()> {
        sqlx::query("UPDATE user SET totp_secret = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(secret)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_user_role(&self, id: i64, role: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE user SET role = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(role)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_user_profile(
        &self,
        id: i64,
        bio: String,
        address: String,
        timezone: String,
        language: String,
        public_email: bool,
    ) -> sqlx::Result<()> {
        sqlx::query("UPDATE user SET bio = ?, address = ?, timezone = ?, language = ?, public_email = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(bio)
            .bind(address)
            .bind(timezone)
            .bind(language)
            .bind(public_email)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_user_avatar_data(&self, username: &str) -> sqlx::Result<Option<Vec<u8>>> {
        // language=sql
        let result: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT a.data FROM user_avatar a join  user u on a.id = u.id WHERE u.username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result)
    }

    pub async fn update_user_avatar(&self, id: i64, data: Vec<u8>) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;
        let query = r#"
                INSERT INTO user_avatar (id, data, update_at)
                VALUES (?, ?, datetime('now'))
                ON CONFLICT(id) DO UPDATE SET data = excluded.data, update_at = excluded.update_at
            "#;
        sqlx::query(query)
            .bind(id)
            .bind(data)
            .execute(&mut *tx)
            .await?;

        let timestamp = Utc::now().timestamp();
        sqlx::query("UPDATE user SET avatar_timestamp = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(timestamp)
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_user_access_stats(
        &self,
        id: i64,
    ) -> sqlx::Result<Option<UserContinuousAccessDays>> {
        let result = sqlx::query_as::<_, UserContinuousAccessDays>(
            r#"select last_access_date,
       access_days,
       continuous_access_days,
       last_checkin_date,
       checkin_days,
       continuous_checkin_days
from user
WHERE id = ?"#,
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(result)
    }
    pub async fn update_user_access_stats(
        &self,
        id: i64,
        last_access_date: chrono::NaiveDate,
        access_days: i64,
        continuous_access_days: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE user SET last_access_date = ?, access_days = ?, continuous_access_days = ? WHERE id = ?",
        )
        .bind(last_access_date)
        .bind(access_days)
        .bind(continuous_access_days)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_user_check_stats(
        &self,
        id: i64,
        last_check_date: chrono::NaiveDate,
        check_days: i64,
        continuous_check_days: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE user SET last_checkin_date = ?, checkin_days = ?, continuous_checkin_days = ? WHERE id = ?",
        )
            .bind(last_check_date)
            .bind(check_days)
            .bind(continuous_check_days)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_user_detail(&self, id: i64) -> sqlx::Result<UserDetail> {
        let user_detail = sqlx::query_as::<_, UserDetail>(
            r#"
            SELECT
                id,username,email,bio,address,timezone,language,public_email,created_at,
                (select count(*) from topic where user_id = user.id) as topics,
                (select count(*) from comment where user_id = user.id) as replies,
                coins,credit_score,avatar_timestamp
            FROM user
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(user_detail)
    }

    pub async fn get_user_attr<T: DeserializeOwned>(&self, id: i64, key: &str) -> sqlx::Result<Option<T>> {
        if key.is_empty() {
            return Ok(None);
        }
        let res = sqlx::query_as::<_, (String,)>(
            r#"
        SELECT value
        FROM user_attribute
        WHERE id = ? and attr = ?
    "#,
        )
        .bind(id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        
        match res {
            Some((v,)) => {
                match serde_json::from_str::<T>(v.as_ref()) {
                    Ok(t) => Ok(Some(t)),
                    Err(e) => Err(sqlx::Error::Protocol(format!("serde error: {}", e))),
                }
            }
            None => Ok(None)
        }
    }
    pub async fn set_user_attr<T: Serialize>(
        &self,
        id: i64,
        key: &str,
        value: Option<&T>,
    ) -> sqlx::Result<()> {
        match value {
            Some(value) => {
                let value = serde_json::to_string(value).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
                let _ = sqlx::query(
                    r#"INSERT INTO user_attribute (id, attr, value) VALUES (?, ?, ?) ON CONFLICT (id, attr) DO UPDATE SET value = ? "#,
                )
                    .bind(id)
                    .bind(key)
                    .bind(&value)
                    .bind(&value)
                    .execute(&self.pool)
                    .await?;
            }
            None => {
                let _ = sqlx::query(r#" delete FROM user_attribute WHERE id = ? and attr = ? "#)
                    .bind(id)
                    .bind(key)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn user_password_reset(&self, id: i64, password: &str) -> sqlx::Result<()> {
        let result = sqlx::query(r#"update user set password_hash = ? where id = ?"#)
            .bind(password)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 1 {
            return Ok(());
        }
        Err(sqlx::Error::RowNotFound)
    }

    pub async fn get_relation(&self, user_id: i64, target_id: i64) -> sqlx::Result<Option<String>> {
        let res: Option<(String,)> = sqlx::query_as(
            "SELECT relation FROM user_relation WHERE user_id = ? AND target_id = ?",
        )
        .bind(user_id)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(res.map(|(r,)| r))
    }

    pub async fn follow_user(&self, user_id: i64, target_id: i64) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO user_relation (user_id, target_id, relation) VALUES (?, ?, 'follow') ON CONFLICT(user_id, target_id) DO UPDATE SET relation = 'follow'")
            .bind(user_id)
            .bind(target_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn unfollow_user(&self, user_id: i64, target_id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM user_relation WHERE user_id = ? AND target_id = ? AND relation = 'follow'")
            .bind(user_id)
            .bind(target_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn block_user(&self, user_id: i64, target_id: i64) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO user_relation (user_id, target_id, relation) VALUES (?, ?, 'block') ON CONFLICT(user_id, target_id) DO UPDATE SET relation = 'block'")
            .bind(user_id)
            .bind(target_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn unblock_user(&self, user_id: i64, target_id: i64) -> sqlx::Result<()> {
        sqlx::query(
            "DELETE FROM user_relation WHERE user_id = ? AND target_id = ? AND relation = 'block'",
        )
        .bind(user_id)
        .bind(target_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn _get_blocked_user_ids(&self, user_id: i64) -> sqlx::Result<Vec<i64>> {
        let ids = sqlx::query_as::<_, (i64,)>(
            "SELECT target_id FROM user_relation WHERE user_id = ? AND relation = 'block'",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(id,)| id)
        .collect();
        Ok(ids)
    }

    pub async fn update_credit_and_coins(&self, user_id: i64, credit: i64, coins: i64) -> sqlx::Result<()> {
        sqlx::query("UPDATE user SET credit_score = credit_score + ?, coins = coins + ? WHERE id = ?")
            .bind(credit)
            .bind(coins)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn update_coins(&self, user_id: i64, amount: i64) -> sqlx::Result<()> {
        sqlx::query("UPDATE user SET coins = coins + ? WHERE id = ?")
            .bind(amount)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn update_credit(&self, user_id: i64, amount: i64) -> sqlx::Result<()> {
        sqlx::query("UPDATE user SET credit_score = credit_score + ? WHERE id = ?")
            .bind(amount)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn user_login_rewards(&self, uid: i64, category: &str, credit: i64, coins: i64) -> sqlx::Result<i64> {
        let today = Utc::now().date_naive();
        let result = sqlx::query(r#"INSERT INTO user_login_rewards (date,user_id, category, credit, coins)
VALUES (?, ?, ?, ?,?)"#)
            .bind(&today)
            .bind(uid)
            .bind(category)
            .bind(credit)
            .bind(coins)
            .execute(&self.pool)
            .await?;
        Ok(result.last_insert_rowid())
    }
}
