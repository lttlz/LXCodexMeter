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

    fn optional_snapshot(
        at: u64,
        weekly: Option<f64>,
        five_hour: Option<f64>,
    ) -> UsageSnapshot {
        UsageSnapshot {
            captured_at_ms: at,
            weekly_remaining_percent: weekly,
            five_hour_remaining_percent: five_hour,
        }
    }

    fn completed_task(id: usize) -> UsageTask {
        UsageTask {
            id: format!("task-{id}"),
            started_at_ms: id as u64,
            ended_at_ms: id as u64 + 1,
            duration_seconds: 0,
            weekly_consumed_percent: Some(1.0),
            five_hour_consumed_percent: Some(1.0),
            end_weekly_remaining_percent: Some(79.0),
            end_five_hour_remaining_percent: Some(59.0),
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
    fn weekly_only_snapshot_never_creates_five_hour_consumption() {
        let mut store = store();
        store
            .record_snapshot(optional_snapshot(1_000, Some(80.0), None), true)
            .unwrap();
        store
            .record_snapshot(optional_snapshot(2_000, Some(77.0), None), true)
            .unwrap();

        let task = &store.view(2_000).tasks[0].task;
        assert_eq!(task.weekly_consumed_percent, Some(3.0));
        assert_eq!(task.five_hour_consumed_percent, None);
    }

    #[test]
    fn missing_quota_clears_baseline_and_recovery_only_rebuilds_it() {
        let mut store = store();
        store
            .record_snapshot(optional_snapshot(1_000, Some(80.0), Some(60.0)), true)
            .unwrap();
        store
            .record_snapshot(optional_snapshot(2_000, Some(79.0), None), true)
            .unwrap();
        store
            .record_snapshot(optional_snapshot(3_000, Some(78.0), Some(55.0)), true)
            .unwrap();

        let recovered = &store.view(3_000).tasks[0].task;
        assert_eq!(recovered.weekly_consumed_percent, Some(2.0));
        assert_eq!(recovered.five_hour_consumed_percent, Some(0.0));

        store
            .record_snapshot(optional_snapshot(4_000, Some(77.0), Some(53.0)), true)
            .unwrap();
        let next = &store.view(4_000).tasks[0].task;
        assert_eq!(next.weekly_consumed_percent, Some(3.0));
        assert_eq!(next.five_hour_consumed_percent, Some(2.0));
    }

    #[test]
    fn weekly_recovery_does_not_overwrite_five_hour_baseline() {
        let mut store = store();
        store
            .record_snapshot(optional_snapshot(1_000, None, Some(60.0)), true)
            .unwrap();
        store
            .record_snapshot(optional_snapshot(2_000, Some(80.0), Some(58.0)), true)
            .unwrap();

        let task = &store.view(2_000).tasks[0].task;
        assert_eq!(task.weekly_consumed_percent, Some(0.0));
        assert_eq!(task.five_hour_consumed_percent, Some(2.0));
    }

    #[test]
    fn both_missing_then_recovering_rebuilds_both_baselines() {
        let mut store = store();
        store
            .record_snapshot(optional_snapshot(1_000, Some(80.0), Some(60.0)), true)
            .unwrap();
        store
            .record_snapshot(optional_snapshot(2_000, None, None), true)
            .unwrap();
        store
            .record_snapshot(optional_snapshot(3_000, Some(70.0), Some(50.0)), true)
            .unwrap();
        assert!(store.view(3_000).tasks.is_empty());
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
        assert!((tasks[0].task.weekly_consumed_percent.unwrap() - 4.0).abs() < 0.0001);
        assert!((tasks[0].task.five_hour_consumed_percent.unwrap() - 7.0).abs() < 0.0001);
        assert_eq!(tasks[0].task.end_weekly_remaining_percent, Some(76.0));
        assert_eq!(tasks[0].task.end_five_hour_remaining_percent, Some(53.0));
    }

    #[test]
    fn active_balance_tracks_latest_snapshot_without_new_consumption() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 77.0, 58.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(3_000, 77.005, 58.005), true)
            .unwrap();

        let task = &store.view(3_000).tasks[0].task;
        assert_eq!(task.end_weekly_remaining_percent, Some(77.005));
        assert_eq!(task.end_five_hour_remaining_percent, Some(58.005));
    }

    #[test]
    fn active_balance_tracks_latest_non_consuming_snapshot() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 77.0, 58.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(3_000, 74.0, 55.0), false)
            .unwrap();

        let task = &store.view(3_000).tasks[0].task;
        assert_eq!(task.weekly_consumed_percent, Some(3.0));
        assert_eq!(task.five_hour_consumed_percent, Some(2.0));
        assert_eq!(task.end_weekly_remaining_percent, Some(74.0));
        assert_eq!(task.end_five_hour_remaining_percent, Some(55.0));
    }

    #[test]
    fn completed_task_keeps_its_stored_end_balance() {
        let mut store = store();
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 77.0, 58.0), true)
            .unwrap();
        store.close_idle_task(2_000 + IDLE_TIMEOUT_MS).unwrap();
        store
            .record_snapshot(snapshot(2_000 + IDLE_TIMEOUT_MS + 1_000, 70.0, 50.0), true)
            .unwrap();

        let task = &store.view(2_000 + IDLE_TIMEOUT_MS + 1_000).tasks[0].task;
        assert_eq!(task.end_weekly_remaining_percent, Some(77.0));
        assert_eq!(task.end_five_hour_remaining_percent, Some(58.0));
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
        assert!((task.weekly_consumed_percent.unwrap() - 1.0).abs() < 0.0001);
        assert!(task.five_hour_consumed_percent.unwrap() >= 0.0);
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
        assert_eq!(tasks[0].task.weekly_consumed_percent, Some(3.0));
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
        let preferences = UsageLogPreferences {
            weekly_filter: "all".to_string(),
            custom_threshold: 2.5,
            time_filter: "7d".to_string(),
            sort_mode: "duration".to_string(),
        };
        store.save_preferences(preferences.clone()).unwrap();
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
        assert_eq!(view.preferences, preferences);
    }

    #[test]
    fn delete_removes_only_the_requested_historical_task() {
        let mut store = store();
        store.data.tasks = vec![completed_task(1), completed_task(2)];
        store
            .record_snapshot(snapshot(1_000, 80.0, 60.0), true)
            .unwrap();
        store
            .record_snapshot(snapshot(2_000, 79.0, 59.0), true)
            .unwrap();

        assert!(store.delete_task("task-1").unwrap());
        assert!(!store.delete_task("task-1").unwrap());
        assert!(!store.delete_task("task-2000-3").unwrap());
        let view = store.view(2_000);
        assert_eq!(view.tasks.len(), 2);
        assert_eq!(view.tasks[0].task.id, "task-2");
        assert!(view.tasks[1].is_active);
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
    fn legacy_schema_preserves_history_closes_active_and_drops_slot_baseline() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lxcodexmeter-legacy-{nonce}.json"));
        let legacy = UsageLogData {
            schema_version: 1,
            last_snapshot: Some(snapshot(1_000, 80.0, 60.0)),
            active_task: Some(ActiveUsageTask {
                id: "legacy-active".to_string(),
                started_at_ms: 1_000,
                last_activity_at_ms: 2_000,
                last_observed_at_ms: 2_500,
                weekly_consumed_percent: Some(2.0),
                five_hour_consumed_percent: Some(3.0),
                end_weekly_remaining_percent: None,
                end_five_hour_remaining_percent: None,
                created_at_ms: 1_000,
                updated_at_ms: 2_000,
            }),
            tasks: vec![completed_task(1)],
            preferences: UsageLogPreferences::default(),
        };
        fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

        let migrated = UsageTaskStore::load(path, 3_000);
        assert_eq!(migrated.data.schema_version, SCHEMA_VERSION);
        assert!(migrated.data.last_snapshot.is_none());
        assert!(migrated.data.active_task.is_none());
        assert_eq!(migrated.data.tasks.len(), 2);
        assert!(migrated.data.tasks[0].is_complete);
        assert!(!migrated.data.tasks[1].is_complete);
        assert!(migrated.data.tasks[1].is_estimated);
        assert!(migrated.warning.is_some());
    }

    #[test]
    fn version_two_upgrade_keeps_baseline_and_missing_balances_are_null() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lxcodexmeter-v2-{nonce}.json"));
        let legacy_json = r#"{
          "schemaVersion": 2,
          "lastSnapshot": {
            "capturedAtMs": 1000,
            "weeklyRemainingPercent": 80.0,
            "fiveHourRemainingPercent": 60.0
          },
          "activeTask": null,
          "tasks": [{
            "id": "v2-task",
            "startedAtMs": 1000,
            "endedAtMs": 2000,
            "durationSeconds": 1,
            "weeklyConsumedPercent": 1.0,
            "fiveHourConsumedPercent": 2.0,
            "recordMode": "automatic",
            "isComplete": true,
            "isEstimated": false,
            "createdAtMs": 1000,
            "updatedAtMs": 2000
          }],
          "preferences": {
            "weeklyFilter": "gte3",
            "customThreshold": 3.0,
            "timeFilter": "30d",
            "sortMode": "latest"
          }
        }"#;
        fs::write(&path, legacy_json).unwrap();

        let migrated = UsageTaskStore::load(path, 3_000);
        assert_eq!(migrated.data.schema_version, SCHEMA_VERSION);
        assert!(migrated.data.last_snapshot.is_some());
        assert_eq!(migrated.data.tasks[0].end_weekly_remaining_percent, None);
        assert_eq!(migrated.data.tasks[0].end_five_hour_remaining_percent, None);
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
