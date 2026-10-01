# -*- coding: utf-8 -*-
"""registry.rs：给所有 handler 接入 TenantId，读面取内层 map、写面用 entry，并补 save/audit/enforce 的 tenant。"""
import io

p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_registry.rs"
s = io.open(p, "r", encoding="utf-8").read()

def rep(old, new, expect=1):
    global s
    n = s.count(old)
    assert n == expect, f"expected {expect}, got {n} for: {old[:60]!r}"
    s = s.replace(old, new)
    print("ok:", old.splitlines()[0][:50])

# ---- create_expert ----
rep(
'''async fn create_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&s, &user, RbacAction::RegisterExpert) {
        return resp;
    }''',
'''async fn create_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&s, &user, tenant.as_str(), RbacAction::RegisterExpert) {
        return resp;
    }''')

rep(
'''    let mut reg = s.registry.lock();
    if reg.contains_key(&id) {
        return err(400, format!("expert id already exists: {}", id));
    }''',
'''    let mut all = s.registry.lock();
    let reg = all.entry(tenant.0.clone()).or_default();
    if reg.contains_key(&id) {
        return err(400, format!("expert id already exists: {}", id));
    }''')

rep(
'''    reg.insert(id.clone(), exp.clone());
    save_registry(&reg);

    emit_audit(&s, &actor_from_opt_user(&user), AuditAction::Unknown("expert.register".into()), "expert", &id, AuditOutcome::Success, Some(&format!("name={}", exp.name)));''',
'''    reg.insert(id.clone(), exp.clone());
    save_registry(tenant.as_str(), reg);

    emit_audit(&s, &actor_from_opt_user(&user), tenant.as_str(), AuditAction::Unknown("expert.register".into()), "expert", &id, AuditOutcome::Success, Some(&format!("name={}", exp.name)));''')

# ---- update_expert ----
rep(
'''async fn update_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&s, &user, RbacAction::UpdateExpert) {
        return resp;
    }

    let mut reg = s.registry.lock();
    match reg.get_mut(&id) {
        Some(exp) if exp.enabled => {
            merge_expert_from_value(exp, &body);
            exp.updated_at = now_iso();
            let updated = exp.clone();
            save_registry(&reg);
            emit_audit(&s, &actor_from_opt_user(&user), AuditAction::Unknown("expert.update".into()), "expert", &id, AuditOutcome::Success, Some(&format!("name={}", updated.name)));''',
'''async fn update_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&s, &user, tenant.as_str(), RbacAction::UpdateExpert) {
        return resp;
    }

    let mut all = s.registry.lock();
    let reg = all.entry(tenant.0.clone()).or_default();
    match reg.get_mut(&id) {
        Some(exp) if exp.enabled => {
            merge_expert_from_value(exp, &body);
            exp.updated_at = now_iso();
            let updated = exp.clone();
            save_registry(tenant.as_str(), reg);
            emit_audit(&s, &actor_from_opt_user(&user), tenant.as_str(), AuditAction::Unknown("expert.update".into()), "expert", &id, AuditOutcome::Success, Some(&format!("name={}", updated.name)));''')

# ---- delete_expert ----
rep(
'''async fn delete_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&s, &user, RbacAction::DeleteExpert) {
        return resp;
    }

    let mut reg = s.registry.lock();
    match reg.get_mut(&id) {
        Some(exp) => {
            exp.enabled = false;
            exp.updated_at = now_iso();
            exp.metadata.insert("deleted_at".into(), json!(now_iso()));
            save_registry(&reg);
            emit_audit(&s, &actor_from_opt_user(&user), AuditAction::Unknown("expert.disable".into()), "expert", &id, AuditOutcome::Success, Some("soft_delete"));''',
'''async fn delete_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&s, &user, tenant.as_str(), RbacAction::DeleteExpert) {
        return resp;
    }

    let mut all = s.registry.lock();
    let reg = all.entry(tenant.0.clone()).or_default();
    match reg.get_mut(&id) {
        Some(exp) => {
            exp.enabled = false;
            exp.updated_at = now_iso();
            exp.metadata.insert("deleted_at".into(), json!(now_iso()));
            save_registry(tenant.as_str(), reg);
            emit_audit(&s, &actor_from_opt_user(&user), tenant.as_str(), AuditAction::Unknown("expert.disable".into()), "expert", &id, AuditOutcome::Success, Some("soft_delete"));''')

# ---- list_capabilities (read) ----
rep(
'''async fn list_capabilities(
    State(s): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();''',
'''async fn list_capabilities(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let all = s.registry.lock();
    let reg = all.get(tenant.as_str()).unwrap_or(empty_registry());''')

# ---- platform_metrics (read) ----
rep(
'''async fn platform_metrics(
    State(s): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let metrics = compute_platform_metrics(&reg);''',
'''async fn platform_metrics(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let all = s.registry.lock();
    let reg = all.get(tenant.as_str()).unwrap_or(empty_registry());
    let metrics = compute_platform_metrics(reg);''')

# ---- platform_overview (read) ----
rep(
'''async fn platform_overview(
    State(s): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let sessions = s.sessions.lock();''',
'''async fn platform_overview(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let all = s.registry.lock();
    let reg = all.get(tenant.as_str()).unwrap_or(empty_registry());
    let sessions = s.sessions.lock();''')

# ---- expert_metrics (read) ----
rep(
'''async fn expert_metrics(
    State(s): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let exp = match reg.get(&id) {''',
'''async fn expert_metrics(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let all = s.registry.lock();
    let reg = all.get(tenant.as_str()).unwrap_or(empty_registry());
    let exp = match reg.get(&id) {''')

# ---- experts_stats_real (read) ----
rep(
'''async fn experts_stats_real(
    State(s): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let m = compute_platform_metrics(&reg);''',
'''async fn experts_stats_real(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let all = s.registry.lock();
    let reg = all.get(tenant.as_str()).unwrap_or(empty_registry());
    let m = compute_platform_metrics(reg);''')

# ---- consult_room_real (read) ----
rep(
'''async fn consult_room_real(
    State(s): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let expert = reg.get(&id).cloned();''',
'''async fn consult_room_real(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let all = s.registry.lock();
    let reg = all.get(tenant.as_str()).unwrap_or(empty_registry());
    let expert = reg.get(&id).cloned();''')

# ---- join_team_real (read) ----
rep(
'''async fn join_team_real(
    State(s): State<Arc<ExpertsSharedState>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {''',
'''async fn join_team_real(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {''')

rep(
'''    let (status, verified_expert_id) = if let Some(eid) = &expert_id {
        let reg = s.registry.lock();
        match reg.get(eid) {''',
'''    let (status, verified_expert_id) = if let Some(eid) = &expert_id {
        let all = s.registry.lock();
        let reg = all.get(tenant.as_str()).unwrap_or(empty_registry());
        match reg.get(eid) {''')

# ---- consult_now_real (read + write counters) ----
rep(
'''async fn consult_now_real(
    State(s): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {''',
'''async fn consult_now_real(
    State(s): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {''')

rep(
'''    // 验证专家存在且在线
    let expert_online = {
        let reg = s.registry.lock();
        match reg.get(&id) {''',
'''    // 验证专家存在且在线（按租户）
    let expert_online = {
        let all = s.registry.lock();
        let reg = all.get(tenant.as_str()).unwrap_or(empty_registry());
        match reg.get(&id) {''')

rep(
'''    //  increment expert consultation counters
    {
        let mut reg = s.registry.lock();
        if let Some(exp) = reg.get_mut(&id) {
            exp.metrics.total_consultations += 1;
            exp.metrics.today_consultations += 1;
            exp.availability.current_load += 1;
            exp.availability.last_active = now.clone();
            save_registry(&reg);
        }
    }

    emit_audit(&s, &actor_from_opt_user(&user), AuditAction::Unknown("expert.consult_now".into()), "session", &session_id, AuditOutcome::Success, Some(&format!("expert_id={}, topic={}", id, topic)));''',
'''    //  increment expert consultation counters（按租户）
    {
        let mut all = s.registry.lock();
        let reg = all.entry(tenant.0.clone()).or_default();
        if let Some(exp) = reg.get_mut(&id) {
            exp.metrics.total_consultations += 1;
            exp.metrics.today_consultations += 1;
            exp.availability.current_load += 1;
            exp.availability.last_active = now.clone();
            save_registry(tenant.as_str(), reg);
        }
    }

    emit_audit(&s, &actor_from_opt_user(&user), tenant.as_str(), AuditAction::Unknown("expert.consult_now".into()), "session", &session_id, AuditOutcome::Success, Some(&format!("expert_id={}, topic={}", id, topic)));''')

io.open(p, "w", encoding="utf-8", newline="").write(s)
print("ALL DONE")
