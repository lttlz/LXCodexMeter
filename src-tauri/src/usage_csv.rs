use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCsvRow {
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_seconds: u64,
    pub weekly_consumed_percent: Option<f64>,
    pub five_hour_consumed_percent: Option<f64>,
    pub end_weekly_remaining_percent: Option<f64>,
    pub end_five_hour_remaining_percent: Option<f64>,
    pub is_estimated: bool,
    pub is_complete: bool,
}

pub async fn export_usage_csv(
    rows: Vec<UsageCsvRow>,
    language: String,
    file_name: String,
) -> Result<bool, String> {
    if rows.is_empty() {
        return Err(localized_error(
            &language,
            "没有可导出的日志",
            "No usage logs to export",
        ));
    }

    let contents = build_csv(&rows, &language);
    let safe_file_name = sanitize_file_name(&file_name, &language);
    let dialog_language = language.clone();
    tokio::task::spawn_blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name(&safe_file_name)
            .save_file()
        else {
            return Ok(false);
        };
        write_csv(path, contents.as_bytes(), &dialog_language)?;
        Ok(true)
    })
    .await
    .map_err(|_| localized_error(&language, "CSV 导出任务失败", "CSV export task failed"))?
}

fn write_csv(path: PathBuf, contents: &[u8], language: &str) -> Result<(), String> {
    fs::write(path, contents)
        .map_err(|_| localized_error(language, "无法保存 CSV 文件", "Failed to save the CSV file"))
}

fn sanitize_file_name(file_name: &str, language: &str) -> String {
    let trimmed = file_name.trim();
    if !trimmed.is_empty()
        && trimmed.ends_with(".csv")
        && !trimmed
            .chars()
            .any(|character| "<>:\"/\\|?*".contains(character))
    {
        return trimmed.to_string();
    }
    if language == "zh" {
        "LXCodexMeter_消耗日志.csv".to_string()
    } else {
        "LXCodexMeter_usage_log.csv".to_string()
    }
}

fn build_csv(rows: &[UsageCsvRow], language: &str) -> String {
    let headers = if language == "zh" {
        [
            "开始时间",
            "结束时间",
            "持续秒数",
            "周额度消耗百分比",
            "5h 额度消耗百分比",
            "结束时周额度余额百分比",
            "结束时 5h 额度余额百分比",
            "是否估算",
            "是否完整",
        ]
    } else {
        [
            "Start time",
            "End time",
            "Duration seconds",
            "Weekly consumed percent",
            "5h consumed percent",
            "End weekly remaining percent",
            "End 5h remaining percent",
            "Is estimated",
            "Is complete",
        ]
    };

    let mut output = String::from('\u{feff}');
    output.push_str(&headers.map(csv_escape).join(","));
    output.push_str("\r\n");
    for row in rows {
        let fields = [
            row.start_time.clone(),
            row.end_time.clone().unwrap_or_default(),
            row.duration_seconds.to_string(),
            optional_number(row.weekly_consumed_percent),
            optional_number(row.five_hour_consumed_percent),
            optional_number(row.end_weekly_remaining_percent),
            optional_number(row.end_five_hour_remaining_percent),
            row.is_estimated.to_string(),
            row.is_complete.to_string(),
        ];
        output.push_str(
            &fields
                .iter()
                .map(|field| csv_escape(field))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push_str("\r\n");
    }
    output
}

fn optional_number(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn localized_error(language: &str, zh: &str, en: &str) -> String {
    if language == "zh" { zh } else { en }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row() -> UsageCsvRow {
        UsageCsvRow {
            start_time: "2026/07/16 01:35".to_string(),
            end_time: Some("2026/07/16 01:44".to_string()),
            duration_seconds: 540,
            weekly_consumed_percent: Some(3.2),
            five_hour_consumed_percent: None,
            end_weekly_remaining_percent: Some(95.0),
            end_five_hour_remaining_percent: None,
            is_estimated: false,
            is_complete: true,
        }
    }

    #[test]
    fn chinese_csv_has_bom_crlf_numeric_and_empty_nulls_without_status() {
        let csv = build_csv(&[sample_row()], "zh");
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("开始时间,结束时间,持续秒数"));
        assert!(csv.contains(",3.2,,95,,false,true\r\n"));
        assert!(!csv.contains("状态"));
        assert_eq!(csv.matches("\r\n").count(), 2);
    }

    #[test]
    fn english_headers_and_csv_escaping_are_correct() {
        let mut row = sample_row();
        row.start_time = "a,\"b\"\nline".to_string();
        row.end_time = None;
        let csv = build_csv(&[row], "en");
        assert!(csv.contains("Start time,End time,Duration seconds"));
        assert!(csv.contains("\"a,\"\"b\"\"\nline\","));
        assert!(!csv.contains("Status"));
    }
}
