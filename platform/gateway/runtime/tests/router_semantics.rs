//! TR-6.2: ai_router 路由语义 6 条表对 4 请求的命中顺序（AC-10 路由语义：静态→少参数→同参数长路径）
//!
//! 不启动 HTTP；纯 RouterTable 单元测试（由 #[cfg(test)] 调用）。
//! 通过 cargo test --test router_semantics 执行。

use runtime::ai_router::RouterTable;

#[test]
fn ac10_six_routes_and_four_requests_match_expectations() {
    let mut rt = RouterTable::new();
    rt.register("s3", "/a/b/c"); // 静态 3 段
    rt.register("p1_long", "/a/b/:x"); // 参数 1 段；静态段=2（a,b）
    rt.register("p1_short", "/a/:y/c"); // 参数 1 段；静态段=2（a,c）；同参同总段 → 序 tiebreak
    rt.register("p2", "/a/:y/:z"); // 参数 2 段
    rt.register("p3", "/a/:y/:z/:w"); // 参数 3 段
    rt.register("s4", "/x/y/z/w"); // 静态 4 段

    // AC-10 预期命中：
    assert_eq!(rt.match_route("/a/b/c").unwrap().handler_id, "s3");
    assert_eq!(rt.match_route("/a/b/hello").unwrap().handler_id, "p1_long");
    assert_eq!(rt.match_route("/a/foo/bar").unwrap().handler_id, "p2");
    assert_eq!(rt.match_route("/x/y/z/w").unwrap().handler_id, "s4");

    // 额外断言：s 全静态优先于任何参数路由，静态 4 段在前面注册时仍能命中。
    let params = rt.match_route("/a/b/hello").unwrap().params;
    assert_eq!(params.get("x").map(String::as_str), Some("hello"));
}

#[test]
fn priority_rules_static_absolutely_first() {
    let mut rt = RouterTable::new();
    rt.register("two_params", "/:a/:b");
    rt.register("static_two", "/a/b");
    let snap = rt.priority_snapshot();
    assert_eq!(snap[0], "static_two");
}

#[test]
fn priority_rules_fewer_params_before_more() {
    let mut rt = RouterTable::new();
    rt.register("two_p", "/a/:x/:y");
    rt.register("one_p", "/a/:x/c");
    let snap = rt.priority_snapshot();
    assert_eq!(snap[0], "one_p");
}

#[test]
fn no_match_returns_none() {
    let rt = RouterTable::new();
    assert!(rt.match_route("/nothing/here").is_none());
}
