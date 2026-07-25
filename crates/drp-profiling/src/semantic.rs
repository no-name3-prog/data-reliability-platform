//! Semantic type detection for string-like columns.

use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

use drp_core::SemanticType;

fn re_email() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}$").unwrap())
}

fn re_phone() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // E.164-ish or common local formats with optional separators
    RE.get_or_init(|| Regex::new(r"^\+?[\d\s().\-]{7,20}$").unwrap())
}

fn re_date() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            ^(?:
                \d{4}-\d{2}-\d{2}           # ISO date
              | \d{2}/\d{2}/\d{4}           # US/EU style
              | \d{4}/\d{2}/\d{2}
            )$
            ",
        )
        .unwrap()
    })
}

fn re_datetime() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            ^\d{4}-\d{2}-\d{2}[T\s]
            \d{2}:\d{2}(:\d{2})?
            (?:\.\d+)?
            (?:Z|[+-]\d{2}:?\d{2})?
            $
            ",
        )
        .unwrap()
    })
}

fn re_uuid() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
            .unwrap()
    })
}

fn re_url() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^https?://[^\s/$.?#].[^\s]*$").unwrap())
}

fn re_ipv4() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)$")
            .unwrap()
    })
}

/// Classify a set of non-null string samples into a semantic type + confidence.
pub fn detect_semantic_type(samples: &[&str]) -> (SemanticType, f64) {
    if samples.is_empty() {
        return (SemanticType::Unknown, 0.0);
    }
    let n = samples.len() as f64;
    let mut scores = [
        (SemanticType::Email, 0u32),
        (SemanticType::Phone, 0u32),
        (SemanticType::DateTime, 0u32),
        (SemanticType::Date, 0u32),
        (SemanticType::Uuid, 0u32),
        (SemanticType::Url, 0u32),
        (SemanticType::IpAddress, 0u32),
    ];

    for s in samples {
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        if re_email().is_match(s) {
            scores[0].1 += 1;
        }
        if re_phone().is_match(s) && s.chars().filter(|c| c.is_ascii_digit()).count() >= 7 {
            scores[1].1 += 1;
        }
        if re_datetime().is_match(s) {
            scores[2].1 += 1;
        } else if re_date().is_match(s) {
            scores[3].1 += 1;
        }
        if re_uuid().is_match(s) {
            scores[4].1 += 1;
        }
        if re_url().is_match(s) {
            scores[5].1 += 1;
        }
        if re_ipv4().is_match(s) {
            scores[6].1 += 1;
        }
    }

    let (best, count) = scores
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .unwrap_or((SemanticType::Unknown, 0));
    let conf = count as f64 / n;
    // Require a clear majority of matching samples.
    if conf >= 0.6 {
        (best, conf)
    } else {
        (SemanticType::Unknown, conf)
    }
}

/// Physical-type based fallback semantic classification.
pub fn semantic_from_physical(
    data_type: drp_common::DataType,
    distinct: u64,
    non_null: u64,
    string_samples: &[&str],
) -> (SemanticType, f64) {
    if !string_samples.is_empty() {
        let (s, c) = detect_semantic_type(string_samples);
        if s != SemanticType::Unknown {
            return (s, c);
        }
    }
    match data_type {
        drp_common::DataType::Boolean => (SemanticType::Boolean, 1.0),
        drp_common::DataType::Integer if non_null > 0 && distinct == non_null => {
            (SemanticType::IntegerId, 0.6)
        }
        drp_common::DataType::Integer | drp_common::DataType::Float => (SemanticType::Numeric, 0.8),
        drp_common::DataType::Date => (SemanticType::Date, 0.9),
        drp_common::DataType::Timestamp => (SemanticType::DateTime, 0.9),
        drp_common::DataType::String
            if non_null > 0 && (distinct as f64 / non_null as f64) < 0.1 =>
        {
            (SemanticType::Category, 0.7)
        }
        drp_common::DataType::String => (SemanticType::Text, 0.5),
        _ => (SemanticType::Unknown, 0.0),
    }
}

/// Extract display string from a JSON value for semantic checks.
pub fn value_as_str(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_email() {
        let samples = ["a@b.com", "x@y.org", "z@w.net"];
        let refs = samples.to_vec();
        let (t, c) = detect_semantic_type(&refs);
        assert_eq!(t, SemanticType::Email);
        assert!(c >= 0.9);
    }

    #[test]
    fn detects_iso_date() {
        let samples = ["2024-01-01", "2024-12-31", "2023-06-15"];
        let refs = samples.to_vec();
        let (t, _) = detect_semantic_type(&refs);
        assert_eq!(t, SemanticType::Date);
    }

    #[test]
    fn detects_phone() {
        let samples = ["+1 (555) 123-4567", "555-987-6543", "020 7946 0958"];
        let refs = samples.to_vec();
        let (t, c) = detect_semantic_type(&refs);
        assert_eq!(t, SemanticType::Phone);
        assert!(c >= 0.7);
    }
}
