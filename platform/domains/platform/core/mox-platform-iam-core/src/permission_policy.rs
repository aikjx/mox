//! Shared effective permission policy: fresh database state, active tenant/user/role/permission.
use rusqlite::{params, Connection};

pub fn permissions(conn: &Connection, tenant: &str, user: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt=conn.prepare("WITH RECURSIVE roles(id) AS (
        SELECT r.role_id FROM iam_user_role ur JOIN iam_role r ON r.role_id=ur.role_id
        WHERE ur.tenant_id=?1 AND ur.user_id=?2 AND r.tenant_id IN (?1,'system') AND r.status='active'
        UNION
        SELECT r.role_id FROM roles child JOIN iam_role_inherit i ON i.child_role_id=child.id
        JOIN iam_role r ON r.role_id=i.parent_role_id
        WHERE i.tenant_id IN (?1,'system') AND r.tenant_id IN (?1,'system') AND r.status='active'
        UNION
        SELECT parent.role_id FROM roles child JOIN iam_role r ON r.role_id=child.id
        JOIN iam_role parent ON parent.role_id=r.parent_id
        WHERE parent.tenant_id IN (?1,'system') AND parent.status='active'
    ) SELECT DISTINCT p.perm_code FROM iam_permission p
        WHERE p.tenant_id IN (?1,'system') AND p.status='active'
        AND EXISTS(SELECT 1 FROM iam_user u JOIN iam_tenant t ON t.tenant_id=u.tenant_id
        WHERE u.tenant_id=?1 AND u.user_id=?2 AND u.user_status='active' AND t.tenant_status='active') AND (
        EXISTS(SELECT 1 FROM iam_user WHERE tenant_id=?1 AND user_id=?2 AND is_superuser=1)
        OR EXISTS(SELECT 1 FROM roles JOIN iam_role_permission rp ON rp.role_id=roles.id
        WHERE rp.tenant_id IN (?1,'system') AND rp.perm_id=p.perm_id)) ORDER BY p.perm_code")?;
    let values = stmt.query_map(params![tenant, user], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(values)
}
