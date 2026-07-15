use crate::{
    meter::{read_status, CodexMeterStatus, LimitWindow},
    usage_task_log::{UsageLogPreferences, UsageLogView, UsageSnapshot, UsageTaskStore},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex as AsyncMutex;

const RECENT_STATUS_MS: u64 = 5_000;

#[derive(Clone)]
pub struct UsageRuntime {
    inner: Arc<UsageRuntimeInner>,
}

struct UsageRuntimeInner {
    request_lock: AsyncMutex<()>,
    store: Mutex<Option<UsageTaskStore>>,
    latest_status: Mutex<Option<CodexMeterStatus>>,
    runtime_warning: Mutex<Option<String>>,
    target_running: AtomicBool,
    final_refresh_requested: AtomicBool,
}

impl UsageRuntime {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(UsageRuntimeInner {
                request_lock: AsyncMutex::new(()),
                store: Mutex::new(None),
                latest_status: Mutex::new(None),
                runtime_warning: Mutex::new(None),
                target_running: AtomicBool::new(false),
                final_refresh_requested: AtomicBool::new(false),
            }),
        }
    }

    pub fn initialize_store(&self, path: PathBuf) {
        let store = UsageTaskStore::load(path, now_ms());
        *self
            .inner
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(store);
    }

    pub fn set_target_running(&self, running: bool) {
        let previous = self.inner.target_running.swap(running, Ordering::SeqCst);
        if previous && !running {
            self.inner
                .final_refresh_requested
                .store(true, Ordering::SeqCst);
        }
    }

    pub fn target_running(&self) -> bool {
        self.inner.target_running.load(Ordering::SeqCst)
    }

    pub fn final_refresh_pending(&self) -> bool {
        self.inner.final_refresh_requested.load(Ordering::SeqCst)
    }

    pub fn take_final_refresh_request(&self) -> bool {
        self.inner
            .final_refresh_requested
            .swap(false, Ordering::SeqCst)
    }

    pub async fn fetch_status(
        &self,
        mode: Option<String>,
        client_text: Option<String>,
        allow_final_consumption: bool,
        force: bool,
    ) -> Result<CodexMeterStatus, String> {
        let _request = self.inner.request_lock.lock().await;

        if !force {
            if let Some(status) = self.recent_status() {
                return Ok(status);
            }
        }

        let status = read_status(mode, client_text).await?;
        if status.ok {
            if let Some(snapshot) = snapshot_from_status(&status) {
                let allow_consumption = self.target_running() || allow_final_consumption;
                if let Err(error) =
                    self.with_store(|store| store.record_snapshot(snapshot, allow_consumption))
                {
                    self.set_warning(error);
                }
            }
        }
        *self
            .inner
            .latest_status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(status.clone());
        Ok(status)
    }

    pub fn close_idle_task(&self) {
        if let Err(error) = self.with_store(|store| store.close_idle_task(now_ms()).map(|_| ())) {
            self.set_warning(error);
        }
    }

    pub fn finish_for_process_exit(&self, final_refresh_ok: bool) {
        if let Err(error) =
            self.with_store(|store| store.finish_for_process_exit(now_ms(), final_refresh_ok))
        {
            self.set_warning(error);
        }
    }

    pub fn finish_for_app_exit(&self) {
        if let Err(error) = self.with_store(|store| store.finish_for_app_exit(now_ms())) {
            self.set_warning(error);
        }
    }

    pub fn log_view(&self) -> Result<UsageLogView, String> {
        let mut view = self.with_store(|store| Ok(store.view(now_ms())))?;
        if view.warning.is_none() {
            view.warning = self
                .inner
                .runtime_warning
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
        }
        Ok(view)
    }

    pub fn delete_task(&self, id: &str) -> Result<bool, String> {
        self.with_store(|store| store.delete_task(id))
    }

    pub fn clear_history(&self) -> Result<(), String> {
        self.with_store(UsageTaskStore::clear_history)
    }

    pub fn save_preferences(&self, preferences: UsageLogPreferences) -> Result<(), String> {
        self.with_store(|store| store.save_preferences(preferences))
    }

    fn recent_status(&self) -> Option<CodexMeterStatus> {
        self.inner
            .latest_status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .filter(|status| {
                now_ms().saturating_sub(status.updated_at_ms.min(u64::MAX as u128) as u64)
                    <= RECENT_STATUS_MS
            })
            .cloned()
    }

    fn with_store<T>(
        &self,
        operation: impl FnOnce(&mut UsageTaskStore) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self
            .inner
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let store = guard
            .as_mut()
            .ok_or_else(|| "消耗日志后端尚未初始化".to_string())?;
        operation(store)
    }

    fn set_warning(&self, warning: String) {
        *self
            .inner
            .runtime_warning
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(warning);
    }
}

fn snapshot_from_status(status: &CodexMeterStatus) -> Option<UsageSnapshot> {
    let mut weekly = None;
    let mut five_hour = None;
    let mut unknown: Vec<&LimitWindow> = Vec::new();

    for limit in [status.primary.as_ref(), status.secondary.as_ref()]
        .into_iter()
        .flatten()
    {
        match limit.window_duration_mins {
            Some(minutes) if minutes <= 360 => five_hour = limit.remaining_percent,
            Some(minutes) if minutes >= 7 * 24 * 60 => weekly = limit.remaining_percent,
            _ => unknown.push(limit),
        }
    }

    if five_hour.is_none() {
        five_hour = status
            .primary
            .as_ref()
            .and_then(|limit| limit.remaining_percent);
    }
    if weekly.is_none() {
        weekly = status
            .secondary
            .as_ref()
            .and_then(|limit| limit.remaining_percent);
    }
    if five_hour.is_none() && weekly.is_none() {
        for limit in unknown {
            if five_hour.is_none() {
                five_hour = limit.remaining_percent;
            } else if weekly.is_none() {
                weekly = limit.remaining_percent;
            }
        }
    }
    if five_hour.is_none() && weekly.is_none() {
        return None;
    }

    Some(UsageSnapshot {
        captured_at_ms: status.updated_at_ms.min(u64::MAX as u128) as u64,
        weekly_remaining_percent: weekly,
        five_hour_remaining_percent: five_hour,
    })
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit(remaining: f64, duration: i64) -> LimitWindow {
        LimitWindow {
            label: String::new(),
            used_percent: Some(100.0 - remaining),
            remaining_percent: Some(remaining),
            window_duration_mins: Some(duration),
            resets_at: None,
            reset_text: None,
            reached_type: None,
        }
    }

    #[test]
    fn snapshot_maps_windows_by_duration() {
        let status = CodexMeterStatus {
            ok: true,
            message: "OK".to_string(),
            source_mode: "app_server".to_string(),
            auth_mode: None,
            plan_type: None,
            primary: Some(limit(70.0, 300)),
            secondary: Some(limit(40.0, 10_080)),
            credit_balance: None,
            credit_limit: None,
            reset_credits_available: None,
            updated_at_ms: 123,
        };
        let snapshot = snapshot_from_status(&status).unwrap();
        assert_eq!(snapshot.five_hour_remaining_percent, Some(70.0));
        assert_eq!(snapshot.weekly_remaining_percent, Some(40.0));
    }
}
