use chrono::Utc;
use crate::store::Store;
use log::{info, warn};
use mail_send::mail_builder::headers::address::Address;
use mail_send::mail_builder::headers::Header;
use mail_send::mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
enum Instruct {
    Mail(i64),
    Reload,
}

#[derive(Clone, Copy)]
pub struct Mail(pub i64);

#[derive(Clone)]
pub struct EmailSenderDaemon {
    channel: tokio::sync::mpsc::Sender<Instruct>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SmtpConfig {
    from_name: String,
    from_mail: String,
    hostname: String,
    port: u16,
    tls_implicit: bool,
    username: String,
    password: String,
    max_per_hour: i64,
    max_per_day: i64,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            from_name: "".to_string(),
            from_mail: "".to_string(),
            hostname: "".to_string(),
            port: 465,
            tls_implicit: false,
            username: "".to_string(),
            password: "".to_string(),
            max_per_hour: 10,
            max_per_day: 20,
        }
    }
}

#[derive(Clone)]
struct Limit {
    max_per_hour: i64,
    max_per_day: i64,
}

#[derive(Clone)]
struct SenderConfig {
    address: Address<'static>,
    builder: SmtpClientBuilder<String>,
    limit: Limit,
}

impl EmailSenderDaemon {
    pub async fn new(store: Store) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel::<Instruct>(100);
        tokio::spawn(run_worker(store, receiver));
        sender.send(Instruct::Reload).await.ok();
        Self { channel: sender }
    }
    pub async fn send(&self, mail: Mail) {
        if let Err(err) = self.channel.send(Instruct::Mail(mail.0)).await {
            warn!("Failed to send mail task: {}", err);
        }
    }
    pub async fn reload(&self) {
        if let Err(err) = self.channel.send(Instruct::Reload).await {
            warn!("Failed to send reload task: {}", err);
        }
    }
}

async fn run_worker(store: Store, mut receiver: tokio::sync::mpsc::Receiver<Instruct>) {
    let mut sender_cfg: Option<SenderConfig> = None;
    while let Some(ins) = receiver.recv().await {
        match ins {
            Instruct::Reload => {
                let cfg: Option<SmtpConfig> = store.get_cfg("smtp_config").await;
                sender_cfg = cfg.map(|cfg| {
                    let builder = SmtpClientBuilder::new(cfg.hostname.clone(), cfg.port)
                        .implicit_tls(cfg.tls_implicit)
                        .credentials((cfg.username.clone(), cfg.password.clone()));
                    let address = Address::new_address(
                        if cfg.from_name.is_empty() {
                            None
                        } else {
                            Some(cfg.from_name.clone())
                        },
                        if cfg.from_mail.is_empty() {
                            cfg.from_mail.clone()
                        } else {
                            cfg.username.clone()
                        },
                    );
                    SenderConfig {
                        address,
                        builder,
                        limit: Limit {
                            max_per_hour: cfg.max_per_hour,
                            max_per_day: cfg.max_per_day,
                        },
                    }
                });
            }
            Instruct::Mail(id) => {
                process_email(&store, id, &sender_cfg).await;
            }
        }
    }
    info!("EmailSenderDaemon exiting...");
}

async fn check_limit(store: &Store, email: &str, limit: &Limit) -> Result<bool, anyhow::Error> {
    if limit.max_per_hour > 0 {
        let n = store
            .count_by_email(email, Utc::now() - chrono::Duration::hours(1))
            .await?;
        if n > limit.max_per_hour {
            return Ok(false);
        }
    }
    if limit.max_per_day > 0 {
        let n = store
            .count_by_email(email, Utc::now() - chrono::Duration::days(1))
            .await?;
        if n > limit.max_per_day {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn process_email(store: &Store, id: i64, sender_cfg: &Option<SenderConfig>) {
    let Ok(Some(mail)) = store.get_email(id).await else {
        warn!("Email(id: {}) not found", id);
        return;
    };

    let (status, msg) = match sender_cfg {
        Some(cfg) => match check_limit(store, &mail.email_to, &cfg.limit).await {
            Err(e) => ("failed", format!("Querying email count failed: {}", e)),
            Ok(false) => ("limited", address_to_string(&cfg.address)),
            Ok(true) => {
                let message = MessageBuilder::new()
                    .from(cfg.address.clone())
                    .to(mail.email_to.as_str())
                    .subject(mail.email_subject.as_str())
                    .html_body(mail.email_body.as_str());

                match cfg.builder.clone().connect().await {
                    Ok(mut client) => match client.send(message).await {
                        Ok(_) => {
                            info!(
                                "The email ({}) has been sent to {}",
                                mail.email_subject, mail.email_to
                            );
                            ("sent", address_to_string(&cfg.address))
                        }
                        Err(e) => {
                            warn!("Email sending failed: {}", e);
                            ("failed", address_to_string(&cfg.address))
                        }
                    },
                    Err(e) => {
                        warn!("Connection to mail agent failed: {}", e);
                        ("failed", address_to_string(&cfg.address))
                    }
                }
            }
        },
        None => {
            info!("smtp cfg not found");
            ("drop", "smtp cfg not found".to_string())
        }
    };

    let _ = store.mark_email_done(mail.id, &msg, status).await;
}

fn address_to_string(addr: &Address) -> String {
    // 这里貌似存在一个 bug，write_header 这个方法返回写死 0, 所以要使用 Vec<u8>
    let mut buffer:Vec<u8> = Vec::with_capacity(1024);
    match addr.write_header(&mut buffer,0) {
        Ok(_) => {
            String::from_utf8_lossy(buffer.as_slice()).to_string()
        }
        Err(_) => {
            format!("{:?}", addr)
        }
    }
}