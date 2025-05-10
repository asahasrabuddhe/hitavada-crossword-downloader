use chrono::NaiveDate;
use chrono::Datelike;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LambdaInput {
    pub date: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LambdaOutput {
    pub message: String,
    pub filename: String,
}

#[derive(Debug, PartialEq)]
pub struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format. Please use YYYY-MM-DD: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_creation() {
        let rect = Rect {
            x1: 0,
            y1: 100,
            x2: 200,
            y2: 300,
        };
        assert_eq!(rect.x1, 0);
        assert_eq!(rect.y1, 100);
        assert_eq!(rect.x2, 200);
        assert_eq!(rect.y2, 300);
    }

    #[test]
    fn test_rect_equality() {
        let rect1 = Rect {
            x1: 0,
            y1: 100,
            x2: 200,
            y2: 300,
        };
        let rect2 = Rect {
            x1: 0,
            y1: 100,
            x2: 200,
            y2: 300,
        };
        let rect3 = Rect {
            x1: 1,
            y1: 100,
            x2: 200,
            y2: 300,
        };
        assert_eq!(rect1, rect2);
        assert_ne!(rect1, rect3);
    }

    #[test]
    fn test_parse_date_valid() {
        let date_str = "2024-03-20";
        let result = parse_date(date_str);
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 3);
        assert_eq!(date.day(), 20);
    }

    #[test]
    fn test_parse_date_invalid() {
        let invalid_dates = vec![
            "2024-13-20", // Invalid month
            "2024-03-32", // Invalid day
            "2024/03/20", // Wrong format
            "not-a-date",
        ];

        for date_str in invalid_dates {
            let result = parse_date(date_str);
            assert!(result.is_err());
        }
    }
} 