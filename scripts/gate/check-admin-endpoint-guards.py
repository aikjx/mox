# -*- coding: utf-8 -*-
"""check-admin-endpoint-guards.py — 系统管理类端点的授权守卫台账门禁（ISD-PERM-GRAN-V1.0）

判的是两件事，口径严格分开：
  A 授权（authz）：敏感前缀下的 handler 函数体里是否真的调用了授权判定
     （require_admin( / check_permission / has_role(）。
  B 身份（identity）：handler 是否吃 ApiAuth(user) 抽取器（只证明"知道调用者是谁"，
     不等于"校验了权限"）。B 单独计数、按名点名，不算通过 A。

handler 名字解析规矩（本轮踩过两次坑，都写进判据）：
  * 函数体必须按大括号配平取，绝不用固定字符窗：窗口会溢到下一个 fn，把紧邻
    require_admin 的裸放行 handler 误判成"已守卫"（假阴）。见证 = G5。
  * 同名 fn 可以跨文件存在（真实例：auth.rs:61 的 revoke_api_key 方法 vs
    system/security.rs:87 的路由 handler）。按"全局首见者"取体会把已带守卫的端点
    判成裸放行（假阳）。解析顺序 = 同目录且模块名吻合 -> 同目录唯一 -> 模块名唯一 -> AMBIGUOUS。
    见证 = G6（同名不同体必须不红）与 G7（无法消歧必须显式报 AMBIGUOUS，不许静默挑一个）。

台账规矩（与 scripts/gate/check-locale-format-outlets.py 同族）：
  * 命中集（现推的"敏感且无授权守卫"集合）必须与 LEDGER **双向相等**：
      多 = 新债（新增了一个裸放行端点），少 = 台账该删（收口了一条，必须删条目而不是写 0）。
  * 分母格：扫描文件数、解析出的 (path, handler) 对数、敏感对数——判集塌缩时硬失败，
      绝不印 "0 条 = 干净"。UNRESOLVED / AMBIGUOUS 按名点名打印（仪器的盲区可见）。
  * --check 有未登记的债就 rc=1；--selftest 跑合成语料（缺陷必须红 + 合法必须不红 + 空判集必须被拒）。
  * 本文件不接 cargo，纯文本解析，CI 里是一条独立步骤。

用法:
  python scripts/gate/check-admin-endpoint-guards.py                 # 现量报告 + 台账相等性判决
  python scripts/gate/check-admin-endpoint-guards.py --emit-ledger   # 把现推命中集打成 Python 字面量（登记用，别手抄）
  python scripts/gate/check-admin-endpoint-guards.py --identity      # 身份绑定率（B 口径）
  python scripts/gate/check-admin-endpoint-guards.py --selftest
"""
import argparse
import hashlib
import io
import os
import re
import shutil
import sys
import tempfile

SRC_REL = os.path.join('platform', 'gateway', 'mox-platform-gateway-svc', 'src')

# 敏感面：系统管理/租户/平台管理/安全凭证/SSO。/api/auth 故意不在内（登录注册刷新必须可匿名，
# 它是 --selftest 里那枚"合法形状必须不红"的正对照）。
SCOPE = ('/api/system', '/api/tenant', '/api/admin', '/api/security', '/api/sso')

AUTHZ_MARKERS = ('require_admin(', 'check_permission', 'has_role(')
IDENTITY_RE = re.compile(r'\bApiAuth\b')

ROUTE_AT = re.compile(r'\.route\(\s*"([^"]+)"')
METHOD_HANDLER = re.compile(r'\b(get|post|put|delete|patch)\(\s*([A-Za-z0-9_:]+)')
FN_DEF = re.compile(r'\bfn\s+([A-Za-z0-9_]+)(?:<[^<>()]*>)?\s*\(')
CFG_TEST = '#[cfg(test)]'


def fn_body(text, start):
    """从 fn 定义处按大括号配平取"签名+函数体"，绝不使用固定字符窗（窗口会溢到下一个 fn -> 假阴）。"""
    i = text.find('{', start)
    if i < 0:
        return ''
    depth = 0
    in_str = in_char = in_line_c = in_block_c = False
    prev = ''
    j = i
    n = len(text)
    while j < n:
        c = text[j]
        if in_line_c:
            if c == '\n':
                in_line_c = False
        elif in_block_c:
            if c == '/' and prev == '*':
                in_block_c = False
        elif in_str:
            if c == '"' and prev != '\\':
                in_str = False
        elif in_char:
            if c == "'" and prev != '\\':
                in_char = False
        else:
            if c == '/' and j + 1 < n:
                if text[j + 1] == '/':
                    in_line_c = True
                elif text[j + 1] == '*':
                    in_block_c = True
            elif c == '"':
                in_str = True
            elif c == "'":
                in_char = True
            elif c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
                if depth == 0:
                    return text[start:j + 1]
        prev = c
        j += 1
    return text[start:]


def stem(path):
    return os.path.splitext(os.path.basename(path))[0]


def build_bodies(files):
    """name -> [(rel, body)]。同名跨文件要全部留着，由 resolve_body 按声明处消歧。"""
    by_name = {}
    for rel, txt in files.items():
        for m in FN_DEF.finditer(txt):
            by_name.setdefault(m.group(1), []).append((rel, fn_body(txt, m.start())))
    return by_name


def resolve_body(by_name, declaring_rel, handler):
    """返回 (body | None, kind)。kind: unique / same-dir+mod / same-dir / mod-stem / UNRESOLVED / AMBIGUOUS"""
    parts = handler.split('::')
    name = parts[-1]
    mod = parts[-2] if len(parts) > 1 else None
    cands = by_name.get(name)
    if not cands:
        return None, 'UNRESOLVED'
    if len(cands) == 1:
        return cands[0][1], 'unique'
    d = os.path.dirname(declaring_rel)
    same_dir_mod = [c for c in cands
                    if os.path.dirname(c[0]) == d and (mod is None or stem(c[0]) == mod)]
    if len(same_dir_mod) == 1:
        return same_dir_mod[0][1], 'same-dir+mod'
    same_dir = [c for c in cands if os.path.dirname(c[0]) == d]
    if len(same_dir) == 1:
        return same_dir[0][1], 'same-dir'
    by_stem = [c for c in cands if mod and stem(c[0]) == mod]
    if len(by_stem) == 1:
        return by_stem[0][1], 'mod-stem'
    return None, 'AMBIGUOUS(%d)' % len(cands)


def parse_routes(text):
    """返回 [(path, handler_ident)]。切片法：两个 .route( 之间的文本属于第一条路由。"""
    out = []
    marks = [(m.start(), m.group(1)) for m in ROUTE_AT.finditer(text)]
    for idx, (pos, path) in enumerate(marks):
        end = marks[idx + 1][0] if idx + 1 < len(marks) else len(text)
        for _meth, handler in METHOD_HANDLER.findall(text[pos:end]):
            out.append((path, handler))
    return out


def collect_routes(files):
    """只统计"接了线的 router 构造函数"里的 .route()。

    真实例：system/approval.rs:386 的 build_approval_router 全仓零调用者，而同样的 8 条路径
    在 system/mod.rs:530 又注册了一遍（限定名形态）。若把两处都当债，同一扇门的账会翻倍。
    未接线构造函数的声明单独点名（DEADROUTER），不计入债但必须可见。
    返回 [(rel, builder, wired, path, handler)]
    """
    texts = list(files.values())
    joined = '\n'.join(texts)
    out = []
    for rel, txt in files.items():
        for m in FN_DEF.finditer(txt):
            name = m.group(1)
            body = fn_body(txt, m.start())
            if '.route(' not in body:
                continue
            pat = re.compile(r'\b%s\b' % re.escape(name))
            own = len(pat.findall(body))
            total = len(pat.findall(joined))
            wired = total > own
            for path, handler in parse_routes(body):
                out.append((rel, name, wired, path, handler))
    return out


def evaluate(files):
    """纯函数：files = {rel: 去掉 #[cfg(test)] 之后的源码} -> (解析对数, 命中列表, 未接线声明数)"""
    by_name = build_bodies(files)
    routes = collect_routes(files)
    pairs = len([r for r in routes if r[2]])
    hits = []
    dead = 0
    for rel, _builder, wired, path, handler in routes:
        if not path.startswith(SCOPE):
            continue
        if not wired:
            dead += 1
            continue
        body, kind = resolve_body(by_name, rel, handler)
        if body is None:
            hits.append((path, handler, rel, kind))
            continue
        if any(k in body for k in AUTHZ_MARKERS):
            continue
        hits.append((path, handler, rel,
                     'identity-only' if IDENTITY_RE.search(body) else 'no-guard'))
    return pairs, hits, dead


def strip_tests(text):
    i = text.find(CFG_TEST)
    return text[:i] if i >= 0 else text


def read_files(root):
    base = os.path.join(root, SRC_REL)
    if not os.path.isdir(base):
        raise SystemExit('FAIL: 找不到扫描根 %s（口径失效，不印 0）' % base)
    files = {}
    for r, _d, fs in os.walk(base):
        for f in fs:
            if f.endswith('.rs'):
                p = os.path.join(r, f)
                rel = os.path.relpath(p, base).replace(os.sep, '/')
                files[rel] = strip_tests(open(p, encoding='utf-8', errors='replace').read())
    return files


def key(h):
    return '%s|%s' % (h[0], h[1])


# LEDGER = 已知"敏感且无授权守卫"的债（path|handler）。条目必须来自 --emit-ledger 的现推输出。
# 收口一条 -> 本条从台账删除（不留 0 值条目）；新增一条裸放行 -> 门禁 rc=1 并点名它。
LEDGER = [
    "/api/admin/users/:id/sessions/:jti|auth_session::admin_revoke_session_handler",
    "/api/admin/users/:id/sessions|auth_session::admin_list_sessions_handler",
    "/api/security/status|security::security_status",
    "/api/security/validate|security::validate_api_key",
    "/api/system/approval/:id/approve|approval::approve_handler",
    "/api/system/approval/:id/history|approval::approval_history_handler",
    "/api/system/approval/:id/reject|approval::reject_handler",
    "/api/system/approval/:id|approval::approval_detail_handler",
    "/api/system/approval/definitions|approval::approval_definitions_handler",
    "/api/system/approval/pending|approval::pending_approvals_handler",
    "/api/system/approval|approval::create_approval_handler",
    "/api/system/approval|approval::list_approvals_handler",
    "/api/system/config/:id|config::delete_config_handler",
    "/api/system/config/:id|config::get_config_detail_handler",
    "/api/system/config/:id|config::update_config_handler",
    "/api/system/config/key/:key|config::get_config_by_key_handler",
    "/api/system/config/refresh-cache|config::refresh_config_cache_handler",
    "/api/system/config|config::create_config_handler",
    "/api/system/config|config::list_configs_handler",
    "/api/system/dept/:id/users|dept::list_dept_users_handler",
    "/api/system/dept/:id|dept::delete_dept_handler",
    "/api/system/dept/:id|dept::get_dept_detail_handler",
    "/api/system/dept/:id|dept::update_dept_handler",
    "/api/system/dept/tree|dept::dept_tree",
    "/api/system/dept|dept::create_dept_handler",
    "/api/system/dept|dept::list_dept",
    "/api/system/dict/data/:id|dict::delete_dict_data_handler",
    "/api/system/dict/data/:id|dict::get_dict_data_detail_handler",
    "/api/system/dict/data/:id|dict::update_dict_data_handler",
    "/api/system/dict/data/type/:dictType|dict::list_dict_data_by_type_handler",
    "/api/system/dict/data|dict::create_dict_data_handler",
    "/api/system/dict/data|dict::list_dict_data_handler",
    "/api/system/dict/type/:id|dict::delete_dict_type_handler",
    "/api/system/dict/type/:id|dict::get_dict_type_detail_handler",
    "/api/system/dict/type/:id|dict::update_dict_type_handler",
    "/api/system/dict/type/all|dict::list_all_dict_types_handler",
    "/api/system/dict/type|dict::create_dict_type_handler",
    "/api/system/dict/type|dict::list_dict_types_handler",
    "/api/system/logininfor/:id|logininfor::delete_login_log_handler",
    "/api/system/logininfor/clean|logininfor::clean_login_logs_handler",
    "/api/system/logininfor/export|logininfor::export_login_logs_handler",
    "/api/system/logininfor|logininfor::list_login_logs_handler",
    "/api/system/menu/:id|menu::delete_menu_handler",
    "/api/system/menu/:id|menu::get_menu_detail_handler",
    "/api/system/menu/:id|menu::update_menu_handler",
    "/api/system/menu/tree|menu::menu_tree",
    "/api/system/menu|menu::create_menu_handler",
    "/api/system/menu|menu::list_menus_handler",
    "/api/system/operlog/:id|operlog::delete_oper_log_handler",
    "/api/system/operlog/:id|operlog::get_oper_log_detail_handler",
    "/api/system/operlog/clean|operlog::clean_oper_logs_handler",
    "/api/system/operlog/export|operlog::export_oper_logs_handler",
    "/api/system/operlog|operlog::list_oper_logs_handler",
    "/api/system/permissions|permission::get_permissions",
    "/api/system/post/:id|post::delete_post_handler",
    "/api/system/post/:id|post::get_post_detail_handler",
    "/api/system/post/:id|post::update_post_handler",
    "/api/system/post/dept/:deptId|post::list_posts_by_dept_handler",
    "/api/system/post|post::create_post_handler",
    "/api/system/post|post::list_posts_handler",
    "/api/system/role/:id/copy|role::copy_role_handler",
    "/api/system/role/:id/dataPerms|role::get_role_data_perms_handler",
    "/api/system/role/:id/dataPerms|role::set_role_data_perms_handler",
    "/api/system/role/:id/menuPerms|role::get_role_menu_perms_handler",
    "/api/system/role/:id/menuPerms|role::set_role_menu_perms_handler",
    "/api/system/role/:id/users|role::list_role_users_handler",
    "/api/system/role/:id|role::delete_role_handler",
    "/api/system/role/:id|role::get_role",
    "/api/system/role/:id|role::update_role_handler",
    "/api/system/role|role::create_role_handler",
    "/api/system/role|role::list_roles",
    "/api/system/user/:id/changeStatus|user::change_user_status_handler",
    "/api/system/user/:id/depts|user::get_user_depts_handler",
    "/api/system/user/:id/depts|user::set_user_depts_handler",
    "/api/system/user/:id/resetPwd|user::reset_user_pwd_handler",
    "/api/system/user/:id/roles|permission::get_user_roles",
    "/api/system/user/:id/roles|user::assign_user_roles_handler",
    "/api/system/user/:id|user::delete_user_handler",
    "/api/system/user/:id|user::get_user_detail_handler",
    "/api/system/user/:id|user::update_user_handler",
    "/api/system/user|user::create_user_handler",
    "/api/system/user|user::list_users_handler",
    "/api/tenant/:id|tenant::delete_tenant_handler",
    "/api/tenant/:id|tenant::get_tenant_detail",
    "/api/tenant/:id|tenant::update_tenant_handler",
    "/api/tenant/switch/:id|tenant::switch_tenant",
    "/api/tenant|tenant::create_tenant_handler",
    "/api/tenant|tenant::list_tenants",
]

MIN_RS_FILES = 40
MIN_PAIRS = 250
MIN_SCOPED = 40


def repo_root():
    return os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))


def _flush(lines, quiet, rc):
    """任何返回路径都要先把证据打出去：失败时静默 = 没人知道门禁为什么红。"""
    if not quiet:
        for l in lines:
            print(l)
    return rc


def report(root, emit=False, quiet=False):
    files = read_files(root)
    pairs, hits, dead = evaluate(files)
    routes = collect_routes(files)
    scoped = sum(1 for r in routes if r[2] and r[3].startswith(SCOPE))
    derived = sorted(set(key(h) for h in hits))
    lines = []
    out = lines.append
    if emit:
        for k in derived:
            print('    "%s",' % k)
        print('# 现推命中 %d 条 / 敏感对数 %d / 解析对数 %d / .rs %d / 未接线声明 %d（来源：本门禁现量，勿手抄）'
              % (len(derived), scoped, pairs, len(files), dead))
        return 0
    out('扫描 %d 个 .rs（%s）' % (len(files), SRC_REL.replace(os.sep, '/')))
    out('接线 router 上的 (path,handler) 对=%d，敏感前缀对=%d，其中已带授权判定=%d'
        % (pairs, scoped, scoped - len(hits)))
    out('未接线 router 构造函数上的敏感声明=%d 条（不计债：零调用者的 build_*_router，逐条见下）' % dead)
    for rel, builder, wired, p, handler in routes:
        if not wired and p.startswith(SCOPE):
            out('  DEADROUTER[%s] %s|%s <- %s' % (builder, p, handler, rel))
    out('SCOPE 每个前缀的裸放行数：%s' % ', '.join(
        '%s=%d' % (s, sum(1 for h in hits if h[0].startswith(s))) for s in SCOPE))
    out('现推"敏感且无授权守卫"=%d 条（去重后），LEDGER=%d 条' % (len(derived), len(LEDGER)))
    blind = [h for h in hits if h[3].startswith('UNRESOLVED') or h[3].startswith('AMBIGUOUS')]
    out('仪器盲区（解析不到 handler 体，按保守记为债并点名）=%d 条' % len(blind))
    for h in sorted(blind, key=key):
        out('  BLIND[%s] %s <- %s' % (h[3], key(h), h[2]))
    for h in sorted(hits, key=key):
        out('  UNGATED[%s] %s <- %s' % (h[3], key(h), h[2]))
    if len(files) < MIN_RS_FILES:
        out('FAIL denominator: .rs 文件数=%d < %d' % (len(files), MIN_RS_FILES))
        return _flush(lines, quiet, 2)
    if pairs < MIN_PAIRS:
        out('FAIL denominator: 解析 (path,handler) 对数=%d < %d' % (pairs, MIN_PAIRS))
        return _flush(lines, quiet, 2)
    if scoped < MIN_SCOPED:
        out('FAIL denominator: 敏感前缀对数=%d < %d（SCOPE 拼错或路由改写都会走这里）' % (scoped, MIN_SCOPED))
        return _flush(lines, quiet, 2)
    extra = [k for k in derived if k not in set(LEDGER)]
    stale = [k for k in LEDGER if k not in set(derived)]
    if extra:
        out('GROWN %d 条新增裸放行（未登记）：%s' % (len(extra), ', '.join(extra[:8])))
    if stale:
        out('STALE %d 条台账已过期（该端点已有守卫或已不在 SCOPE）：%s' % (len(stale), ', '.join(stale[:8])))
    if extra or stale:
        out('RESULT FAIL（命中集与台账必须双向相等）')
        return _flush(lines, quiet, 1)
    out('RESULT PASS（命中集==台账，%d 条已登记；身份绑定率见 --identity）' % len(derived))
    return _flush(lines, quiet, 0)


def identity_report(root):
    files = read_files(root)
    by_name = build_bodies(files)
    total = bound = 0
    for rel, _builder, wired, path, handler in collect_routes(files):
        if not wired or not path.startswith(SCOPE):
            continue
        total += 1
        body, _kind = resolve_body(by_name, rel, handler)
        if body and IDENTITY_RE.search(body):
            bound += 1
            print('  BOUND %s|%s <- %s' % (path, handler, rel))
    print('敏感 (path,handler) 对=%d，其中吃 ApiAuth 身份=%d（%d/%d，未绑=%d）'
          % (total, bound, bound, total, total - bound))


def flag_files(files):
    """合成语料上的判决：返回 (命中键集, 盲区键集)"""
    _pairs, hits, _dead = evaluate(files)
    ks = set(key(h) for h in hits)
    blind = set(key(h) for h in hits if h[3].startswith('UNRESOLVED') or h[3].startswith('AMBIGUOUS'))
    return ks, blind


ROUTER = 'pub fn build_router<S>() -> Router<S> { Router::new()\n%s\n}\n'
# 夹具里的 build_router 必须"被接线"（真实仓库里的 build_*_router 都由上层 mount 调用），
# 否则会被 collect_routes 判成零调用者的死声明。G10 单独测这个区分。
WIRING = {'app/wiring.rs': 'pub fn mount_all() { let _r = build_router::<GatewayState>(); }'}


def fx(files):
    out = dict(WIRING)
    out.update(files)
    return out


def selftest():
    cases = []
    gated = ROUTER % '    .route("/api/system/user/:id/roles", put(user::assign_roles_handler))' + '''
async fn assign_roles_handler(ApiAuth(user): ApiAuth, Json(b): Json<Value>) -> Value {
    if require_admin(&user).is_some() { return err("forbidden") } ok(json!({}))
}
'''
    naked = ROUTER % (
        '    .route("/api/tenant", get(tenant::list_tenants).post(tenant::create_tenant_handler))\n'
        '    .route("/api/system/dept", get(dept::list_dept))') + '''
async fn list_tenants(Query(q): Query<Value>) -> Value { ok(json!([])) }
async fn create_tenant_handler(Json(b): Json<Value>) -> Value {
    let t = json!({ "note": "这里有一个大括号 { 在字符串里，配平必须吃掉它" });
    ok(t)
}
async fn list_dept(Query(q): Query<Value>) -> Value {
    ok(json!({ "rows": dump(|| { inner_block() }) }))
}
'''
    public_auth = ROUTER % (
        '    .route("/api/auth/login", post(auth_session::login_handler))\n'
        '    .route("/api/auth/refresh", post(auth_session::refresh_handler))') + '''
async fn login_handler(Json(b): Json<Value>) -> Value { ok(json!({})) }
async fn refresh_handler(Json(b): Json<Value>) -> Value { ok(json!({})) }
'''
    EXPECT_NAKED = {
        '/api/tenant|tenant::list_tenants',
        '/api/tenant|tenant::create_tenant_handler',
        '/api/system/dept|dept::list_dept'}

    a, _ba = flag_files(fx({'sys/user.rs': gated}))
    cases.append(('G1 已带 require_admin 的敏感端点必须不红', len(a) == 0, str(sorted(a))))
    b, _bb = flag_files(fx({'sys/tenant.rs': naked}))
    cases.append(('G2 裸放行的敏感端点必须全部红（3 条）', b == EXPECT_NAKED, str(sorted(b))))
    c, _bc = flag_files(fx({'sys/auth_session.rs': public_auth}))
    cases.append(('G3 匿名 auth 路由属正对照，必须不红', len(c) == 0, str(sorted(c))))
    scoped_pairs = sum(1 for p, _h in parse_routes(public_auth) if p.startswith(SCOPE))
    cases.append(('G4 判集塌缩必须被拒（SCOPE 在语料里 0 条 -> 不可当"干净"）',
                  scoped_pairs == 0, 'scoped=%d' % scoped_pairs))
    # G5：大括号配平的见证。裸放行的最后一条 handler 与"带 require_admin 的 handler"**写在同一份文件文本里**
    #      （真实仓库就是这个形状：一个 .rs 里连排多个 handler）。固定字符窗会溢出到下一个 fn
    #      => list_dept 白捡一个守卫、少咬一条；配平取体则只咬裸放行那 3 条。
    #      反面教训（本轮实测）：把两份夹具拆成两个 key 会让窗口变异体照样绿 => 见证必须有同一文本里的邻接。
    real, _br = flag_files(fx({'sys/mixed.rs': naked + gated}))
    cases.append(('G5 混合语料上只咬裸放行那 3 条（函数体不得溢到相邻 fn）',
                  real == EXPECT_NAKED, str(sorted(real))))
    # G6：同名 fn 跨文件（真实例 auth.rs:61 revoke_api_key 方法 vs system/security.rs:87 handler）。
    #      路由声明在 security.rs 且 handler 体带 require_admin => 必须不红。
    other_file = 'impl AuthState { pub fn revoke_api_key(&self, k: &str) { self.mem.remove(k); } }\n'
    routed_guard = ROUTER % '    .route("/api/security/api-keys/:id", delete(security::revoke_api_key))' + '''
pub(crate) async fn revoke_api_key(ApiAuth(user): ApiAuth, Path(id): Path<String>) -> Value {
    if let Some(resp) = require_admin(&user) { return resp } ok(json!({ "revoked": id }))
}
'''
    g6, _g6b = flag_files(fx({'system/auth.rs': other_file, 'system/security.rs': routed_guard}))
    cases.append(('G6 同名不同体的 handler 必须按声明处解析（已守卫的不得判成裸放行）',
                  len(g6) == 0, str(sorted(g6))))
    # G7：同一目录里两处同名 handler（模块名对不上、同目录候选 >1）时必须显式报 AMBIGUOUS，
    #      不许静默挑一个当成判决。
    ambiguous = ROUTER % '    .route("/api/admin/w/id", delete(other_ns::revoke_api_key))'
    _g7, g7b = flag_files(fx({'admin/router.rs': ambiguous,
                              'admin/holder.rs': other_file,
                              'admin/holder2.rs': other_file.replace('AuthState', 'AuthState2')}))
    cases.append(('G7 同名两处且都不同目录时必须报 AMBIGUOUS（盲区可见，不许静默取首见）',
                  g7b == {'/api/admin/w/id|other_ns::revoke_api_key'}, str(sorted(g7b))))
    # G10：零调用者的 build_*_router 上的敏感声明不得计债（真实例 system/approval.rs:386 与
    #       system/mod.rs:530 把同 8 条路径各注册一遍，两处都算会让同一扇门的账翻倍）。
    dead_router = ('pub fn build_dead_router<S>() -> Router<S> { Router::new()\n'
                   '    .route("/api/tenant/dead", get(tenant::dead_list))\n}\n'
                   'async fn dead_list(Query(q): Query<Value>) -> Value { ok(json!([])) }\n')
    _p10, hits10, dead10 = evaluate(fx({'sys/dead.rs': dead_router, 'sys/tenant.rs': naked}))
    cases.append(('G10 未接线 router 的敏感声明必须排除出题（且被按名计入 dead）',
                  dead10 == 1 and '/api/tenant/dead|tenant::dead_list' not in set(key(h) for h in hits10),
                  'dead=%d hits=%s' % (dead10, sorted(key(h) for h in hits10))))
    # G8/G9：失败路径必须把证据打到 stdout（曾经只 return 1/2 而一个字都不印，
    #        门禁红得无声无息 = 等于没有门禁）。夹具落在临时目录，跑完即删。
    NAMES = ('MIN_RS_FILES', 'MIN_PAIRS', 'MIN_SCOPED')

    def run_report(files, denominators):
        root = tempfile.mkdtemp(prefix='isd-gate-')
        base = os.path.join(root, *SRC_REL.split(os.sep))
        os.makedirs(base)
        for rel, txt in files.items():
            p = os.path.join(base, *rel.split('/'))
            d = os.path.dirname(p)
            if d and not os.path.isdir(d):
                os.makedirs(d)
            open(p, 'w', encoding='utf-8', newline='\n').write(txt)
        saved = tuple(globals()[k] for k in NAMES)
        old = sys.stdout
        try:
            for k, v in zip(NAMES, denominators):
                globals()[k] = v
            buf = io.StringIO()
            sys.stdout = buf
            rc = report(root)
            sys.stdout = old
            text = buf.getvalue()
        finally:
            sys.stdout = old
            for k, v in zip(NAMES, saved):
                globals()[k] = v
            shutil.rmtree(root, ignore_errors=True)
        return rc, text

    rc8, t8 = run_report(fx({'sys/tenant.rs': naked}), (1, 1, 1))
    cases.append(('G8 台账不符而失败时必须打印逐条证据（不许静默 rc=1）',
                  rc8 != 0 and 'UNGATED[' in t8 and 'RESULT FAIL' in t8,
                  'rc=%s bytes=%d' % (rc8, len(t8.encode('utf-8')))))
    rc9, t9 = run_report(fx({'sys/tenant.rs': naked}), (1000, 1, 1))
    cases.append(('G9 分母塌缩而失败时同样必须打印（不许静默 rc=2）',
                  rc9 == 2 and 'FAIL denominator' in t9,
                  'rc=%s bytes=%d' % (rc9, len(t9.encode('utf-8')))))
    ok = True
    for name, passed, detail in cases:
        print('%s %s  %s' % ('PASS' if passed else 'FAIL', name, '' if passed else 'detail=' + detail[:200]))
        ok = ok and passed
    print('SELFTEST %s (%d 例) LEDGER 条目=%d 门禁自身 sha=%s' % (
        'PASS' if ok else 'FAIL', len(cases), len(LEDGER),
        hashlib.md5(open(os.path.abspath(__file__), 'rb').read()).hexdigest()[:16]))
    return 0 if ok else 1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--emit-ledger', action='store_true')
    ap.add_argument('--identity', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    a = ap.parse_args()
    root = repo_root()
    if a.selftest:
        return selftest()
    if a.identity:
        identity_report(root)
        return 0
    return report(root, emit=a.emit_ledger)


if __name__ == '__main__':
    sys.exit(main())
