// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

/// 4-level Miji (密级) classification per Bell-LaPadula.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MijiLevel {
    Internal = 1,
    Secret = 2,
    Confidential = 3,
    TopSecret = 4,
}

impl MijiLevel {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
    pub fn name(self) -> &'static str {
        match self {
            MijiLevel::Internal => "Internal",
            MijiLevel::Secret => "Secret",
            MijiLevel::Confidential => "Confidential",
            MijiLevel::TopSecret => "TopSecret",
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MijiLevelConvertError {
    #[error("invalid miji level discriminant: {0}")]
    InvalidDiscriminant(u8),
}

impl TryFrom<u8> for MijiLevel {
    type Error = MijiLevelConvertError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(MijiLevel::Internal),
            2 => Ok(MijiLevel::Secret),
            3 => Ok(MijiLevel::Confidential),
            4 => Ok(MijiLevel::TopSecret),
            other => Err(MijiLevelConvertError::InvalidDiscriminant(other)),
        }
    }
}

/// Subject clearance level. Wrapper around u8 for type safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Clearance(pub u8);

#[derive(Debug, Error, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MijiError {
    #[error("Simple-Security read denied: clearance={clearance} < object={obj} (up-read forbidden)")]
    ReadUpDenied { clearance: u8, obj: u8 },
    #[error("*-Property write denied: clearance={clearance} > object={obj} (down-write forbidden)")]
    WriteStarDownDenied { clearance: u8, obj: u8 },
    #[error("Miji enforce disabled: audit reason: {reason}")]
    EnforceDisabledAudit { reason: String },
}

/// Bell-LaPadula Simple-Security (no read-up).
/// `user.0 >= obj as u8` => read allowed (down-read + same allowed; up-read forbidden).
/// When `enforce = false`, always Ok(()) but audit log notes it.
pub fn judge_read(user: Clearance, obj: MijiLevel, enforce: bool) -> Result<(), MijiError> {
    if !enforce {
        return Ok(());
    }
    let obj_v = obj.as_u8();
    if user.0 >= obj_v {
        Ok(())
    } else {
        Err(MijiError::ReadUpDenied {
            clearance: user.0,
            obj: obj_v,
        })
    }
}

/// Bell-LaPadula *-Property (no write-down).
/// `user.0 <= obj as u8` => write allowed (up-write + same allowed; down-write forbidden).
/// When `enforce = false`, always Ok(()).
pub fn judge_write(user: Clearance, obj: MijiLevel, enforce: bool) -> Result<(), MijiError> {
    if !enforce {
        return Ok(());
    }
    let obj_v = obj.as_u8();
    if user.0 <= obj_v {
        Ok(())
    } else {
        Err(MijiError::WriteStarDownDenied {
            clearance: user.0,
            obj: obj_v,
        })
    }
}
