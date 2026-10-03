use chrono::{Datelike, Duration, Local, NaiveDate};

/// Parse various date formats including ROC (民國) into standard ISO YYYY-MM-DD
pub fn normalize_work_date(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Local::now().format("%Y-%m-%d").to_string();
    }

    // Try parsing standard YYYY-MM-DD, YYYY/MM/DD, YYYY.MM.DD
    let clean = trimmed.replace(['/', '.'], "-");
    if let Ok(d) = NaiveDate::parse_from_str(&clean, "%Y-%m-%d") {
        if d.year() < 1900 {
            if let Some(d2) = NaiveDate::from_ymd_opt(d.year() + 1911, d.month(), d.day()) {
                return d2.format("%Y-%m-%d").to_string();
            }
        }
        return d.format("%Y-%m-%d").to_string();
    }

    // Handle 8-digit compact Western date (e.g. "20240520")
    if trimmed.len() == 8 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%Y%m%d") {
            return d.format("%Y-%m-%d").to_string();
        }
    }

    // Handle 7-digit compact ROC date (e.g. "1130520" -> 民國113年5月20日 -> 2024-05-20)
    if trimmed.len() == 7 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        if let (Ok(roc_year), Ok(month), Ok(day)) = (
            trimmed[0..3].parse::<i32>(),
            trimmed[3..5].parse::<u32>(),
            trimmed[5..7].parse::<u32>(),
        ) {
            if let Some(d) = NaiveDate::from_ymd_opt(roc_year + 1911, month, day) {
                return d.format("%Y-%m-%d").to_string();
            }
        }
    }

    // Handle ROC (民國) formats:
    // e.g. "民國113年5月20日", "113年05月20日", "113/5/20", "113-5-20"
    let roc_str = trimmed
        .replace("民國", "")
        .replace(['年', '月'], "-")
        .replace('日', "")
        .replace('/', "-");

    let parts: Vec<&str> = roc_str
        .split('-')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() == 3 {
        if let (Ok(roc_year), Ok(month), Ok(day)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            let western_year = if roc_year < 1900 {
                roc_year + 1911
            } else {
                roc_year
            };
            if let Some(d) = NaiveDate::from_ymd_opt(western_year, month, day) {
                return d.format("%Y-%m-%d").to_string();
            }
        }
    }

    // Fallback: if not parseable, return today or the trimmed string if it looks like a date
    trimmed.to_string()
}

/// Calculate due date by adding duration_days to work_date
pub fn calculate_due_date(work_date_str: &str, duration_days: i64) -> String {
    let norm = normalize_work_date(work_date_str);
    if let Ok(date) = NaiveDate::parse_from_str(&norm, "%Y-%m-%d") {
        let due = date + Duration::days(duration_days);
        due.format("%Y-%m-%d").to_string()
    } else {
        Local::now().format("%Y-%m-%d").to_string()
    }
}

/// Calculate how many days overdue relative to today.
/// If today > due_date, returns positive integer (overdue days).
/// If today <= due_date, returns 0 or negative.
pub fn calculate_overdue_days(due_date_str: &str) -> i64 {
    let today = Local::now().date_naive();
    if let Ok(due) = NaiveDate::parse_from_str(due_date_str, "%Y-%m-%d") {
        (today - due).num_days()
    } else {
        0
    }
}

/// Parse and clean receipt ticket number: strips prefixes like "NO.", "No.", "NO:", colons, spaces,
/// extracting a pure numeric i64 identifier.
pub fn normalize_receipt_no(input: &str) -> Option<i64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<i64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_receipt_no() {
        assert_eq!(normalize_receipt_no("NO.12345678"), Some(12345678));
        assert_eq!(normalize_receipt_no("No. 888999"), Some(888999));
        assert_eq!(normalize_receipt_no("NO: 00123"), Some(123));
        assert_eq!(normalize_receipt_no("工單號：20241001"), Some(20241001));
        assert_eq!(normalize_receipt_no("654321"), Some(654321));
        assert_eq!(normalize_receipt_no(""), None);
        assert_eq!(normalize_receipt_no("   "), None);
        assert_eq!(normalize_receipt_no("NO."), None);
    }

    #[test]
    fn test_roc_date_parsing() {
        assert_eq!(normalize_work_date("民國113年5月20日"), "2024-05-20");
        assert_eq!(normalize_work_date("113年05月20日"), "2024-05-20");
        assert_eq!(normalize_work_date("113/5/20"), "2024-05-20");
        assert_eq!(normalize_work_date("113-5-20"), "2024-05-20");
        assert_eq!(normalize_work_date("2024-05-20"), "2024-05-20");
        assert_eq!(normalize_work_date("2024/05/20"), "2024-05-20");
        assert_eq!(normalize_work_date("1130520"), "2024-05-20");
        assert_eq!(normalize_work_date("20240520"), "2024-05-20");
        assert_eq!(calculate_due_date("2024-05-20", 30), "2024-06-19");
        assert_eq!(calculate_due_date("113/5/20", 30), "2024-06-19");
        assert_eq!(calculate_due_date("1130520", 30), "2024-06-19");
    }

    #[test]
    fn test_overdue_calculation() {
        // A date far in the future shouldn't be overdue (> 0)
        assert!(calculate_overdue_days("2099-01-01") <= 0);
        // A date in 2000 should definitely be overdue (> 0)
        assert!(calculate_overdue_days("2000-01-01") > 8000);
    }
}
