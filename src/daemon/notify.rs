use askama::Template;
use crate::store::Store;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use crate::store::topic::TopicDisplay;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notify {
    pub user_id: i64,
    pub category: String,
    pub content: String,
    pub link_id: i64,
    pub meta: Option<serde_json::Value>,
}


#[derive(Template)]
#[template(path = "home/topic/at-tips.html")]
pub struct AtUserTipsTemplate<'a> {
    locale: &'a str,
    topic: &'a TopicDisplay,
    username: &'a str,
    reply: bool,
    floor: i64,
}

impl Notify {
    fn new(user_id: i64, category: String, content: String, link_id: i64) -> Self {
        Self {
            user_id,
            category,
            content,
            link_id,
            meta: None,
        }
    }
    pub fn reply(user_id: i64, topic_id: i64, content: String) -> Self {
        Self::new(user_id, "topic.reply".to_string(), content, topic_id)
    }
    pub fn topic_move(user_id: i64, topic_id: i64, content: String) -> Self {
        Self::new(user_id, "topic.move".to_string(), content, topic_id)
    }
    pub fn comment_delete(user_id: i64, topic_id: i64, content: String) -> Self {
        Self::new(user_id, "comment.delete".to_string(), content, topic_id)
    }
}
#[derive(Clone)]
pub struct NotifyDaemon {
    channel: tokio::sync::mpsc::Sender<Instruct>,
}

enum Instruct {
    NewTopicAtUsers(i64, Vec<String>),
    NewCommentAtUsers(i64, Vec<String>),
    Notify(Notify),
}
impl NotifyDaemon {
    pub async fn new(store: Store) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Instruct>(100);
        tokio::spawn(async move {
            while let Some(t) = receiver.recv().await {
                match t {
                    Instruct::NewTopicAtUsers(id, users) => {
                        let Some(topic) = store.get_topic(id).await.inspect_err(|err|{
                            error!("Error getting topic: {}", err);
                        }).unwrap_or_default() else{
                            continue
                        };
                        let Ok(msg) = AtUserTipsTemplate {
                            locale: "en_US",// TODO: 查询用户语言偏好
                            topic: &topic,
                            username: &topic.username,
                            reply: false,
                            floor: 0,
                        }.render().inspect_err(|err|{
                            error!("Error rendering at tips: {}", err);
                        }) else {
                            continue;
                        };
                        for username in &users {
                            let Some(target_user) = store.get_user(username).await.unwrap_or_default()else {
                                continue
                            };

                            if let Err(err) = store.add_notification(target_user.id, "topic.at_user", id, &msg, None).await {
                                error!("Error sending notification: {}", err);
                            }
                        }

                    }
                    Instruct::NewCommentAtUsers(id, users) => {
                        let Some(comment) = store.get_comment(id).await.unwrap_or_default() else {
                            continue
                        };
                        let Some(topic) = store.get_topic(comment.article_id).await.inspect_err(|err|{
                            error!("Error getting topic: {}", err);
                        }).unwrap_or_default() else{
                            continue
                        };
                        let Ok(msg) = AtUserTipsTemplate {
                            locale: "en_US",// TODO: 查询用户语言偏好
                            topic: &topic,
                            username: &topic.username,
                            reply: true,
                            floor: comment.floor,
                        }.render().inspect_err(|err|{
                            error!("Error rendering at tips: {}", err);
                        }) else {
                            continue;
                        };
                        for username in &users {
                            let Some(target_user) = store.get_user(username).await.unwrap_or_default()else {
                                continue
                            };
                            if let Err(err) = store.add_notification(target_user.id, "topic.at_user", id, &msg, None).await {
                                error!("Error sending notification: {}", err);
                            }
                        }
                    }
                    Instruct::Notify(t) => {
                        let user_id = t.user_id;
                        let category = t.category;
                        let content = t.content;
                        let link_id = t.link_id;
                        let meta = t.meta.map(|v| v.to_string()).clone();
                        if let Err(err) = store
                            .add_notification(user_id, &category, link_id, &content, meta)
                            .await
                        {
                            error!("Failed to add notification: {}", err);
                        }
                    }
                }
            }
            info!("NotifyDaemon exiting...");
        });
        Self { channel: sender }
    }

    pub async fn send(&self, msg: Notify) {
        if let Err(err) = self.channel.send(Instruct::Notify(msg)).await {
            warn!("Failed to send notification task: {}", err);
        }
    }
    pub async fn topic_at_users(&self, id: i64, users: Vec<String>) {
        if let Err(err) = self.channel.send(Instruct::NewTopicAtUsers(id, users)).await {
            warn!("Failed to send topic_at_users task: {}", err);
        }
    }
    pub async fn comment_at_users(&self, id: i64, users: Vec<String>) {
        // 延迟 30s 发送通知信息，让自动审核程序有时间删除违规评论
        let channel = self.channel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            let _ = channel.send(Instruct::NewCommentAtUsers(id, users)).await;
        });
    }
}
