//! Fresh IAM authorization, held through inbox commit; no JWT role or permission cache trust.
use super::repository::StoreError;
use mox_platform_iam_core::IamRepository;
use rusqlite::{params, TransactionBehavior};

pub fn authorize_delivery<T>(
    iam: &IamRepository,
    tenant: &str,
    sender: &str,
    receivers: &[String],
    deliver: impl FnOnce() -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let mut conn = iam.conn.lock();
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    // Serialize against IAM writers, including other same-host connections. Lock order: IAM -> inbox.
    // IAM is read-only here; the inbox transaction remains the sole business commit.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let active: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM iam_tenant WHERE tenant_id=?1 AND tenant_status='active')",
        [tenant],
        |r| r.get(0),
    )?;
    if !active {
        return Err(StoreError::Forbidden);
    }
    for user in std::iter::once(sender).chain(receivers.iter().map(String::as_str)) {
        let active: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM iam_user WHERE tenant_id=?1 AND user_id=?2 AND user_status='active')",
            params![tenant,user], |r| r.get(0),
        )?;
        if !active {
            return Err(StoreError::Forbidden);
        }
    }
    if receivers.iter().any(|r| r != sender) {
        let superuser: bool = tx.query_row(
            "SELECT is_superuser=1 FROM iam_user WHERE tenant_id=?1 AND user_id=?2",
            params![tenant, sender],
            |r| r.get(0),
        )?;
        let allowed = superuser
            || mox_platform_iam_core::permission_policy::permissions(&tx, tenant, sender)?
                .iter()
                .any(|permission| permission == "message:send");
        if !allowed {
            return Err(StoreError::Forbidden);
        }
    }
    deliver()
}
