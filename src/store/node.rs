use crate::store::Store;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub show_in_list: bool,
    pub member_access_required: bool,
    pub moderator_access_required: bool,
    pub isolated: bool,
    pub access_only: bool,
    pub topic_reward: i64,
    pub comment_reward: i64,
    pub topic_count: i64,
    pub created_at: DateTime<Utc>,
    pub attributes: sqlx::types::Json<Vec<(String, String)>>,
}
impl Node {
    pub fn url(&self) -> String {
        format!("/go/{}", self.slug)
    }
    pub fn attr(&self, key: &str) -> Option<String> {
        self.attributes.iter().find(|(k, _)| k == key)
            .filter(|(_, v)| !v.is_empty())
            .map(|(_, v)| v.clone())
    }
    pub fn is_enabled(&self, key: &str) -> bool {
        self.attributes.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| ["1", "y", "yes", "true"].contains(&v.as_str()))
            .unwrap_or_default()
    }
}
impl Store {
    pub async fn get_node(&self, slug: &str) -> sqlx::Result<Option<Node>> {
        sqlx::query_as::<_, Node>(
            r#"SELECT id,
       name,
       slug,
       description,
       created_at,
       show_in_list,
       member_access_required,
       moderator_access_required,
       topic_reward,
       comment_reward,
       isolated,
       access_only,
       topic_count,
       attributes
FROM node
WHERE slug = ?
limit 1"#,
        )
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
    }
    pub async fn get_node_by_id(&self, id: i64) -> sqlx::Result<Option<Node>> {
        sqlx::query_as::<_, Node>(
            r#"SELECT id,
       name,
       slug,
       description,
       created_at,
       show_in_list,
       member_access_required,
       moderator_access_required,
       topic_reward,
       comment_reward,
       isolated,
       access_only,
       topic_count,
       attributes
FROM node
WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_nodes(&self) -> sqlx::Result<Vec<Node>> {
        sqlx::query_as::<_, Node>("SELECT * FROM node ORDER BY created_at")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn exist_node_slug(&self, slug: &str) -> sqlx::Result<bool> {
        let row: bool = sqlx::query_scalar("select count(*) from node where slug = ?")
            .bind(slug)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn create_node(
        &self,
        name: &str,
        slug: &str,
        description: &str,
        show_in_list: bool,
        member_access_required: bool,
        moderator_access_required: bool,
        isolated: bool,
        access_only: bool,
        topic_reward: i64,
        comment_reward: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"INSERT INTO node (name, slug, description,
                   show_in_list, member_access_required, moderator_access_required, isolated, access_only, topic_reward, comment_reward, created_at)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))"#,
        )
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(show_in_list)
        .bind(member_access_required)
        .bind(moderator_access_required)
        .bind(isolated)
        .bind(access_only)
        .bind(topic_reward)
        .bind(comment_reward)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_node(
        &self,
        id: i64,
        name: &str,
        slug: &str,
        description: &str,
        show_in_list: bool,
        member_access_required: bool,
        moderator_access_required: bool,
        isolated: bool,
        access_only: bool,
        topic_reward: i64,
        comment_reward: i64,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query(
            r#"update node
set name             = ?,
    slug             = ?,
    description      = ?,
    show_in_list     = ?,
    member_access_required = ?,
    moderator_access_required = ?,
    isolated = ?,
    access_only = ?,
    topic_reward = ?,
    comment_reward = ?
where id = ?
    "#,
        )
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(show_in_list)
        .bind(member_access_required)
        .bind(moderator_access_required)
        .bind(isolated)
        .bind(access_only)
        .bind(topic_reward)
        .bind(comment_reward)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn update_node_attr(
        &self,
        id: i64,
        key: &str,
        value: Option<&str>,
    ) -> sqlx::Result<bool> {
        //language=sql
        let mut attrs: sqlx::types::Json<Vec<(String, String)>> = sqlx::query_scalar("select attributes from node where id = ?")
            .bind(id)
            .fetch_one(&self.pool).await?;
        // 移除原有同名属性
        attrs.retain(|(k, _)| k != key);
        // 添加新的属性
        if let Some(v) = value {
            attrs.push((key.to_string(), v.to_string()));
        }
        // 按 key 排序
        attrs.sort_by(|(a, _), (b, _)| a.cmp(b));
        let result = sqlx::query(
            r#"update node set attributes = ? where id = ? "#,
        )
            .bind(attrs)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }


    // 删除节点的时候，必须提供一个新的节点 ID 用来防止关联帖子给删除
    pub async fn delete_node(&self, id: i64, move_to: i64) -> sqlx::Result<bool> {
        let mut tx = self.pool.begin().await?;
        let _ = sqlx::query("update topic set node_id = ? where node_id = ? ")
            .bind(move_to)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let result = sqlx::query("DELETE FROM node WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }
}