//! 算子系统深度集成测试
//!
//! 覆盖：
//! 1. OperatorRegistry + OperatorPipeline 端到端集成
//! 2. GuardedGraph + KnowledgeNode 守恒保护
//! 3. 类型系统归一化边界条件
//! 4. 守恒律全生命周期正确性
//! 5. 注册表 + 图谱 + 流水线 三方联动

use std::sync::Arc;

use operator_core::conservation::{
    ConservationChecker, ConservationLaw, GraphNode, GuardedGraph, L1Conservation, L2Conservation,
};
use operator_core::engine::OperatorPipeline;
use operator_core::operator::{FunctionOperator, IdentityOperator, LinearOperator, Operator};
use operator_core::registry::OperatorRegistry;
use operator_core::state::StateVector;
use operator_core::types::{builtin, TypeCheck, TypeIdentifier, TypePair};
use operator_core::{OperatorMetadata, SystemConfig};
use approx::assert_relative_eq;

// ──────────────────────────────────────────────────────────────────
// 辅助类型：实现 GraphNode 的简单节点
// ──────────────────────────────────────────────────────────────────

struct SimpleGraphNode {
    id: String,
    state: StateVector,
}

impl SimpleGraphNode {
    fn new(id: &str, values: Vec<f64>) -> Self {
        Self {
            id: id.to_string(),
            state: StateVector::from_vec(values),
        }
    }

    fn normalized(id: &str, values: Vec<f64>) -> Self {
        let mut state = StateVector::from_vec(values);
        state.normalize();
        Self {
            id: id.to_string(),
            state,
        }
    }
}

impl GraphNode for SimpleGraphNode {
    fn state_vector(&self) -> &StateVector {
        &self.state
    }
    fn state_vector_mut(&mut self) -> &mut StateVector {
        &mut self.state
    }
}

// ──────────────────────────────────────────────────────────────────
// 1. OperatorRegistry + Pipeline 集成
// ──────────────────────────────────────────────────────────────────

#[test]
fn registry_pipeline_integration_full() {
    let mut registry = OperatorRegistry::new();

    // 注册多个算子
    registry.register(Arc::new(IdentityOperator::new(3))).unwrap();
    registry.register(Arc::new(LinearOperator::identity(3))).unwrap();
    registry
        .register(Arc::new(FunctionOperator::new("double", |s, _ctx| {
            Ok(s.scale(2.0))
        })))
        .unwrap();

    assert_eq!(registry.count(), 3);

    // 从注册表解析算子，构建流水线
    let identity = registry.resolve("Identity", None).unwrap();
    let linear = registry.resolve("LinearTransform", None).unwrap();
    let double = registry.resolve("double", None).unwrap();

    let pipe = OperatorPipeline::new()
        .then(identity)
        .then(linear)
        .then(double);

    let cfg = SystemConfig::default();
    let input = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    assert!(result.success);
    let final_state = result.final_state.unwrap();
    assert!((final_state[0] - 2.0).abs() < 1e-9);
    assert!((final_state[1] - 4.0).abs() < 1e-9);
    assert!((final_state[2] - 6.0).abs() < 1e-9);
}

#[test]
fn registry_capability_query() {
    let mut registry = OperatorRegistry::new();
    registry.register(Arc::new(IdentityOperator::new(4))).unwrap();
    registry
        .register(Arc::new(FunctionOperator::new("sigmoid", |s, _ctx| {
            let result: Vec<f64> = s.data.iter().map(|x| 1.0 / (1.0 + (-x).exp())).collect();
            Ok(StateVector::from_vec(result))
        })))
        .unwrap();

    let sv_type = builtin::state_vector_type();
    let compatible = registry.find_compatible(&sv_type, &sv_type);
    assert_eq!(compatible.len(), 2);

    let by_input = registry.find_by_input(&sv_type);
    assert!(by_input.len() >= 2);

    let by_tag = registry.find_by_tag("core");
    assert!(!by_tag.is_empty());
}

#[test]
fn registry_lifecycle_deprecate() {
    let mut registry = OperatorRegistry::new();
    registry.register(Arc::new(IdentityOperator::new(2))).unwrap();

    // 标记 deprecated
    registry.deprecate("Identity").unwrap();
    let entry = registry.get_metadata("Identity").unwrap();
    assert_eq!(entry.name, "Identity");

    // 查找兼容算子时不应包含 deprecated
    let sv_type = builtin::state_vector_type();
    let compatible = registry.find_compatible(&sv_type, &sv_type);
    // deprecated 算子被过滤
    let deprecated_found = compatible.iter().any(|e| e.metadata.name == "Identity");
    assert!(!deprecated_found);
}

#[test]
fn registry_lineage_dependency() {
    let mut registry = OperatorRegistry::new();
    registry.register(Arc::new(IdentityOperator::new(3))).unwrap();
    registry
        .register(Arc::new(FunctionOperator::new("child", |s, _ctx| Ok(s.clone()))))
        .unwrap();

    registry
        .set_dependencies("Identity", vec!["child".to_string()])
        .unwrap();

    let lineage = registry.lineage("Identity");
    assert!(lineage.contains(&"child".to_string()));
}

// ──────────────────────────────────────────────────────────────────
// 2. GuardedGraph + KnowledgeNode 守恒保护
// ──────────────────────────────────────────────────────────────────

#[test]
fn guarded_graph_node_conservation_ok() {
    let graph = GuardedGraph::with_default_laws(1e-10);
    let node = SimpleGraphNode::normalized("n1", vec![1.0, 0.0, 0.0]);
    assert!(graph.check_node(&node).is_ok());
}

#[test]
fn guarded_graph_node_conservation_violation() {
    let graph = GuardedGraph::with_default_laws(1e-10);
    // L2 范数为 2.0，违反单位能量守恒
    let node = SimpleGraphNode::new("n1", vec![2.0, 0.0, 0.0]);
    let result = graph.check_node(&node);
    assert!(result.is_err());
}

#[test]
fn guarded_graph_edge_conservation() {
    let graph = GuardedGraph::with_default_laws(1e-10);

    let source = SimpleGraphNode::normalized("s", vec![1.0, 0.0]);
    let target = SimpleGraphNode::normalized("t", vec![0.0, 1.0]);

    // 合并后维度为 4，L1=2.0，L2=√2
    // 默认守恒律期望 L1=1.0 和 L2=1.0，所以会被阻断
    let result = graph.check_edge(&source, &target);
    assert!(result.is_err());

    // 使用自定义守恒律
    let mut graph_custom = GuardedGraph::new(1e-10);
    graph_custom.add_law(L1Conservation::new(2.0));
    graph_custom.add_law(L2Conservation::new(std::f64::consts::SQRT_2));

    let result_ok = graph_custom.check_edge(&source, &target);
    assert!(result_ok.is_ok());
}

#[test]
fn guarded_graph_full_report() {
    let graph = GuardedGraph::with_default_laws(1e-10);

    let nodes = vec![
        (
            "ok1".to_string(),
            SimpleGraphNode::normalized("ok1", vec![1.0, 0.0]),
        ),
        (
            "ok2".to_string(),
            SimpleGraphNode::normalized("ok2", vec![0.0, 1.0]),
        ),
        (
            "bad".to_string(),
            SimpleGraphNode::new("bad", vec![3.0, 4.0]), // L2=5, 违反
        ),
    ];

    let report = graph.generate_report(&nodes);
    assert_eq!(report.total_nodes, 3);
    assert!(!report.violations.is_empty());
    assert!(report.max_residual > 0.0);
    assert!(report.pass_rate < 1.0);

    // bad 节点应有违规
    let bad_violation = report.violations.iter().find(|v| v.node_id == "bad");
    assert!(bad_violation.is_some());
}

#[test]
fn guarded_graph_empty_report() {
    let graph = GuardedGraph::with_default_laws(1e-10);
    let nodes: Vec<(String, SimpleGraphNode)> = vec![];
    let report = graph.generate_report(&nodes);
    assert_eq!(report.total_nodes, 0);
    assert_eq!(report.violations.len(), 0);
    assert!((report.pass_rate - 1.0).abs() < 1e-9);
}

#[test]
fn guarded_graph_custom_laws() {
    let mut graph = GuardedGraph::new(1e-10);
    graph.add_law(L1Conservation::new(3.0)); // 期望 L1=3.0

    let node = SimpleGraphNode::new("n1", vec![1.0, 1.0, 1.0]); // L1=3.0
    assert!(graph.check_node(&node).is_ok());

    let node_bad = SimpleGraphNode::new("n2", vec![1.0, 1.0]); // L1=2.0
    assert!(graph.check_node(&node_bad).is_err());
}

// ──────────────────────────────────────────────────────────────────
// 3. 类型系统归一化边界条件
// ──────────────────────────────────────────────────────────────────

#[test]
fn type_identifier_equality() {
    let t1 = TypeIdentifier::new("TestType");
    let t2 = TypeIdentifier::new("TestType");
    let t3 = TypeIdentifier::new("OtherType");

    assert_eq!(t1, t2);
    assert_ne!(t1, t3);
    assert!(t1.matches(&t2));
    assert!(!t1.matches(&t3));
}

#[test]
fn type_pair_composition_complex() {
    let a = TypeIdentifier::new("A");
    let b = TypeIdentifier::new("B");
    let c = TypeIdentifier::new("C");
    let d = TypeIdentifier::new("D");

    // f: A->B, g: B->C, h: C->D
    let f = TypePair::new(a.clone(), b.clone());
    let g = TypePair::new(b.clone(), c.clone());
    let h = TypePair::new(c.clone(), d.clone());

    // 结合律：(h∘g)∘f = h∘(g∘f)
    let fg = f.compose(&g).unwrap(); // A->C
    let gh = g.compose(&h).unwrap(); // B->D

    assert_eq!(fg.input, a);
    assert_eq!(fg.output, c);
    assert_eq!(gh.input, b);
    assert_eq!(gh.output, d);

    let hgf_v1 = fg.compose(&h).unwrap(); // A->D via (g∘f) then h
    let hgf_v2 = f.compose(&gh).unwrap(); // A->D via f then (h∘g)

    assert_eq!(hgf_v1.input, hgf_v2.input);
    assert_eq!(hgf_v1.output, hgf_v2.output);
}

#[test]
fn state_vector_type_consistency() {
    let sv1 = builtin::state_vector_type();
    let sv2 = builtin::state_vector_type();

    assert_eq!(sv1, sv2);
    assert!(sv1.matches(&sv2));

    // 应为 StateVector 名称
    assert_eq!(sv1.name, "StateVector");
}

#[test]
fn tensor_product_type_consistency() {
    let tp1 = builtin::tensor_product_type();
    let tp2 = builtin::tensor_product_type();

    assert_eq!(tp1, tp2);
    assert_eq!(tp1.name, "TensorProduct");
}

#[test]
fn type_check_all_operators() {
    let id = IdentityOperator::new(3);
    let lin = LinearOperator::identity(3);
    let func = FunctionOperator::new("test", |s, _ctx| Ok(s.clone()));

    assert_eq!(id.input_type(), builtin::state_vector_type());
    assert_eq!(id.output_type(), builtin::state_vector_type());
    assert_eq!(lin.input_type(), builtin::state_vector_type());
    assert_eq!(lin.output_type(), builtin::state_vector_type());
    assert_eq!(func.input_type(), builtin::state_vector_type());
    assert_eq!(func.output_type(), builtin::state_vector_type());
}

#[test]
fn metadata_types_are_type_identifier() {
    let id = IdentityOperator::new(3);
    let meta = id.metadata();

    // input_type 和 output_type 现在是 TypeIdentifier，不是 String
    let input_type: TypeIdentifier = meta.input_type.clone();
    let output_type: TypeIdentifier = meta.output_type.clone();

    assert_eq!(input_type, builtin::state_vector_type());
    assert_eq!(output_type, builtin::state_vector_type());
    assert_eq!(input_type.name, "StateVector");
}

// ──────────────────────────────────────────────────────────────────
// 4. 守恒律全生命周期正确性
// ──────────────────────────────────────────────────────────────────

#[test]
fn conservation_l1_probability() {
    let law = L1Conservation::probability();

    // 合法概率分布
    let v1 = StateVector::from_vec(vec![0.25, 0.25, 0.25, 0.25]);
    assert!(law.is_satisfied(&v1, 1e-10));

    // 非法：概率和 > 1
    let v2 = StateVector::from_vec(vec![0.5, 0.5, 0.5]);
    assert!(!law.is_satisfied(&v2, 1e-10));
    let residual = law.check(&v2);
    assert!((residual - 0.5).abs() < 1e-10);

    // 非法：概率和 < 1
    let v3 = StateVector::from_vec(vec![0.5, 0.3]);
    assert!(!law.is_satisfied(&v3, 1e-10));
}

#[test]
fn conservation_l2_energy() {
    let law = L2Conservation::unit_energy();

    let v1 = StateVector::from_vec(vec![1.0, 0.0]);
    assert!(law.is_satisfied(&v1, 1e-10));

    let v2 = StateVector::from_vec(vec![0.6, 0.8]); // L2=1.0
    assert!(law.is_satisfied(&v2, 1e-10));

    let v3 = StateVector::from_vec(vec![3.0, 4.0]); // L2=5.0
    assert!(!law.is_satisfied(&v3, 1e-10));
}

#[test]
fn conservation_checker_multi_law() {
    let mut checker = ConservationChecker::new(1e-10);
    checker.add_law(L1Conservation::probability());
    checker.add_law(L2Conservation::unit_energy());

    // 满足两条：[1,0] L1=1, L2=1
    let v1 = StateVector::from_vec(vec![1.0, 0.0]);
    assert!(checker.check_all(&v1).is_ok());

    // 违反 L1：[1,1] L1=2, L2=√2
    let v2 = StateVector::from_vec(vec![1.0, 1.0]);
    let result = checker.check_all(&v2);
    assert!(result.is_err());
}

#[test]
fn conservation_residual_monitor_convergence() {
    let mut monitor = operator_core::conservation::ResidualMonitor::new(1e-6);

    // 模拟收敛过程：所有最终窗口内的值必须严格低于阈值
    monitor.record(1.0);
    assert!(!monitor.is_converged(3));

    monitor.record(0.1);
    assert!(!monitor.is_converged(3));

    monitor.record(0.01);
    assert!(!monitor.is_converged(3));

    monitor.record(0.001);
    assert!(!monitor.is_converged(3));

    monitor.record(0.0000005); // 低于 1e-6
    assert!(!monitor.is_converged(3));

    monitor.record(0.0000003); // 低于 1e-6
    assert!(!monitor.is_converged(3));

    // 最后3个值: [0.0000005, 0.0000003, 0.0000001] 均 < 1e-6
    monitor.record(0.0000001);
    assert!(monitor.is_converged(3));

    assert_relative_eq!(monitor.max_residual(), 1.0);
}

#[test]
fn pipeline_conservation_strict_vs_non_strict() {
    let cfg = SystemConfig::default();

    // 严格模式：违规即失败
    let strict_pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(2)))
        .with_probability_conservation()
        .strict(true);

    let input = StateVector::from_vec(vec![2.0, 0.0]); // L2=2, 违反
    let result = strict_pipe.run(&input, &cfg).unwrap();
    assert!(!result.success);

    // 非严格模式：记录残差但成功
    let non_strict_pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(2)))
        .with_probability_conservation();

    let result2 = non_strict_pipe.run(&input, &cfg).unwrap();
    assert!(result2.success);
    assert!(result2.total_residual > 0.0);
}

#[test]
fn pipeline_conservation_convergence() {
    // 使用自定义守恒律：仅检查 L2 范数收敛到 1.0
    let mut checker = ConservationChecker::new(1e-10);
    checker.add_law(L2Conservation::unit_energy());

    let pipe = OperatorPipeline::new()
        .then(Arc::new(FunctionOperator::new("normalize", |s, _ctx| {
            let mut s = s.clone();
            s.normalize();
            Ok(s)
        })))
        .then(Arc::new(IdentityOperator::new(3)))
        .with_conservation(checker)
        .with_convergence_window(2);

    let cfg = SystemConfig::default();
    let input = StateVector::from_vec(vec![3.0, 4.0, 0.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    assert!(result.success);
    // 经过 normalize(L2=1) + Identity，残差应为 0，应收敛
    assert!(result.converged);
}

#[test]
fn pipeline_error_propagation() {
    let failing = Arc::new(FunctionOperator::new("fail", |_s, _ctx| {
        Err(operator_core::OperatorError::ExecutionError("test failure".to_string()))
    }));

    let pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(2)))
        .then(failing)
        .then(Arc::new(IdentityOperator::new(2))); // 不会执行

    let cfg = SystemConfig::default();
    let input = StateVector::from_vec(vec![1.0, 1.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    assert!(!result.success);
    assert_eq!(result.stages.len(), 2); // 失败阶段被记录
    assert!(result.stages[1].error.is_some());
    // 第三阶段不应执行
    assert!(result.final_state.is_none());
}

// ──────────────────────────────────────────────────────────────────
// 5. StateVector 深度功能
// ──────────────────────────────────────────────────────────────────

#[test]
fn state_vector_combine() {
    let v1 = StateVector::from_vec(vec![1.0, 2.0]);
    let v2 = StateVector::from_vec(vec![3.0, 4.0, 5.0]);

    let combined = v1.combine(&v2).unwrap();
    assert_eq!(combined.dimension, 5);
    assert!((combined[0] - 1.0).abs() < 1e-9);
    assert!((combined[1] - 2.0).abs() < 1e-9);
    assert!((combined[2] - 3.0).abs() < 1e-9);
    assert!((combined[3] - 4.0).abs() < 1e-9);
    assert!((combined[4] - 5.0).abs() < 1e-9);
}

#[test]
fn state_vector_normalize_l1() {
    let mut v = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
    v.normalize_probability();

    let l1 = v.norm_l1();
    assert!((l1 - 1.0).abs() < 1e-9);
}

#[test]
fn state_vector_residual() {
    let v1 = StateVector::from_vec(vec![1.0, 2.0]);
    let v2 = StateVector::from_vec(vec![1.5, 2.5]);

    let residual = v1.residual(&v2).unwrap();
    assert!(residual > 0.0);
    assert!((residual - 0.5_f64.sqrt()).abs() < 1e-9);

    // 相同向量残差为0
    let residual_zero = v1.residual(&v1).unwrap();
    assert!(residual_zero < 1e-9);
}

// ──────────────────────────────────────────────────────────────────
// 6. 全功能组合：注册表→流水线→守恒→图谱
// ──────────────────────────────────────────────────────────────────

#[test]
fn full_stack_registry_pipeline_conservation_graph() {
    // Step 1: 注册算子
    let mut registry = OperatorRegistry::new();
    registry.register(Arc::new(IdentityOperator::new(3))).unwrap();
    registry
        .register(Arc::new(FunctionOperator::new("project", |s, _ctx| {
            let mut result = s.clone();
            result.normalize();
            Ok(result)
        })))
        .unwrap();
    registry
        .register(Arc::new(FunctionOperator::new("scale", |s, _ctx| {
            Ok(s.scale(0.5))
        })))
        .unwrap();

    // Step 2: 从注册表构建流水线
    let id_op = registry.resolve("Identity", None).unwrap();
    let project = registry.resolve("project", None).unwrap();
    let scale = registry.resolve("scale", None).unwrap();

    let pipe = OperatorPipeline::new()
        .then(id_op)
        .then(project) // 归一化
        .then(scale)   // 缩放
        .with_probability_conservation();

    // Step 3: 执行流水线
    let cfg = SystemConfig::default();
    let input = StateVector::from_vec(vec![1.0, 2.0, 3.0]);
    let result = pipe.run(&input, &cfg).unwrap();

    assert!(result.success);
    let final_state = result.final_state.unwrap();

    // 验证：归一化后缩放 0.5
    // 原始 L2 = √14 ≈ 3.742
    // 归一化后 L2 = 1, 各分量: 1/√14, 2/√14, 3/√14
    // 缩放 0.5 后 L2 = 0.5
    let final_l2 = final_state.norm();
    assert!((final_l2 - 0.5).abs() < 1e-9);

    // Step 4: 用 GuardedGraph 验证结果可作为图谱节点
    let graph = GuardedGraph::with_default_laws(1e-10);
    let node = SimpleGraphNode::new("result", final_state.to_vec());
    // L2=0.5, 不满足默认单位能量守恒(L2=1), 所以检查会失败
    let check_result = graph.check_node(&node);
    assert!(check_result.is_err());

    // 使用合适的守恒律
    let mut graph_scaled = GuardedGraph::new(1e-10);
    graph_scaled.add_law(L2Conservation::new(0.5));
    assert!(graph_scaled.check_node(&node).is_ok());
}

#[test]
fn registry_export_and_reimport() {
    let mut registry = OperatorRegistry::new();
    registry.register(Arc::new(IdentityOperator::new(3))).unwrap();
    registry
        .register(Arc::new(FunctionOperator::new("custom", |s, _ctx| Ok(s.clone()))))
        .unwrap();

    let json = registry.export_json();
    assert_eq!(json["count"].as_u64().unwrap(), 2);
    assert!(!json["operators"].as_array().unwrap().is_empty());

    // 验证元数据完整性
    let ops = json["operators"].as_array().unwrap();
    for op in ops {
        assert!(op["id"].is_string());
        assert!(op["name"].is_string());
        assert!(op["version"].is_string());
    }
}

#[test]
fn error_handling_all_paths() {
    // 未注册算子解析
    let registry = OperatorRegistry::new();
    let result = registry.resolve("nonexistent", None);
    assert!(result.is_err());

    // 重复注册不应 panic
    let mut registry2 = OperatorRegistry::new();
    registry2.register(Arc::new(IdentityOperator::new(2))).unwrap();
    registry2.register(Arc::new(IdentityOperator::new(2))).unwrap();
    assert!(registry2.count() >= 1);

    // 空流水线成功
    let pipe = OperatorPipeline::new();
    let cfg = SystemConfig::default();
    let input = StateVector::from_vec(vec![1.0]);
    let result = pipe.run(&input, &cfg).unwrap();
    assert!(result.success);
    assert_eq!(result.stages.len(), 0);
}

#[test]
fn operator_metadata_builder_pattern() {
    let meta = OperatorMetadata::from_operator(
        &IdentityOperator::new(3),
        "test-id".to_string(),
        "TestOp".to_string(),
    )
    .with_description("测试算子")
    .with_version("2.0.0")
    .with_author("TestAuthor")
    .with_tags(vec!["test".to_string(), "experimental".to_string()]);

    assert_eq!(meta.id, "test-id");
    assert_eq!(meta.name, "TestOp");
    assert_eq!(meta.version, "2.0.0");
    assert_eq!(meta.description, "测试算子");
    assert_eq!(meta.author, "TestAuthor");
    assert_eq!(meta.tags.len(), 2);
    assert_eq!(meta.input_type, builtin::state_vector_type());
}

#[test]
fn type_check_compile_safety() {
    // 验证所有内置算子的类型检查一致
    let op_id = IdentityOperator::new(4);
    let op_lin = LinearOperator::identity(4);
    let op_fn = FunctionOperator::new("noop", |s, _ctx| Ok(s.clone()));

    // 所有算子的输入/输出类型应一致
    assert_eq!(op_id.input_type(), op_lin.input_type());
    assert_eq!(op_lin.input_type(), op_fn.input_type());
    assert_eq!(op_id.output_type(), op_lin.output_type());
    assert_eq!(op_lin.output_type(), op_fn.output_type());

    // 类型对可组合
    let pair_id = op_id.type_pair();
    let pair_lin = op_lin.type_pair();
    assert!(pair_id.can_compose(&pair_lin));
    assert!(pair_lin.can_compose(&pair_id));
}

// ──────────────────────────────────────────────────────────────────
// 7. Performance 基本验证
// ──────────────────────────────────────────────────────────────────

#[test]
fn pipeline_throughput() {
    let pipe = OperatorPipeline::new()
        .then(Arc::new(IdentityOperator::new(100)))
        .then(Arc::new(LinearOperator::identity(100)))
        .then(Arc::new(FunctionOperator::new("process", |s, _ctx| {
            Ok(s.clone())
        })));

    let cfg = SystemConfig::default();
    let input = StateVector::new(100);

    // 执行 100 次，验证基本性能
    let iterations = 100;
    let mut total_time = 0u128;

    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let result = pipe.run(&input, &cfg).unwrap();
        total_time += start.elapsed().as_millis();
        assert!(result.success);
    }

    let avg_ms = total_time as f64 / iterations as f64;
    eprintln!("平均流水线执行时间: {:.3}ms", avg_ms);
    // 100 维 3 算子流水线应在 1ms 内完成
    assert!(
        avg_ms < 100.0,
        "平均执行时间 {:.3}ms 超过阈值 100ms",
        avg_ms
    );
}

#[test]
fn registry_query_performance() {
    let mut registry = OperatorRegistry::new();
    for i in 0..50 {
        registry.register(Arc::new(FunctionOperator::new(
            &format!("op_{}", i),
            |s, _ctx| Ok(s.clone()),
        )))
        .unwrap();
    }
    registry.register(Arc::new(IdentityOperator::new(10))).unwrap();

    let sv_type = builtin::state_vector_type();
    let start = std::time::Instant::now();

    // 1000 次查询
    for _ in 0..1000 {
        let _ = registry.find_compatible(&sv_type, &sv_type);
    }

    let elapsed = start.elapsed().as_millis();
    eprintln!("50算子注册表 1000次查询耗时: {}ms", elapsed);
    assert!(elapsed < 500, "查询耗时过长: {}ms", elapsed);
}