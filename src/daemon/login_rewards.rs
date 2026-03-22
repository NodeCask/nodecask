use crate::store::Store;
use anyhow::anyhow;
use boa_engine::property::PropertyKey;
use boa_engine::{Context, JsValue, Source};
use boa_runtime::console::DefaultLogger;
use boa_runtime::Console;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use boa_engine::vm::RuntimeLimits;

#[derive(Clone)]
pub struct LoginRewardsDaemon {
    channel: Sender<Instruct>,
}

type Message = (i64, tokio::sync::oneshot::Sender<anyhow::Result<LoginRewards>>);

enum Instruct {
    Access(Message),
    CheckIn(Message),
    Reload,
}

#[derive(Serialize)]
struct AccessDaysData {
    #[serde(rename = "LastAccessDate")]
    last_access_date: String,
    #[serde(rename = "AccessDays")]
    access_days: i32,
    #[serde(rename = "ContinuousAccessDays")]
    continuous_access_days: i32,
}

#[derive(Serialize)]
struct CheckInData {
    // snake_case to match docs/rewards.ts and expected JS usage
    last_checkin_date: String,
    total_checkin_count: i32,
    current_continuous_checkin_count: i32,
}

// js vm 数字只有 32 位浮点数
#[derive(Clone, Deserialize, Debug)]
pub struct LoginRewardsF32 {
    pub credit: f32,
    pub coins: f32,
}
impl Into<LoginRewards> for LoginRewardsF32 {
    fn into(self) -> LoginRewards {
        LoginRewards{
            credit: self.credit as i32,
            coins: self.coins as i32,
        }
    }
}
#[derive(Clone, Deserialize, Debug)]
pub struct LoginRewards {
    pub credit: i32,
    pub coins: i32,
}

struct Calculator {
    ctx: Context,
    calculate_login_rewards: JsValue,
    calculate_check_in_rewards: JsValue,
}

impl Calculator {
    fn new(js_script: &str) -> anyhow::Result<Self> {
        let mut context = Context::default();
        let mut limit = RuntimeLimits::default();
        limit.set_recursion_limit(0xffff);
        limit.set_loop_iteration_limit(0xffff);
        limit.set_backtrace_limit(0xffff);
        context.set_runtime_limits(limit);
        Console::register_with_logger(DefaultLogger, &mut context)
            .expect("the console object shouldn't exist yet");
        context
            .eval(Source::from_bytes(js_script))
            .map_err(|e| anyhow!("JS Parsing Error: {}", e))?;

        let global_object = context.global_object();

        // Try camelCase first (per docs), then snake_case fallback
        let calculate_login_rewards = global_object
            .get(
                PropertyKey::String("calculateLoginRewards".into()),
                &mut context,
            )
            .or_else(|_| {
                global_object.get(
                    PropertyKey::String("calculate_login_rewards".into()),
                    &mut context,
                )
            })
            .map_err(|e| anyhow!("Get calculateLoginRewards error: {}", e))?;

        if !calculate_login_rewards.is_callable() {
            return Err(anyhow!("calculateLoginRewards is not a function"));
        }

        let calculate_check_in_rewards = global_object
            .get(
                PropertyKey::String("calculateCheckInRewards".into()),
                &mut context,
            )
            .or_else(|_| {
                global_object.get(
                    PropertyKey::String("calculate_check_in_rewards".into()),
                    &mut context,
                )
            })
            .map_err(|e| anyhow!("Get calculateCheckInRewards error: {}", e))?;

        if !calculate_check_in_rewards.is_callable() {
            return Err(anyhow!("calculateCheckInRewards is not a function"));
        }

        let mut instance = Self {
            ctx: context,
            calculate_login_rewards,
            calculate_check_in_rewards,
        };

        // Validate with mock data
        instance.validate_script()?;

        Ok(instance)
    }

    fn validate_script(&mut self) -> anyhow::Result<()> {
        let test_cases = [
            (1, 1, "2026-01-01"),
            (7, 7, "2026-01-07"),
            (14, 14, "2026-01-14"),
            (21, 21, "2026-01-21"),
            (28, 28, "2026-01-28"),
            (30, 30, "2026-01-30"),
            (60, 60, "2026-03-01"),
            (100, 10, "2026-04-10"),
            (365, 365, "2027-01-01"),
            (1000, 1, "2028-01-01"),
            (0, 0, "2026-01-01"),
            (-1, -1, "2026-01-01"),
        ];

        for (days, continuous, date) in test_cases {
            self.login_rewards(AccessDaysData {
                last_access_date: date.to_string(),
                access_days: days,
                continuous_access_days: continuous,
            })
            .map_err(|err| {
                anyhow!(
                    "Validation failed for login_rewards (days={}, continuous={}, date={}): {}",
                    days,
                    continuous,
                    date,
                    err
                )
            })?;

            self.check_in_rewards(CheckInData {
                last_checkin_date: date.to_string(),
                total_checkin_count: days,
                current_continuous_checkin_count: continuous,
            })
            .map_err(|err| {
                anyhow!(
                    "Validation failed for check_in_rewards (days={}, continuous={}, date={}): {}",
                    days,
                    continuous,
                    date,
                    err
                )
            })?;
        }

        // Fuzz-like data
        for i in 1..=50 {
            let days = (i * 17) % 1000;
            let continuous = (i * 7) % (days.max(0) + 1);
            self.login_rewards(AccessDaysData {
                last_access_date: "2026-01-01".to_string(),
                access_days: days,
                continuous_access_days: continuous,
            })
            .map_err(|err| anyhow!("Fuzz validation failed for login_rewards: {}", err))?;

            self.check_in_rewards(CheckInData {
                last_checkin_date: "2026-01-01".to_string(),
                total_checkin_count: days,
                current_continuous_checkin_count: continuous,
            })
            .map_err(|err| anyhow!("Fuzz validation failed for check_in_rewards: {}", err))?;
        }

        Ok(())
    }

    fn login_rewards(&mut self, data: AccessDaysData) -> anyhow::Result<LoginRewardsF32> {
        let json_value = serde_json::to_value(&data)?;
        let js_arg = JsValue::from_json(&json_value, &mut self.ctx)
            .map_err(|e| anyhow!("Input conversion error: {}", e))?;

        let result_js_value = self
            .calculate_login_rewards
            .as_callable()
            .unwrap() // Verified in new()
            .call(&JsValue::undefined(), &[js_arg], &mut self.ctx)
            .map_err(|e| anyhow!("Execution error: {}", e))?;

        let result_json = result_js_value
            .to_json(&mut self.ctx)
            .map_err(|e| anyhow!("Output conversion error: {}", e))?;

        let rewards: LoginRewardsF32 = serde_json::from_value(result_json.unwrap())?;
        Ok(rewards)
    }

    fn check_in_rewards(&mut self, data: CheckInData) -> anyhow::Result<LoginRewardsF32> {
        let json_value = serde_json::to_value(&data)?;
        let js_arg = JsValue::from_json(&json_value, &mut self.ctx)
            .map_err(|e| anyhow!("Input conversion error: {}", e))?;

        let result_js_value = self
            .calculate_check_in_rewards
            .as_callable()
            .unwrap() // Verified in new()
            .call(&JsValue::undefined(), &[js_arg], &mut self.ctx)
            .map_err(|e| anyhow!("Execution error: {}", e))?;

        let result_json = result_js_value
            .to_json(&mut self.ctx)
            .map_err(|e| anyhow!("Output conversion error: {}", e))?;

        let rewards: LoginRewardsF32 = serde_json::from_value(result_json.unwrap())?;
        Ok(rewards)
    }
}

struct DaemonWorker {
    store: Store,
    tokio_rt: tokio::runtime::Runtime,
    calculator: Option<Calculator>,
}

impl DaemonWorker {
    fn new(store: Store) -> Self {
        let tokio_rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        Self {
            store,
            tokio_rt,
            calculator: None,
        }
    }

    fn run(&mut self, receiver: Receiver<Instruct>) {
        while let Ok(ins) = receiver.recv() {
            match ins {
                Instruct::Reload => self.handle_reload(),
                Instruct::Access((uid, feedback)) => self.handle_access(uid, feedback),
                Instruct::CheckIn((uid, feedback)) => self.handle_checkin(uid, feedback),
            }
        }
        info!("LoginRewardsDaemon exiting...");
    }

    fn handle_reload(&mut self) {
        let result = self.tokio_rt.block_on(self.store.get_login_rewards_script());
        if result.script.is_empty() {
            info!("Login rewards script not configured");
            self.calculator = None;
            return;
        }
        match Calculator::new(&result.script) {
            Ok(calculator) => {
                info!("Login rewards script loaded successfully");
                self.calculator = Some(calculator);
            }
            Err(err) => {
                warn!("Login rewards script has errors: {}", err);
                // Keep old calculator? Or set to None?
                // Setting to None is safer to avoid stale bad behavior,
                // but if we want resilience, maybe keep old one?
                // Current behavior: None.
                self.calculator = None;
            }
        };
    }

    fn handle_access(
        &mut self,
        uid: i64,
        feedback: tokio::sync::oneshot::Sender<anyhow::Result<LoginRewards>>,
    ) {
        let Some(calculator) = self.calculator.as_mut() else {
            // Calculator not loaded, ignore request? Or return error?
            // Returning error is better so caller doesn't hang (if they didn't implement timeout).
            // But daemon loop usually continues.
            // Let's send an error.
            let _ = feedback.send(Err(anyhow!("奖励计算脚本未加载")));
            return;
        };

        match self.tokio_rt.block_on(self.store.get_user_access_stats(uid)) {
            Ok(Some(data)) => {
                let access_data = AccessDaysData {
                    last_access_date: data.last_access_date.format("%Y-%m-%d").to_string(),
                    access_days: data.access_days as i32,
                    continuous_access_days: data.continuous_access_days as i32,
                };

                match calculator.login_rewards(access_data) {
                    Ok(rewards) => {
                        if let Err(err) = self.tokio_rt.block_on(self.store.update_credit_and_coins(
                            uid,
                            rewards.credit as i64,
                            rewards.coins as i64,
                        )) {
                            warn!("Failed to update login rewards in DB: {}", err);
                            // Even if DB update fails, we probably shouldn't return success?
                            // Or should we return error?
                            let _ = feedback.send(Err(anyhow!("更新奖励失败: {}", err)));
                        } else {
                            let _ = self.tokio_rt.block_on(self.store.user_login_rewards(uid, "login", rewards.credit as i64, rewards.coins as i64));
                            let _ = feedback.send(Ok(rewards.into()));
                        }
                    }
                    Err(err) => {
                        warn!("Failed to calculate login rewards: {}", err);
                        let _ = feedback.send(Err(err));
                    }
                }
            }
            Ok(None) => {
                let _ = feedback.send(Err(anyhow!("会员未找到，id:{}", uid)));
            }
            Err(err) => {
                let _ = feedback.send(Err(anyhow!("查询会员信息失败： {}", err)));
            }
        }
    }

    fn handle_checkin(
        &mut self,
        uid: i64,
        feedback: tokio::sync::oneshot::Sender<anyhow::Result<LoginRewards>>,
    ) {
        let Some(calculator) = self.calculator.as_mut() else {
            let _ = feedback.send(Err(anyhow!("奖励计算脚本未加载")));
            return;
        };

        match self.tokio_rt.block_on(self.store.get_user_access_stats(uid)) {
            Ok(Some(data)) => {
                let checkin_data = CheckInData {
                    last_checkin_date: data.last_checkin_date.format("%Y-%m-%d").to_string(),
                    total_checkin_count: data.checkin_days as i32,
                    current_continuous_checkin_count: data.continuous_checkin_days as i32,
                };

                match calculator.check_in_rewards(checkin_data) {
                    Ok(rewards) => {
                        if let Err(err) = self.tokio_rt.block_on(self.store.update_credit_and_coins(
                            uid,
                            rewards.credit as i64,
                            rewards.coins as i64,
                        )) {
                            warn!("Failed to update check-in rewards in DB: {}", err);
                            let _ = self.tokio_rt.block_on(self.store.user_login_rewards(uid, "checkin", rewards.credit as i64, rewards.coins as i64));
                            let _ = feedback.send(Err(anyhow!("更新奖励失败: {}", err)));
                        } else {
                            let _ = feedback.send(Ok(rewards.into()));
                        }
                    }
                    Err(err) => {
                        warn!("Failed to calculate check-in rewards: {}", err);
                        let _ = feedback.send(Err(err));
                    }
                }
            }
            Ok(None) => {
                let _ = feedback.send(Err(anyhow!("会员未找到，id:{}", uid)));
            }
            Err(err) => {
                let _ = feedback.send(Err(anyhow!("查询会员信息失败： {}", err)));
            }
        }
    }
}
impl LoginRewardsDaemon {
    pub fn new(store: Store) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<Instruct>();
        
        std::thread::spawn(move || {
            let mut worker = DaemonWorker::new(store);
            worker.run(receiver);
        });
        
        // Trigger initial load
        let _ = sender.send(Instruct::Reload);
        
        Self { channel: sender }
    }

    pub async fn login(&self, uid: i64) -> anyhow::Result<LoginRewards> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<anyhow::Result<LoginRewards>>();
        self.channel.send(Instruct::Access((uid, sender)))
            .map_err(|_| anyhow!("Daemon channel closed"))?;
        receiver.await.map_err(|_| anyhow!("Daemon response channel closed"))?
    }

    pub async fn checkin(&self, uid: i64) -> anyhow::Result<LoginRewards> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<anyhow::Result<LoginRewards>>();
        self.channel.send(Instruct::CheckIn((uid, sender)))
             .map_err(|_| anyhow!("Daemon channel closed"))?;
        receiver.await.map_err(|_| anyhow!("Daemon response channel closed"))?
    }
    
    pub fn reload(&self) {
        let _ = self.channel.send(Instruct::Reload);
    }
}

pub fn is_script_valid(script: &str) -> anyhow::Result<()> {
    Calculator::new(script)?;
    Ok(())
}