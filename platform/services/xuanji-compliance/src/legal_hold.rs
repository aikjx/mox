use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegalHold {
    pub placed_by: String,
    pub placed_at_ms: i64,
    pub hold_until_ms: i64,
}

#[derive(Debug, Error, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LHError {
    #[error("LegalHold still held by {placed_by} until {hold_until_ms}ms (now={now_ms}ms, operation={op})")]
    StillHeld {
        placed_by: String,
        hold_until_ms: i64,
        now_ms: i64,
        op: String,
    },
}

pub fn check_delete(hold: Option<&LegalHold>, now_ms: i64) -> Result<(), LHError> {
    match hold {
        None => Ok(()),
        Some(h) => {
            if now_ms >= h.hold_until_ms {
                Ok(())
            } else {
                Err(LHError::StillHeld {
                    placed_by: h.placed_by.clone(),
                    hold_until_ms: h.hold_until_ms,
                    now_ms,
                    op: "delete".to_string(),
                })
            }
        }
    }
}

pub fn check_overwrite(hold: Option<&LegalHold>, now_ms: i64) -> Result<(), LHError> {
    match hold {
        None => Ok(()),
        Some(h) => {
            if now_ms >= h.hold_until_ms {
                Ok(())
            } else {
                Err(LHError::StillHeld {
                    placed_by: h.placed_by.clone(),
                    hold_until_ms: h.hold_until_ms,
                    now_ms,
                    op: "overwrite".to_string(),
                })
            }
        }
    }
}

pub fn parse_cli_hold_until(date_str: &str) -> Result<i64, String> {
    chrono::DateTime::parse_from_rfc3339(date_str)
        .map(|dt| dt.timestamp_millis())
        .map_err(|e| format!("invalid hold_until date '{}': {}", date_str, e))
}
