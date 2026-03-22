use crate::store::{Pagination, PaginationQuery, RangeQuery, Store};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tantivy::doc;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TopicIndex {
    pub id: i64,
    pub title: String,
    pub view_count: i64,
    pub reply_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: i64,
    pub username: String,
    pub last_reply_by: Option<String>,
    pub node_id: i64,
    pub node_name: String,
    pub node_slug: String,
    pub is_locked: bool,
    pub is_pinned: bool,
    pub avatar_timestamp: i64,
}
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TopicDisplay {
    pub id: i64,
    pub title: String,
    pub content: String,        // 原始文本
    pub content_render: String, // 渲染后的文本
    pub content_plain: String,  // 纯文本，用来制作索引
    pub view_count: i64,
    pub reply_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: i64,
    pub username: String,
    pub last_reply_by: Option<String>,
    pub node_id: i64,
    pub node_name: String,
    pub node_slug: String,
    pub is_locked: bool,
    pub is_pinned: bool,
    pub avatar_timestamp: i64,
    #[sqlx(skip)]
    pub ops: Vec<String>,
}
impl TopicDisplay {
    pub fn render(&self) -> String {
        self.content_render.clone()
    }
    pub fn op(&self, op: &str) -> bool {
        self.ops.contains(&op.to_string())
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CommentDisplay {
    pub id: i64,
    pub article_id: i64,
    pub user_id: i64,
    pub content: String,
    pub content_render: String,
    pub content_plain: String,
    pub created_at: DateTime<Utc>,
    pub username: String,
    pub floor: i64,
    pub avatar_timestamp: i64,
    #[sqlx(skip)]
    pub ops: Vec<String>,
}
impl CommentDisplay {
    pub fn render(&self) -> String {
        self.content_render.clone()
    }
    pub fn op(&self, op: &str) -> bool {
        self.ops.contains(&op.to_string())
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserCommentDisplay {
    pub id: i64,
    pub article_id: i64,
    pub article_title: String,
    pub article_author_username: String,
    pub user_id: i64,
    pub content: String,
    pub content_render: String,
    pub content_plain: String,
    pub created_at: DateTime<Utc>,
    pub username: String,
    pub floor: i64,
    pub avatar_timestamp: i64,
}
impl UserCommentDisplay {
    pub fn render(&self) -> String {
        self.content_render.clone()
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopicSearch {
    pub sort: Option<String>, // 排序规则：如果 latest 表示发帖时间，不然就是热点排序
    pub q: Option<String>,    // 帖子查询
    pub user_id: Option<i64>, // 发帖用户 ID
    pub username: Option<String>, // 发帖用户
    pub node_id: Option<i64>, // 节点过滤
    pub node_slug: Option<String>, // 发帖版块
    pub viewer_id: Option<i64>, // 设定访问用户，用来排除屏蔽
    pub recent_flag: bool,    // 如果 recent_flag 标记开启的话，会检查 nodes.show_in_list 标记
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommentSearch {
    pub topic: Option<i64>,
    pub username: Option<String>, // 发言用户
    pub viewer_id: Option<i64>,
}
impl Store {
    pub async fn get_topic(&self, id: i64) -> sqlx::Result<Option<TopicDisplay>> {
        sqlx::query_as::<_, TopicDisplay>(
            r#"SELECT t.id,
       t.title,
       t.content,
       t.content_render,
       t.content_plain,
       t.view_count,
       t.reply_count,
       t.created_at,
       t.updated_at,
       t.last_reply_by,
       u.id   as user_id,
       u.username,
       u.avatar_timestamp,
       n.name as node_name,
       n.slug as node_slug,
       t.node_id,
       t.is_locked,
       t.is_pinned
FROM topic t
         JOIN user u ON t.user_id = u.id
         JOIN node n ON t.node_id = n.id
WHERE t.id = ?
    "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_topic_count(&self) -> sqlx::Result<i64> {
        let n: (i64,) = sqlx::query_as("SELECT count(*) FROM topic")
            .fetch_one(&self.pool)
            .await?;
        Ok(n.0)
    }

    pub async fn topics(
        &self,
        search: &TopicSearch,
        page: PaginationQuery,
    ) -> sqlx::Result<Pagination<TopicIndex>> {
        // language=sql
        let total: u32 = sqlx::query_scalar(
            r#"SELECT count(*)
FROM topic t
         JOIN user u ON t.user_id = u.id
         JOIN node n ON t.node_id = n.id
         LEFT JOIN user_relation ur ON ur.target_id = t.user_id AND ur.relation = 'block' AND ur.user_id = ?
WHERE
    ur.user_id IS NULL AND
    (? is false or n.show_in_list) and
    (? is NULL or t.title like '%'||?||'%') and
    (? is NULL or u.id = ?) and
    (? is NULL or u.username = ?) and
    (? is NULL or n.id = ?) and
    (? is NULL or n.slug = ?) and
    (n.isolated = 0 or t.user_id = ?)"#,
        )
        .bind(search.viewer_id)
        .bind(search.recent_flag)
        .bind(&search.q)
        .bind(&search.q)
        .bind(&search.user_id)
        .bind(&search.user_id)
        .bind(&search.username)
        .bind(&search.username)
        .bind(&search.node_id)
        .bind(&search.node_id)
        .bind(&search.node_slug)
        .bind(&search.node_slug)
        .bind(search.viewer_id)
        .fetch_one(&self.pool)
        .await?;
        let order_col = search
            .sort
            .as_ref()
            .map(|s| match s.as_str() {
                "latest" => "t.created_at",
                "update" => "t.updated_at",
                "rank" => "t.rank_score",
                _ => "t.id",
            })
            .unwrap_or("t.created_at");
        // language=sql
        let sql = "SELECT t.id,
       t.title,
       t.content,
       t.content_render,
       t.content_plain,
       t.view_count,
       t.reply_count,
       t.created_at,
       t.updated_at,
       t.last_reply_by,
       u.id   as user_id,
       u.username,
       u.avatar_timestamp,
       n.name as node_name,
       n.slug as node_slug,
       t.node_id,
       t.is_locked,
       t.is_pinned
FROM topic t
         JOIN user u ON t.user_id = u.id
         JOIN node n ON t.node_id = n.id
         LEFT JOIN user_relation ur ON ur.target_id = t.user_id AND ur.relation = 'block' AND ur.user_id = ?
WHERE ur.user_id IS NULL
  AND (? is false or n.show_in_list)
  and (? is NULL or t.title like '%' || ? || '%')
  and (? is NULL or u.id = ?)
  and (? is NULL or u.username = ?)
  and (? is NULL or n.id = ?)
  and (? is NULL or n.slug = ?)
  and (n.isolated = 0 or t.user_id = ?)";
        let data: Vec<TopicIndex> = sqlx::query_as::<_, TopicIndex>(&format!(
            "{sql} ORDER BY t.is_pinned DESC, {order_col} DESC LIMIT ?,?
        "
        ))
        .bind(search.viewer_id)
        .bind(search.recent_flag)
        .bind(&search.q)
        .bind(&search.q)
        .bind(&search.user_id)
        .bind(&search.user_id)
        .bind(&search.username)
        .bind(&search.username)
        .bind(&search.node_id)
        .bind(&search.node_id)
        .bind(&search.node_slug)
        .bind(&search.node_slug)
        .bind(search.viewer_id)
        .bind(page.start())
        .bind(page.size())
        .fetch_all(&self.pool)
        .await?;

        Ok(Pagination::new(page, total, data))
    }

    pub async fn topics_before(
        &self,
        search: &TopicSearch,
        range: &RangeQuery,
    ) -> sqlx::Result<Vec<TopicIndex>> {
        // language=sql
        let data: Vec<TopicIndex> = sqlx::query_as::<_, TopicIndex>(
            "SELECT t.id,
       t.title,
       t.content,
       t.content_render,
       t.content_plain,
       t.view_count,
       t.reply_count,
       t.created_at,
       t.updated_at,
       t.last_reply_by,
       u.id   as user_id,
       u.username,
       u.avatar_timestamp,
       n.name as node_name,
       n.slug as node_slug,
       t.node_id,
       t.is_locked,
       t.is_pinned
FROM topic t
         JOIN user u ON t.user_id = u.id
         JOIN node n ON t.node_id = n.id
         LEFT JOIN user_relation ur ON ur.target_id = t.user_id AND ur.relation = 'block' AND ur.user_id = ?
WHERE ur.user_id IS NULL
  AND (? is false or n.show_in_list)
  and (? is NULL or t.title like '%' || ? || '%')
  and (? is NULL or u.id = ?)
  and (? is NULL or u.username = ?)
  and (? is NULL or n.id = ?)
  and (? is NULL or n.slug = ?)
  and (n.isolated = 0 or t.user_id = ?)
  and (? is null or t.created_at < ?)
  and (? is null or t.created_at > ?)
ORDER BY t.id DESC
LIMIT ? "
        )
            .bind(search.viewer_id).bind(search.recent_flag)
            .bind(&search.q)
            .bind(&search.q)
            .bind(&search.user_id)
            .bind(&search.user_id)
            .bind(&search.username)
            .bind(&search.username)
            .bind(&search.node_id)
            .bind(&search.node_id)
            .bind(&search.node_slug)
            .bind(&search.node_slug)
            .bind(search.viewer_id)
            .bind(range.before)
            .bind(range.after)
            .bind(range.amount)
            .fetch_all(&self.pool)
            .await?;

        Ok(data)
    }

    pub async fn new_topic(
        &self,
        user: i64,
        node: i64,
        title: &str,
        content: &str,
        content_render: &str,
        content_plain: &str,
    ) -> sqlx::Result<i64> {
        let rank_score = Utc::now().timestamp();
        let result = sqlx::query(r#"INSERT INTO topic (user_id, node_id, title, content,content_render,content_plain, created_at, updated_at, rank_score)
VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'), ?)"#)
            .bind(user)
            .bind(node)
            .bind(&title)
            .bind(&content)
            .bind(&content_render)
            .bind(&content_plain)
            .bind(rank_score)
            .execute(&self.pool)
            .await?;
        Ok(result.last_insert_rowid())
    }
    pub async fn update_topic(
        &self,
        id: i64,
        title: String,
        content: String,
        content_render: String,
        content_plain: String,
    ) -> sqlx::Result<i64> {
        let result = sqlx::query(
            "UPDATE topic set title = ?, content = ?, content_render = ?, content_plain=?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&title)
        .bind(&content)
            .bind(&content_render)
            .bind(&content_plain)
        .bind(id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(id)
    }

    pub async fn trash_topic(&self, id: i64) -> sqlx::Result<()> {
        // language=sql
        let node_id: i64 = sqlx::query_scalar("select id from node where slug='trashed' limit 1")
            .fetch_one(&self.pool)
            .await?;
        sqlx::query("UPDATE topic SET node_id = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(node_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn move_topic(&self, id: i64, node_id: i64) -> sqlx::Result<()> {
        sqlx::query("UPDATE topic SET node_id = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(node_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn move_and_lock_topic(&self, id: i64, node_id: i64) -> sqlx::Result<()> {
        sqlx::query("UPDATE topic SET is_locked = 1, node_id = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(node_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_topic(&self, id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM topic WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn lock_topic(&self, id: i64, locked: bool) -> sqlx::Result<()> {
        sqlx::query("UPDATE topic SET is_locked = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(locked)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn pin_topic(&self, id: i64, pinned: bool) -> sqlx::Result<()> {
        sqlx::query("UPDATE topic SET is_pinned = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(pinned)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn incr_view_count(&self, id: i64) -> sqlx::Result<()> {
        sqlx::query("UPDATE topic SET view_count = view_count + 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn discover(&self, viewer_id: Option<i64>) -> sqlx::Result<Vec<TopicIndex>> {
        sqlx::query_as::<_, TopicIndex>(
            r#"SELECT t.id,
       t.title,
       t.view_count,
       t.reply_count,
       t.created_at,
       t.updated_at,
       t.last_reply_by,
       u.id   as user_id,
       u.username,
       u.avatar_timestamp,
       n.name as node_name,
       n.slug as node_slug,
       t.node_id,
       t.is_locked,
       t.is_pinned
FROM topic t
         JOIN user u ON t.user_id = u.id
         JOIN node n ON t.node_id = n.id
         LEFT JOIN user_relation ur ON ur.target_id = t.user_id AND ur.relation = 'block' AND ur.user_id = ?
WHERE n.show_in_list and ur.user_id IS NULL
ORDER BY t.is_pinned DESC, t.rank_score DESC
LIMIT 20"#,
        )
        .bind(viewer_id)
        .fetch_all(&self.pool)
        .await
    }
    // 返回某个帖子某个会员实际可以看到回复数量（有些已经删除，有些在黑名单里面不显示）
    pub async fn comments_count(&self, tid: i64, viewer_id: Option<i64>) -> sqlx::Result<u32> {
        let total: u32 = sqlx::query_scalar(
            r#"
            SELECT count(*) FROM comment c
            JOIN user u ON c.user_id = u.id
            LEFT JOIN user_relation ur ON ur.target_id = c.user_id AND ur.relation = 'block' AND ur.user_id = ?
            WHERE ur.user_id IS NULL AND c.article_id = ?
        "#,
        )
            .bind(viewer_id)
            .bind(tid)
            .fetch_one(&self.pool)
            .await?;
        Ok(total)
    }

    pub async fn comments(
        &self,
        search: &CommentSearch,
        page: PaginationQuery,
    ) -> sqlx::Result<Pagination<CommentDisplay>> {
        // language=sql
        let total: u32 = sqlx::query_scalar(
            r#"SELECT count(*)
FROM comment c
         JOIN user u ON c.user_id = u.id
         LEFT JOIN user_relation ur ON ur.target_id = c.user_id AND ur.relation = 'block' AND ur.user_id = ?
WHERE ur.user_id IS NULL
  AND (? is NULL or c.article_id = ?)
  and (? is null or u.username = ?)
        "#,
        )
        .bind(search.viewer_id)
        .bind(search.topic)
            .bind(search.topic)
            .bind(&search.username)
            .bind(&search.username)
        .fetch_one(&self.pool)
        .await?;

        let data = sqlx::query_as::<_, CommentDisplay>(
            r#"SELECT c.id,
       c.article_id,
       c.user_id,
       c.content,
       c.content_render,
       c.content_plain,
       c.created_at,
       u.username,
       u.avatar_timestamp,
       c.floor
FROM comment c
         JOIN user u ON c.user_id = u.id
         LEFT JOIN user_relation ur ON ur.target_id = c.user_id AND ur.relation = 'block' AND ur.user_id = ?
WHERE ur.user_id IS NULL
  AND (? is NULL or c.article_id = ?)
  and (? is null or u.username = ?)
ORDER BY c.created_at
LIMIT ?,?
        "#,
        )
            .bind(search.viewer_id)
            .bind(search.topic)
            .bind(search.topic)
            .bind(&search.username)
            .bind(&search.username)
            .bind(page.start())
            .bind(page.size())
            .fetch_all(&self.pool)
            .await?;
        Ok(Pagination::new(page, total, data))
    }

    pub async fn comments_before(
        &self,
        search: &CommentSearch,
        range: &RangeQuery,
    ) -> sqlx::Result<Vec<CommentDisplay>> {
        let data = sqlx::query_as::<_, CommentDisplay>(
            r#"SELECT c.id,
       c.article_id,
       c.user_id,
       c.content,
       c.content_render,
       c.content_plain,
       c.created_at,
       u.username,
       u.avatar_timestamp,
       c.floor
FROM comment c
         JOIN user u ON c.user_id = u.id
         LEFT JOIN user_relation ur ON ur.target_id = c.user_id AND ur.relation = 'block' AND ur.user_id = ?
WHERE ur.user_id IS NULL
  AND (? is NULL or c.article_id = ?)
  and (? is null or u.username = ?)
  and (? is null or c.created_at < ?)
  and (? is null or c.created_at > ?)
ORDER BY c.created_at
LIMIT ?
        "#,
        )
            .bind(search.viewer_id)
            .bind(search.topic)
            .bind(search.topic)
            .bind(&search.username)
            .bind(&search.username)
            .bind(range.before)
            .bind(range.after)
            .bind(range.amount)
            .fetch_all(&self.pool)
            .await?;
        Ok(data)
    }

    pub async fn comment(
        &self,
        uid: i64,
        tid: i64,
        content: String,
        content_render: String,
        content_plain: String,
    ) -> sqlx::Result<i64> {
        let mut tx = self.pool.begin().await?;
        #[derive(sqlx::FromRow)]
        struct TopicState {
            reply_count: i64,
            created_at: DateTime<Utc>,
            rank_score: i64,
        }
        let state: TopicState =
            sqlx::query_as("SELECT reply_count, created_at, rank_score FROM topic WHERE id = ?")
                .bind(tid)
                .fetch_one(&mut *tx)
                .await?;

        let current_time = Utc::now().timestamp() as f64;
        let created_ts = state.created_at.timestamp() as f64;
        let old_rank = state.rank_score as f64;

        let diff_days = (current_time - created_ts) / 86400.0;
        let factor = if diff_days < 3.0 {
            1.0
        } else if diff_days < 7.0 {
            0.8
        } else if diff_days < 30.0 {
            0.5
        } else if diff_days < 90.0 {
            0.2
        } else {
            0.1
        };

        let bonus = if state.reply_count <= 10 {
            600.0
        } else if state.reply_count <= 50 {
            300.0
        } else {
            60.0
        };

        let new_rank = old_rank + (current_time - old_rank) * factor + bonus;

        let floor = state.reply_count + 1;

        let res = sqlx::query(r#"INSERT INTO comment (article_id, user_id, content, content_render, content_plain, floor, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))"#)
            .bind(tid)
            .bind(uid)
            .bind(&content)
            .bind(&content_render)
            .bind(&content_plain)
            .bind(floor)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            UPDATE topic
            SET reply_count = reply_count + 1, 
                updated_at = datetime('now'), 
                last_reply_by = (SELECT username FROM user WHERE id = ?),
                rank_score = ?
            WHERE id = ?
        "#,
        )
        .bind(uid)
        .bind(new_rank as i64)
        .bind(tid)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn delete_comment(&self, id: i64) -> sqlx::Result<()> {
        // 删除回复的时候，不需要更新主题回复数量
        sqlx::query("DELETE FROM comment WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_comment(&self, id: i64) -> sqlx::Result<Option<CommentDisplay>> {
        sqlx::query_as::<_, CommentDisplay>(
            r#"
            SELECT 
                c.id, c.article_id, c.user_id, c.content, c.content_render, c.content_plain, c.created_at,
                u.username, u.avatar_timestamp, c.floor
            FROM comment c
            JOIN user u ON c.user_id = u.id
            WHERE c.id = ?
        "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn user_comments(
        &self,
        uid: i64,
        page: PaginationQuery,
    ) -> sqlx::Result<Pagination<UserCommentDisplay>> {
        let total: (u32,) = sqlx::query_as("SELECT count(*) FROM comment WHERE user_id = ?")
            .bind(uid)
            .fetch_one(&self.pool)
            .await?;

        let data = sqlx::query_as::<_, UserCommentDisplay>(
            r#"SELECT c.id,
       c.article_id,
       t.title as article_title,
       tu.username as article_author_username,
       c.user_id,
       c.content,
       c.content_render,
       c.content_plain,
       c.created_at,
       u.username,
       u.avatar_timestamp,
       c.floor
FROM comment c
         JOIN user u ON c.user_id = u.id
         JOIN topic t ON c.article_id = t.id
         JOIN user tu ON t.user_id = tu.id
WHERE c.user_id = ?
ORDER BY c.created_at DESC
LIMIT ?,?
        "#,
        )
        .bind(uid)
        .bind(page.start())
        .bind(page.size())
        .fetch_all(&self.pool)
        .await?;

        Ok(Pagination::new(page, total.0, data))
    }

    pub async fn update_topic_bot_status(&self, id: i64, is_bot: bool) -> sqlx::Result<()> {
        sqlx::query("UPDATE topic SET bot = ? WHERE id = ?")
            .bind(is_bot)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_comment_bot_status(&self, id: i64, is_bot: bool) -> sqlx::Result<()> {
        sqlx::query("UPDATE comment SET bot = ? WHERE id = ?")
            .bind(is_bot)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
