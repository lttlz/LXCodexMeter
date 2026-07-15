use crate::meter::LimitWindow;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub(crate) struct NormalizedRateLimits {
    pub(crate) five_hour: Option<LimitWindow>,
    pub(crate) weekly: Option<LimitWindow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuotaKind {
    FiveHour,
    Weekly,
}

pub(crate) fn parse_limit(source_key: &str, value: Option<&Value>) -> Option<LimitWindow> {
    let value = value?;
    if value.is_null() {
        return None;
    }
    let used = value.get("usedPercent").and_then(Value::as_f64);
    let remaining = value
        .get("remainingPercent")
        .and_then(Value::as_f64)
        .or_else(|| used.map(|value| 100.0 - value));
    let duration = value.get("windowDurationMins").and_then(Value::as_i64);
    let resets_at = value.get("resetsAt").and_then(Value::as_i64);
    let reached_type = value
        .get("rateLimitReachedType")
        .and_then(Value::as_str)
        .map(str::to_string);
    if used.is_none()
        && remaining.is_none()
        && duration.is_none()
        && resets_at.is_none()
        && reached_type.is_none()
    {
        return None;
    }

    let semantic_id = semantic_value(value, &["id", "limitId", "rateLimitId"]).or_else(|| {
        (!matches!(source_key, "primary" | "secondary")).then(|| source_key.to_string())
    });
    let semantic_label = semantic_value(value, &["name", "label"]);
    Some(LimitWindow {
        label: semantic_label
            .clone()
            .or_else(|| semantic_id.clone())
            .unwrap_or_default(),
        used_percent: used,
        remaining_percent: remaining,
        window_duration_mins: duration,
        resets_at,
        reset_text: None,
        reached_type,
        semantic_id,
        semantic_label,
    })
}

pub(crate) fn collect_limit_windows(value: &Value) -> Vec<LimitWindow> {
    value
        .as_object()
        .into_iter()
        .flat_map(|object| object.iter())
        .filter_map(|(key, value)| parse_limit(key, Some(value)))
        .collect()
}

pub(crate) fn normalize_rate_limits(limits: &[LimitWindow]) -> NormalizedRateLimits {
    let mut five_hour: Option<(u8, LimitWindow)> = None;
    let mut weekly: Option<(u8, LimitWindow)> = None;

    for limit in limits {
        let Some((kind, confidence)) = classify_limit(limit) else {
            continue;
        };
        let target = match kind {
            QuotaKind::FiveHour => &mut five_hour,
            QuotaKind::Weekly => &mut weekly,
        };
        if target
            .as_ref()
            .is_none_or(|(existing_confidence, _)| confidence > *existing_confidence)
        {
            *target = Some((confidence, limit.clone()));
        }
    }

    NormalizedRateLimits {
        five_hour: five_hour.map(|(_, limit)| limit),
        weekly: weekly.map(|(_, limit)| limit),
    }
}

fn classify_limit(limit: &LimitWindow) -> Option<(QuotaKind, u8)> {
    match limit.window_duration_mins {
        Some(300) => return Some((QuotaKind::FiveHour, 3)),
        Some(10_080) => return Some((QuotaKind::Weekly, 3)),
        _ => {}
    }

    classify_semantic(limit.semantic_id.as_deref())
        .map(|kind| (kind, 2))
        .or_else(|| classify_semantic(limit.semantic_label.as_deref()).map(|kind| (kind, 1)))
}

fn classify_semantic(value: Option<&str>) -> Option<QuotaKind> {
    let value = value?.to_ascii_lowercase();
    let five_hour = ["5h", "5-hour", "5 hour", "five hour", "session", "short"]
        .iter()
        .any(|keyword| value.contains(keyword));
    let weekly = ["weekly", "week", "7-day", "7 day", "周", "long"]
        .iter()
        .any(|keyword| value.contains(keyword));
    match (five_hour, weekly) {
        (true, false) => Some(QuotaKind::FiveHour),
        (false, true) => Some(QuotaKind::Weekly),
        _ => None,
    }
}

fn semantic_value(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn normalized(value: Value) -> NormalizedRateLimits {
        normalize_rate_limits(&collect_limit_windows(&value))
    }

    #[test]
    fn normalizes_duration_windows_independent_of_slot_order() {
        let regular = normalized(json!({
            "primary": { "usedPercent": 30.0, "windowDurationMins": 300 },
            "secondary": { "usedPercent": 60.0, "windowDurationMins": 10080 }
        }));
        assert_eq!(regular.five_hour.unwrap().remaining_percent, Some(70.0));
        assert_eq!(regular.weekly.unwrap().remaining_percent, Some(40.0));

        let reversed = normalized(json!({
            "primary": { "usedPercent": 41.0, "windowDurationMins": 10080 },
            "secondary": { "usedPercent": 28.0, "windowDurationMins": 300 }
        }));
        assert_eq!(reversed.five_hour.unwrap().remaining_percent, Some(72.0));
        assert_eq!(reversed.weekly.unwrap().remaining_percent, Some(59.0));
    }

    #[test]
    fn normalizes_single_known_window_without_guessing_the_missing_kind() {
        let weekly_in_primary = normalized(json!({
            "primary": { "usedPercent": 41.0, "windowDurationMins": 10080 },
            "secondary": null
        }));
        assert!(weekly_in_primary.five_hour.is_none());
        assert_eq!(
            weekly_in_primary.weekly.unwrap().remaining_percent,
            Some(59.0)
        );

        let weekly_in_secondary = normalized(json!({
            "primary": null,
            "secondary": { "usedPercent": 50.0, "windowDurationMins": 10080 }
        }));
        assert!(weekly_in_secondary.five_hour.is_none());
        assert!(weekly_in_secondary.weekly.is_some());

        let five_hour_only = normalized(json!({
            "primary": { "usedPercent": 20.0, "windowDurationMins": 300 }
        }));
        assert!(five_hour_only.five_hour.is_some());
        assert!(five_hour_only.weekly.is_none());

        let absent = normalized(json!({ "primary": null, "secondary": null }));
        assert!(absent.five_hour.is_none());
        assert!(absent.weekly.is_none());
    }

    #[test]
    fn semantic_fallback_uses_upstream_identifiers_but_never_slot_names() {
        let labeled = normalized(json!({
            "primary": { "usedPercent": 25.0, "label": "Weekly limit" },
            "secondary": { "usedPercent": 10.0, "id": "session-quota" }
        }));
        assert_eq!(labeled.five_hour.unwrap().remaining_percent, Some(90.0));
        assert_eq!(labeled.weekly.unwrap().remaining_percent, Some(75.0));

        let unknown = normalized(json!({
            "primary": { "usedPercent": 25.0, "label": "Flexible quota" }
        }));
        assert!(unknown.five_hour.is_none());
        assert!(unknown.weekly.is_none());
    }

    #[test]
    fn exact_duration_wins_over_conflicting_labels_and_one_window_has_one_kind() {
        let conflict = normalized(json!({
            "primary": {
                "usedPercent": 25.0,
                "windowDurationMins": 10080,
                "id": "session-quota",
                "label": "5-hour limit"
            }
        }));
        assert!(conflict.five_hour.is_none());
        assert!(conflict.weekly.is_some());

        let ambiguous = normalized(json!({
            "primary": { "usedPercent": 25.0, "label": "session weekly quota" }
        }));
        assert!(ambiguous.five_hour.is_none());
        assert!(ambiguous.weekly.is_none());
    }

    #[test]
    fn future_semantic_slot_is_classified_without_slot_position() {
        let limits = normalized(json!({
            "long-window": { "usedPercent": 12.0, "name": "weekly" }
        }));
        assert!(limits.five_hour.is_none());
        assert_eq!(limits.weekly.unwrap().remaining_percent, Some(88.0));
    }
}
