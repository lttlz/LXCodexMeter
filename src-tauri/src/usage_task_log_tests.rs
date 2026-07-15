mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn store() -> UsageTaskStore {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lxcodexmeter-usage-log-{nonce}.json"));
        UsageTaskStore::load(path, 1_000)
    }

    fn snapshot(at: u64, weekly: f64, five_hour: f64) -> UsageSnapshot {
        UsageSnapshot {
            captured_at_ms: at,
            weekly_remaining_percent: Some(weekly),
            five_hour_remaining_percent: Some(five_hour),
        }
    }

    fn completed_task(id: usize) -> UsageTask {
        UsageTask {
            id: format!("task-{id}"),
            started_at_ms: id as u64,
            ended_at_ms: id as u64 + 1,
            duration_seconds: 0,
            weekly_consumed_percent: 1.0,
            five_hour_consumed_percent: 1.0,
            record_mode: "automatic".to_string(),
            is_complete: true,
            is_estimated: false,
            created_at_ms: id as u64,
            updated_at_ms: id as u64 + 1,
        }
    }

    #[test]
    fn first_snapshot_only_builds_baseline() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        assert!(store.view(1_000).tasks.is_empty());
    }

    #[test]
    fn positive_deltas_create_and_accumulate_one_task() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 77.0, 55.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(3_000, 76.0, 53.0), true)
            .unwrap();
        let tasks = store.view(3_000).tasks;
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].is_active);
        assert!((tasks[0].task.weekly_consumed_percent - 4.0).abs() < 0.0001);
        assert!((tasks[0].task.five_hour_consumed_percent - 7.0).abs() < 0.0001);
    }

    #[test]
    fn reset_and_noise_never_create_negative_consumption() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 77.0, 50.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 100.0, 50.005), true)
            .unwrap();
        assert!(store.view(2_000).tasks.is_empty());
        store
            .record_snapshot(snapshot(3_000, 99.0, 49.995), true)
            .unwrap();
        let task = &store.view(3_000).tasks[0].task;
        assert!((task.weekly_consumed_percent - 1.0).abs() < 0.0001);
        assert!(task.five_hour_consumed_percent >= 0.0);
    }

    #[test]
    fn reset_finishes_an_active_task_without_negative_delta() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 77.0, 58.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(3_000, 100.0, 58.0), true)
            .unwrap();
        let tasks = store.view(3_000).tasks;
        assert_eq!(tasks.len(), 1);
        assert!(!tasks[0].is_active);
        assert_eq!(tasks[0].task.weekly_consumed_percent, 3.0);
    }

    #[test]
    fn idle_timeout_splits_tasks() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 79.0, 59.0), true)
            .unwrap();
        assert!(store.close_idle_task(2_000 + IDLE_TIMEOUT_MS).unwrap());
        store
            .record_snapshot(snapshot(2_000 + IDLE_TIMEOUT_MS + 1_000, 78.0, 58.0), true)
            .unwrap();
        let tasks = store.view(2_000 + IDLE_TIMEOUT_MS + 1_000).tasks;
        assert_eq!(tasks.len(), 2);
        assert!(!tasks[0].is_active);
        assert!(tasks[1].is_active);
    }

    #[test]
    fn clear_history_keeps_active_task_and_baseline() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 79.0, 59.0), true)
            .unwrap();
        store.clear_history().unwrap();
        let view = store.view(2_000);
        assert_eq!(view.tasks.len(), 1);
        assert!(view.tasks[0].is_active);
        assert!(store.data.last_snapshot.is_some());
    }

    #[test]
    fn restart_recovers_active_task_as_incomplete() {
        let mut store = store();
        let path = store.path.clone();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 79.0, 59.0), true)
            .unwrap();
        drop(store);

        let recovered = UsageTaskStore::load(path, 3_000);
        let tasks = recovered.view(3_000).tasks;
        assert_eq!(tasks.len(), 1);
        assert!(!tasks[0].is_active);
        assert!(!tasks[0].task.is_complete);
        assert!(tasks[0].task.is_estimated);
    }

    #[test]
    fn corrupt_file_is_preserved_and_replaced_with_empty_log() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lxcodexmeter-corrupt-{nonce}.json"));
        let backup_stamp = (nonce % u64::MAX as u128) as u64;
        fs::write(&path, b"not-json").unwrap();
        let store = UsageTaskStore::load(path.clone(), backup_stamp);
        assert!(store.view(backup_stamp).tasks.is_empty());
        assert!(store.view(backup_stamp).warning.is_some());
        assert!(path
            .with_file_name(format!("usage-task-log.corrupt-{backup_stamp}.json"))
            .exists());
    }

    #[test]
    fn task_history_is_trimmed_to_ten_thousand() {
        let mut tasks: Vec<UsageTask> = (0..=MAX_TASKS).map(completed_task).collect();
        trim_oldest(&mut tasks);
        assert_eq!(tasks.len(), MAX_TASKS);
        assert_eq!(tasks.first().unwrap().id, "task-1");
    }

    #[test]
    fn invalid_preferences_fall_back_to_required_defaults() {
        let mut store = store();
        store
            .save_preferences(UsageLogPreferences {
                weekly_filter: "bad".to_string(),
                custom_threshold: 200.0,
                time_filter: "bad".to_string(),
                sort_mode: "bad".to_string(),
            })
            .unwrap();
        assert_eq!(
            store.view(1_000).preferences,
            UsageLogPreferences::default()
        );
    }
}
