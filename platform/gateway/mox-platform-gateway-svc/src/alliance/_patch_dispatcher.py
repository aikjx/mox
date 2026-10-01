# -*- coding: utf-8 -*-
"""experts_dispatcher.rs：自由函数与 handler 接入 tenant，registry 读面取内层、写面 entry。"""
import io

p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_dispatcher.rs"
s = io.open(p, "r", encoding="utf-8").read()

def rep(old, new, expect=1):
    global s
    n = s.count(old)
    assert n == expect, f"expected {expect}, got {n}: {old[:70]!r}"
    s = s.replace(old, new)
    print("ok:", old.splitlines()[0][:55])

# --- 自由函数 dispatch_task 加 tenant ---
rep(
'''pub fn dispatch_task(
    state: &ExpertsSharedState,
    _task_type: &str,
    input: &str,
    specified_ids: Option<Vec<String>>,
) -> (Vec<String>, HashMap<String, f64>, String) {
    let config = state.dispatcher_config.lock().clone();
    let registry = state.registry.lock();''',
'''pub fn dispatch_task(
    state: &ExpertsSharedState,
    tenant: &str,
    _task_type: &str,
    input: &str,
    specified_ids: Option<Vec<String>>,
) -> (Vec<String>, HashMap<String, f64>, String) {
    let config = state.dispatcher_config.lock().clone();
    let all_reg = state.registry.lock();
    let registry = all_reg.get(tenant).map(|m| m).unwrap_or(empty_registry());''')

# --- 自由函数 dispatch_n_experts 加 tenant ---
rep(
'''fn dispatch_n_experts(
    state: &ExpertsSharedState,
    input: &str,
    n: usize,
) -> (Vec<String>, HashMap<String, f64>) {
    let config = state.dispatcher_config.lock().clone();
    let registry = state.registry.lock();''',
'''fn dispatch_n_experts(
    state: &ExpertsSharedState,
    tenant: &str,
    input: &str,
    n: usize,
) -> (Vec<String>, HashMap<String, f64>) {
    let config = state.dispatcher_config.lock().clone();
    let all_reg = state.registry.lock();
    let registry = all_reg.get(tenant).map(|m| m).unwrap_or(empty_registry());''')

# --- update_config：签名加 TenantId + enforce 加 tenant ---
rep(
'''async fn update_config(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<UpdateConfigBody>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：调度配置写属管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&state, &user, RbacAction::UpdateConfig) {''',
'''async fn update_config(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<UpdateConfigBody>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：调度配置写属管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::UpdateConfig) {''')

# --- dispatcher_status：签名 + 克隆租户内层 ---
rep(
'''async fn dispatcher_status(
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let config = state.dispatcher_config.lock().clone();
    let records = state.dispatch_records.lock().clone();
    let registry = state.registry.lock().clone();''',
'''async fn dispatcher_status(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
) -> ApiResponse<Value> {
    let config = state.dispatcher_config.lock().clone();
    let records = state.dispatch_records.lock().clone();
    let all_reg = state.registry.lock();
    let registry = all_reg.get(tenant.as_str()).cloned().unwrap_or_default();''')

# --- dispatch handler ---
rep(
'''async fn dispatch(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<DispatchBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");

    let (assigned_ids, match_scores, strategy_used) =
        dispatch_task(&state, &body.task_type, &body.input, body.expert_ids.clone());''',
'''async fn dispatch(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<DispatchBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");

    let (assigned_ids, match_scores, strategy_used) =
        dispatch_task(&state, tenant.as_str(), &body.task_type, &body.input, body.expert_ids.clone());''')
rep(
'''    crate::alliance::experts_common::emit_audit(&state, &actor_from_opt_user(&user), AuditAction::ExpertDispatch, "dispatch", &dispatch_id, AuditOutcome::Success, Some(&format!("task_type={}, assigned={:?}", body.task_type, assigned_ids)));

    // 构造 assigned_experts 详情
    let registry = state.registry.lock();''',
'''    crate::alliance::experts_common::emit_audit(&state, &actor_from_opt_user(&user), tenant.as_str(), AuditAction::ExpertDispatch, "dispatch", &dispatch_id, AuditOutcome::Success, Some(&format!("task_type={}, assigned={:?}", body.task_type, assigned_ids)));

    // 构造 assigned_experts 详情
    let all_reg = state.registry.lock();
    let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());''')

# --- consult handler ---
rep(
'''async fn consult(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<ConsultBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");

    let (assigned_ids, match_scores, strategy_used) =
        dispatch_task(&state, "consult", &body.question, None);

    if assigned_ids.is_empty() {
        return err(503, "no available experts for consult".to_string());
    }

    let expert_id = &assigned_ids[0];
    let registry = state.registry.lock();''',
'''async fn consult(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<ConsultBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");

    let (assigned_ids, match_scores, strategy_used) =
        dispatch_task(&state, tenant.as_str(), "consult", &body.question, None);

    if assigned_ids.is_empty() {
        return err(503, "no available experts for consult".to_string());
    }

    let expert_id = &assigned_ids[0];
    let all_reg = state.registry.lock();
    let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());''')
rep(
'''    crate::alliance::experts_common::emit_audit(&state, &actor_from_opt_user(&user), AuditAction::Unknown("expert.consult".into()), "consult", &dispatch_id, AuditOutcome::Success, Some(&format!("expert_id={}", expert.id)));''',
'''    crate::alliance::experts_common::emit_audit(&state, &actor_from_opt_user(&user), tenant.as_str(), AuditAction::Unknown("expert.consult".into()), "consult", &dispatch_id, AuditOutcome::Success, Some(&format!("expert_id={}", expert.id)));''')

# --- multi_consult handler ---
rep(
'''async fn multi_consult(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<MultiConsultBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");
    let max_experts = body.max_experts.unwrap_or(3).max(1);

    let (assigned_ids, match_scores) = dispatch_n_experts(&state, &body.question, max_experts);

    if assigned_ids.is_empty() {
        return err(503, "no available experts for multi-consult".to_string());
    }

    let registry = state.registry.lock();''',
'''async fn multi_consult(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Json(body): Json<MultiConsultBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");
    let max_experts = body.max_experts.unwrap_or(3).max(1);

    let (assigned_ids, match_scores) = dispatch_n_experts(&state, tenant.as_str(), &body.question, max_experts);

    if assigned_ids.is_empty() {
        return err(503, "no available experts for multi-consult".to_string());
    }

    let all_reg = state.registry.lock();
    let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());''')
rep(
'''    crate::alliance::experts_common::emit_audit(&state, &actor_from_opt_user(&user), AuditAction::Unknown("expert.multi_consult".into()), "consult", &dispatch_id, AuditOutcome::Success, Some(&format!("experts={:?}", assigned_ids)));''',
'''    crate::alliance::experts_common::emit_audit(&state, &actor_from_opt_user(&user), tenant.as_str(), AuditAction::Unknown("expert.multi_consult".into()), "consult", &dispatch_id, AuditOutcome::Success, Some(&format!("experts={:?}", assigned_ids)));''')

# --- reset_expert ---
rep(
'''async fn reset_expert(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
    Json(body): Json<ResetBody>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：调度负载重置属管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&state, &user, RbacAction::ResetDispatcher) {
        return resp;
    }

    let now = now_iso();
    let (previous_load, previous_failures) = {
        let registry = state.registry.lock();''',
'''async fn reset_expert(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(id): Path<String>,
    Json(body): Json<ResetBody>,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：调度负载重置属管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::ResetDispatcher) {
        return resp;
    }

    let now = now_iso();
    let (previous_load, previous_failures) = {
        let all_reg = state.registry.lock();
        let registry = all_reg.get(tenant.as_str()).unwrap_or(empty_registry());''')
rep(
'''    // 清零注册表中的 current_load
    {
        let mut registry = state.registry.lock();
        if let Some(expert) = registry.get_mut(&id) {''',
'''    // 清零注册表中的 current_load（按租户）
    {
        let mut all_reg = state.registry.lock();
        let registry = all_reg.entry(tenant.0.clone()).or_default();
        if let Some(expert) = registry.get_mut(&id) {''')

# --- reset_all ---
rep(
'''async fn reset_all(
    State(state): State<Arc<ExpertsSharedState>>,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：全量重置属管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&state, &user, RbacAction::ResetAllDispatcher) {
        return resp;
    }

    let now = now_iso();
    let reset_ids: Vec<String> = {
        let mut registry = state.registry.lock();''',
'''async fn reset_all(
    State(state): State<Arc<ExpertsSharedState>>,
    TenantId(tenant): TenantId,
    OptionalAuthUser(user): OptionalAuthUser,
) -> ApiResponse<Value> {
    // R1 后端 RBAC 强制：全量重置属管理写面，需 super_admin / tenant_admin
    if let Err(resp) = enforce_admin_or_respond(&state, &user, tenant.as_str(), RbacAction::ResetAllDispatcher) {
        return resp;
    }

    let now = now_iso();
    let reset_ids: Vec<String> = {
        let mut all_reg = state.registry.lock();
        let registry = all_reg.entry(tenant.0.clone()).or_default();''')

io.open(p, "w", encoding="utf-8", newline="").write(s)
print("ALL DONE")
