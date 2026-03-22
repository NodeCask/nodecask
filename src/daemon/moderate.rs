use crate::store::topic::TopicDisplay;
use crate::store::Store;
use anyhow::anyhow;
use askama::Template;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

enum TaskExtend {
    Task(Task), // 审核任务
    Reload,     // 配置更新，需要重新加载配置
}

pub(crate) enum Task {
    NewTopic(i64),
    NewComment(i64),
}

#[derive(Template)]
#[template(path = "home/topic/system-delete-tips.html")]
pub struct CommentDeleteBySystemTips<'a> {
    pub locale: &'a str,
    pub content: &'a str,
    pub reason: &'a str,
}
#[derive(Template)]
#[template(path = "home/topic/system-delete-topic-tips.html")]
pub struct TopicDeleteBySystemTips<'a> {
    pub locale: &'a str,
    pub topic: &'a TopicDisplay,
    pub reason: &'a str,
}

#[derive(Clone)]
pub struct ModerateDaemon(Arc<ModerateDaemonInner>);
impl ModerateDaemon {
    pub async fn new(store: Store) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<TaskExtend>(1024);
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let mut topic_moderators: Vec<JoinedModerator> = vec![];
            let mut comment_moderators: Vec<JoinedModerator> = vec![];
            let mut trashed_id: i64 = 0;

            while let Some(task) = receiver.recv().await {
                if let TaskExtend::Reload = task {
                    info!("Reload the LLM moderate configuration.");
                    let cfg: ModeratorConfig =
                        store.get_cfg("llm_moderator").await.unwrap_or_default();
                    
                    let mut models_map: HashMap<String, ModelCfg> = HashMap::new();
                    for m in cfg.models {
                         models_map.insert(m.name.clone(), m);
                    }

                    topic_moderators.clear();
                    comment_moderators.clear();

                    for mod_cfg in cfg.moderators {
                        if !mod_cfg.enable {
                            continue;
                        }
                         if let Some(model_cfg) = models_map.get(&mod_cfg.model) {
                            let joined = JoinedModerator {
                                rule: mod_cfg.clone(),
                                model: model_cfg.clone(),
                            };
                            
                            if mod_cfg.target == "topic" {
                                info!("LLM moderator loaded: {} (Topic) using model {}", mod_cfg.name, mod_cfg.model);
                                topic_moderators.push(joined);
                            } else if mod_cfg.target == "comment" {
                                info!("LLM moderator loaded: {} (Comment) using model {}", mod_cfg.name, mod_cfg.model);
                                comment_moderators.push(joined);
                            }
                        } else {
                            warn!("LLM moderator {} references missing model: {}", mod_cfg.name, mod_cfg.model);
                        }
                    }

                    // 加载 trashed 节点 ID，如果找不到，删除帖子操作就是真的删除了
                    match store.get_node("trashed").await {
                        Ok(Some(node)) => {
                            trashed_id = node.id;
                        }
                        Ok(None) => {
                            error!("Node (trashed) not found");
                            trashed_id = 0;
                        }
                        Err(err) => {
                            error!("Failed to query node (trashed): {}", err);
                            trashed_id = 0;
                        }
                    }
                    if trashed_id == 0 {
                        info!("Violating topics will be deleted directly!!!")
                    } else {
                        info!("Violating topics will be moved to node: trashed (id: {})", trashed_id);
                    }
                    continue;
                }
                match task {
                    TaskExtend::Task(Task::NewTopic(id)) => {
                        let Ok(Some(topic)) = store.get_topic(id).await else {
                            continue;
                        };

                        let content_handler = |text: &str| -> String {
                            text.replace("%TOPIC_TITLE%", &topic.title)
                                .replace("%TOPIC_CONTENT%", &topic.content)
                                .replace("%TOPIC_CONTENT_RENDER%", &topic.content_render)
                                .replace("%TOPIC_CONTENT_PLAIN%", &topic.content_plain)
                        };
                        for x in &topic_moderators {
                            let content_handler = |text: &str| -> String {
                                cut(content_handler(text), x.model.max_content as usize)
                            };
                            info!("LLM moderator: {} - evaluating topic: {}", x.rule.name, &topic.title);
                            match x.evaluate(&client, content_handler).await {
                                Ok(res) => {
                                    info!(
                                        "LLM moderator: {} - evaluating topic: {} result: {:?}",
                                        x.rule.name, &topic.title, &res
                                    );
                                    match res.action.as_str() {
                                        "pass" => continue,
                                        "pass-and-exit" => break,
                                        "lock" => {
                                            if let Err(err) = store.lock_topic(topic.id, true).await
                                            {
                                                warn!("Lock failed: {:?}", err);
                                            }
                                            break;
                                        }
                                        "delete" => {
                                            if trashed_id <= 0 {
                                                if let Err(err) = store.delete_topic(topic.id).await
                                                {
                                                    warn!(
                                                        "delete topic ({}) failed: {:?}",
                                                        &topic.title, err
                                                    );
                                                }
                                            } else {
                                                if let Err(err) = store
                                                    .move_and_lock_topic(topic.id, trashed_id)
                                                    .await
                                                {
                                                    warn!(
                                                        "move topic ({}) to trashed failed: {:?}",
                                                        &topic.title, err
                                                    );
                                                }
                                            }
                                            // 发送提醒消息
                                            let msg = TopicDeleteBySystemTips {
                                                locale:"en_US", // TODO: 查询用户语言偏好
                                                topic: &topic,
                                                reason: &res.reason,
                                            }
                                            .render()
                                            .unwrap_or_default();
                                            if let Err(err) = store
                                                .add_notification(
                                                    topic.user_id,
                                                    "topic.delete",
                                                    topic.id,
                                                    &msg,
                                                    None,
                                                )
                                                .await
                                            {
                                                warn!(
                                                    "send topic ({}) delete message failed: {:?}",
                                                    &topic.title, err
                                                );
                                            }
                                            // 扣除金币或是信用分数
                                            if x.rule.coins_damage > 0 {
                                                let _ = store.update_coins(topic.user_id, - x.rule.coins_damage).await;
                                            }
                                            if x.rule.credit_damage > 0 {
                                                let _ = store.update_credit(topic.user_id, - x.rule.credit_damage).await;
                                            }
                                            break;
                                        }
                                        "move" => {
                                            // TODO: 移动帖子
                                            break;
                                        }
                                        other => {
                                            warn!("Unrecognized action: {}", other);
                                            continue;
                                        }
                                    }
                                }
                                Err(err) => {
                                    warn!(
                                        "LLM moderator: {} - evaluating topic: {} failed: {}",
                                        x.rule.name,
                                        &topic.title,
                                        err.to_string()
                                    );
                                }
                            }
                        }
                    }
                    TaskExtend::Task(Task::NewComment(id)) => {
                        let Ok(Some(comment)) = store.get_comment(id).await else {
                            continue;
                        };
                        let content_handler = |text: &str| -> String {
                            text.replace("%COMMENT_CONTENT%", &comment.content)
                                .replace("%COMMENT_CONTENT_RENDER%", &comment.content_render)
                                .replace("%COMMENT_CONTENT_PLAIN%", &comment.content_plain)
                        };
                        for x in &comment_moderators {
                            let content_handler = |text: &str| -> String {
                                cut(content_handler(text), x.model.max_content as usize)
                            };
                            let res = x.evaluate(&client, content_handler).await;
                            info!(
                                "LLM moderator: {} - evaluating comment: {} result: {:?}",
                                x.rule.name, &comment.content_plain, &res
                            );
                            if match res {
                                Ok(res) => match res.action.as_str() {
                                    "pass" => true,
                                    "pass-and-exit" => false,
                                    "delete" => {
                                        if let Err(err) = store.delete_comment(comment.id).await {
                                            warn!(
                                                "delete comment ({}) failed: {:?}",
                                                &comment.content_plain, err
                                            );
                                            continue;
                                        }
                                        // 发送提醒消息
                                        let msg = CommentDeleteBySystemTips {
                                            locale:"en_US", // TODO: 查询用户语言偏好
                                            content: &comment.content_plain,
                                            reason: &res.reason,
                                        }
                                        .render()
                                        .unwrap_or_default();
                                        if let Err(err) = store
                                            .add_notification(
                                                comment.user_id,
                                                "topic.reply",
                                                comment.article_id,
                                                &msg,
                                                None,
                                            )
                                            .await
                                        {
                                            warn!(
                                                "delete comment ({}) failed: {:?}",
                                                &comment.content_plain, err
                                            );
                                        }
                                        // 扣除金币或是信用分数
                                        if x.rule.coins_damage > 0 {
                                            let _ = store.update_coins(comment.user_id, -x.rule.coins_damage).await;
                                        }
                                        if x.rule.credit_damage > 0 {
                                            let _ = store.update_credit(comment.user_id, -x.rule.coins_damage).await;
                                        }
                                        true
                                    }
                                    other => {
                                        warn!("Unrecognized action: {}", other);
                                        true
                                    }
                                },
                                Err(err) => {
                                    warn!(
                                        "LLM moderator: {} - evaluating comment: {} failed: {}",
                                        x.rule.name,
                                        &comment.content_plain,
                                        err.to_string()
                                    );
                                    true
                                }
                            } {
                                continue;
                            }
                            break;
                        }
                    }
                    TaskExtend::Reload => {
                        unreachable!()
                    }
                }
            }
            info!("ModerateDaemon exiting...");
        });

        if let Err(err) = sender.send(TaskExtend::Reload).await {
            warn!("Failed to send reload task: {}", err);
        }
        ModerateDaemon(Arc::new(ModerateDaemonInner { channel: sender }))
    }

    pub async fn push(&self, task: Task) {
        if let Err(err) = self.0.channel.send(TaskExtend::Task(task)).await {
            warn!("Failed to push moderate task: {}", err);
        }
    }
    pub async fn reload(&self) {
        if let Err(err) = self.0.channel.send(TaskExtend::Reload).await {
            warn!("Failed to send reload task: {}", err);
        }
    }
}

pub struct ModerateDaemonInner {
    channel: tokio::sync::mpsc::Sender<TaskExtend>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PromptConfig {
    pub system: String,
    pub user: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelCfg {
    pub name: String, // name 不能重复
    pub url: String,
    pub key: String,
    pub model: String,
    pub temperature: f32,
    pub max_content: i64,
    pub flavor: String, // "google" | "openai"
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModeratorCfg {
    pub name: String,
    pub model: String, // 对应上面 Model 类型 name 字段
    pub enable: bool,
    pub target: String, // "topic" | "comment"
    pub prompt: PromptConfig,
    pub credit_damage: i64,
    pub coins_damage: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModeratorConfig {
    pub models: Vec<ModelCfg>,
    pub moderators: Vec<ModeratorCfg>,
}

pub struct JoinedModerator {
    pub rule: ModeratorCfg,
    pub model: ModelCfg,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct EvaluateResult {
    action: String,
    reason: String,
    #[allow(dead_code)]
    move_to: Option<String>,
}

impl JoinedModerator {
    async fn evaluate(
        &self,
        client: &reqwest::Client,
        content_handler: impl Fn(&str) -> String,
    ) -> anyhow::Result<EvaluateResult> {
        let mut system_prompt = String::new();
        if !self.rule.prompt.system.is_empty() {
            system_prompt = content_handler(&self.rule.prompt.system);
        }
        let mut user_prompt = String::new();
         if !self.rule.prompt.user.is_empty() {
             user_prompt = content_handler(&self.rule.prompt.user);
        }

        let content = if self.model.flavor == "google" {
            let system_instruction: Value = if system_prompt.is_empty() {
                Value::Null
            } else {
                serde_json::json!({
                    "parts": [ { "text": system_prompt } ]
                })
            };

            let payload = serde_json::json!({
                "systemInstruction": system_instruction,
                "contents": [ {
                        "role": "user",
                        "parts": [ { "text": user_prompt } ]
                } ],
                "generationConfig": {
                    "temperature": self.model.temperature,
                }
            });
            let mut url = Url::parse(self.model.url.as_str())?;
            url.query_pairs_mut().append_pair("key", self.model.key.as_str());

            let response = client
                .post(url)
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?;

             if !response.status().is_success() {
                let error_text = response.text().await?;
                return Err(anyhow!("Google API request failed: {}", error_text));
            }

            let response_json: Value = response.json().await?;
            // candidates[0].content.parts[0].text
            response_json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .ok_or(anyhow!("No content in Google API response"))?
                .to_string()

        } else {
            // OpenAI Flavor
            let mut messages: Vec<Value> = vec![];
            if !system_prompt.is_empty() {
                 messages.push(serde_json::json!({
                    "role": "system".to_string(),
                    "content": system_prompt,
                }));
            }
             if !user_prompt.is_empty() {
                 messages.push(serde_json::json!({
                    "role": "user".to_string(),
                    "content": user_prompt,
                }));
            }

            let payload = serde_json::json!({
                "model": self.model.model.clone(),
                "messages": messages,
                "temperature": self.model.temperature,
                "stream": false,
            });

            let response = client
                .post(self.model.url.as_str())
                .header("Authorization", format!("Bearer {}", &self.model.key))
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .json(&payload)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await?;
                return Err(anyhow!("OpenAI API request failed: {}", error_text));
            }

            let response_json: Value = response.json().await?;
            response_json["choices"][0]["message"]["content"]
                .as_str()
                .ok_or(anyhow!("No content in OpenAI API response"))?
                .to_string()
        };

        let clean_json_str = clean_markdown_json(&content);
        let result: EvaluateResult = serde_json::from_str(&clean_json_str).map_err(|e| {
            anyhow!(
                "Failed to parse LLM JSON output: {}. Content was: {}",
                e,
                content
            )
        })?;

        Ok(result)
    }
}

fn clean_markdown_json(input: &str) -> String {
    let input = input.trim();
    if input.starts_with("```json") {
        input
            .trim_start_matches("```json")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else if input.starts_with("```") {
        input
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else {
        input.to_string()
    }
}

fn cut(mut text: String, max: usize) -> String {
    if max == 0 { return text; } // 0 means no limit? Or typical usage.
    match text.char_indices().nth(max) {
        Some((byte_index, _)) => {
            text.truncate(byte_index);
            text
        }
        None => text,
    }
}
