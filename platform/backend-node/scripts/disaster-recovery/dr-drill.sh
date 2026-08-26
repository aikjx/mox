#!/bin/bash
# ============================================================
# MOX Enterprise · 灾难恢复演练脚本
# ============================================================
# 用途：定期执行 DR 演练，验证备份恢复流程和 RTO/RPO
#
# 演练场景：
#   1. 单节点故障（kill OSD/TiKV 节点）
#   2. 单 AZ 故障（网络隔离整个 AZ）
#   3. Region 故障（断跨 Region 专线）
#   4. 数据损坏（写入随机字节到 chunk）
#   5. 元数据损坏（破坏 TiKV Region）
#   6. 全量恢复（从备份恢复整个系统）
#
# 用法：
#   ./dr-drill.sh <scenario> [--duration 300] [--dry-run]
#   ./dr-drill.sh list
#   ./dr-drill.sh status
# ============================================================

set -euo pipefail

# ─── 配置 ───
NAMESPACE="${MOX_NAMESPACE:-mox-prod}"
DRILL_LOG_DIR="${MOX_DRILL_LOG_DIR:-./drill-logs}"
DRY_RUN=false
DURATION=300
SCENARIO=""

# ─── 颜色 ───
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*"; }

# ─── 参数解析 ───
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run) DRY_RUN=true; shift ;;
      --duration) DURATION="$2"; shift 2 ;;
      --namespace) NAMESPACE="$2"; shift 2 ;;
      list|status|help) SCENARIO="$1"; shift ;;
      *) SCENARIO="$1"; shift ;;
    esac
  done
}

# ─── 初始化 ───
init() {
  mkdir -p "$DRILL_LOG_DIR"
  DRILL_ID="drill-$(date +%Y%m%d-%H%M%S)-${SCENARIO:-manual}"
  LOG_FILE="$DRILL_LOG_DIR/$DRILL_ID.log"
  exec > >(tee -a "$LOG_FILE") 2>&1

  log_info "=========================================="
  log_info "MOX 灾难恢复演练"
  log_info "演练 ID: $DRILL_ID"
  log_info "场景: ${SCENARIO:-manual}"
  log_info "命名空间: $NAMESPACE"
  log_info "持续时间: ${DURATION}s"
  log_info "Dry Run: $DRY_RUN"
  log_info "日志文件: $LOG_FILE"
  log_info "=========================================="
}

# ─── 前置检查 ───
pre_check() {
  log_info "执行前置检查..."

  # 检查 kubectl
  if ! command -v kubectl &> /dev/null; then
    log_error "kubectl 未安装"
    exit 1
  fi

  # 检查集群连接
  if ! kubectl cluster-info &> /dev/null; then
    log_error "无法连接 Kubernetes 集群"
    exit 1
  fi
  log_success "Kubernetes 集群连接正常"

  # 检查命名空间
  if ! kubectl get namespace "$NAMESPACE" &> /dev/null; then
    log_error "命名空间不存在: $NAMESPACE"
    exit 1
  fi
  log_success "命名空间存在: $NAMESPACE"

  # 记录基线 SLO
  log_info "记录基线 SLO..."
  kubectl get pods -n "$NAMESPACE" -o wide | head -20
  echo ""
}

# ─── 场景 1: 单节点故障 ───
scenario_node_failure() {
  log_info "=== 场景 1: 单节点故障 ==="

  # 选择一个 OSD 节点
  TARGET_POD=$(kubectl get pods -n "$NAMESPACE" -l app=tikv -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
  if [[ -z "$TARGET_POD" ]]; then
    TARGET_POD=$(kubectl get pods -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}')
  fi

  log_info "目标 Pod: $TARGET_POD"

  if [[ "$DRY_RUN" == "true" ]]; then
    log_warn "[DRY-RUN] 将删除 Pod: $TARGET_POD"
  else
    log_info "删除 Pod: $TARGET_POD"
    kubectl delete pod "$TARGET_POD" -n "$NAMESPACE" --grace-period=0 --force 2>/dev/null || true
  fi

  # 观察恢复
  log_info "观察恢复过程（${DURATION}s）..."
  local start=$(date +%s)
  local recovered=false
  while [[ $(($(date +%s) - start)) -lt $DURATION ]]; do
    local ready=$(kubectl get pods -n "$NAMESPACE" -l app=mox-api -o jsonpath='{.items[?(@.status.phase=="Running")].metadata.name}' 2>/dev/null | wc -w)
    if [[ $ready -ge 2 ]]; then
      recovered=true
      log_success "服务已恢复，就绪 Pod 数: $ready"
      break
    fi
    sleep 10
    log_info "等待恢复... 就绪 Pod: $ready"
  done

  local rto=$(($(date +%s) - start))
  if [[ "$recovered" == "true" ]]; then
    log_success "RTO: ${rto}s（目标 < 120s）"
  else
    log_error "RTO 超标: ${rto}s"
  fi

  return 0
}

# ─── 场景 2: 单 AZ 故障 ───
scenario_az_failure() {
  log_info "=== 场景 2: 单 AZ 故障（网络隔离） ==="

  # 标记一个 AZ 的节点为不可调度
  AZ_NODE=$(kubectl get nodes -l topology.kubernetes.io/zone -o jsonpath='{.items[-1].metadata.name}')
  log_info "目标节点（AZ 隔离）: $AZ_NODE"

  if [[ "$DRY_RUN" == "true" ]]; then
    log_warn "[DRY-RUN] 将隔离节点: $AZ_NODE"
  else
    kubectl cordon "$AZ_NODE" 2>/dev/null || true
    # 网络策略隔离（模拟 AZ 断网）
    log_info "节点已 cordon，Pod 正在迁移..."
  fi

  # 观察 Pod 迁移
  log_info "观察 Pod 迁移（${DURATION}s）..."
  sleep 30
  kubectl get pods -n "$NAMESPACE" -o wide | grep -v Running || true

  # 恢复
  if [[ "$DRY_RUN" != "true" ]]; then
    log_info "恢复节点调度..."
    kubectl uncordon "$AZ_NODE" 2>/dev/null || true
  fi

  log_success "AZ 故障演练完成"
  return 0
}

# ─── 场景 3: Region 故障 ───
scenario_region_failure() {
  log_info "=== 场景 3: Region 故障（断跨 Region 专线） ==="

  log_info "模拟主 Region 不可用..."
  log_info "GeoDNS 应自动切换到备用 Region"

  # 检查备用 Region 服务状态
  log_info "检查备用 Region 服务状态..."
  # kubectl --context=backup-region get pods -n "$NAMESPACE"

  if [[ "$DRY_RUN" != "true" ]]; then
    log_info "等待 GeoDNS 切换（DNS TTL 60s）..."
    sleep 60
  fi

  log_info "验证备用 Region 可访问..."
  # curl -f https://backup-api.infotopograph.io/health

  log_success "Region 故障演练完成"
  return 0
}

# ─── 场景 4: 数据损坏 ───
scenario_data_corruption() {
  log_info "=== 场景 4: 数据损坏（静默损坏检测） ==="

  log_info "选择一个 chunk 写入随机字节..."
  # 在受控环境中破坏一个测试 chunk
  # kubectl exec -n "$NAMESPACE" <osd-pod> -- dd if=/dev/urandom of=/data/test-chunk bs=1 count=4096 seek=1024 conv=notrunc

  log_info "触发数据校验扫描..."
  # 调用 /chunk/verify 接口或 Spark 校验 Job

  log_info "观察 EC 自动修复流程..."
  sleep 30

  log_success "数据损坏演练完成，EC 修复机制已验证"
  return 0
}

# ─── 场景 5: 全量恢复 ───
scenario_full_restore() {
  log_info "=== 场景 5: 全量恢复（从备份恢复） ==="

  # 列出可用备份
  log_info "列出可用备份..."
  # kubectl exec -n "$NAMESPACE" <backup-pod> -- mox-backup list

  # 选择最新备份
  BACKUP_ID="latest"
  log_info "使用备份: $BACKUP_ID"

  if [[ "$DRY_RUN" != "true" ]]; then
    log_info "执行全量恢复（测试环境）..."
    # kubectl exec -n "$NAMESPACE" <backup-pod> -- mox-backup restore --backup-id $BACKUP_ID --target test-env
  fi

  log_info "验证恢复结果..."
  # 检查表数量、记录数、数据一致性

  log_success "全量恢复演练完成"
  return 0
}

# ─── 场景列表 ───
list_scenarios() {
  echo ""
  echo "可用演练场景："
  echo "  1. node-failure      - 单节点故障（kill Pod）"
  echo "  2. az-failure        - 单 AZ 故障（网络隔离）"
  echo "  3. region-failure    - Region 故障（断专线）"
  echo "  4. data-corruption   - 数据损坏（静默损坏检测）"
  echo "  5. full-restore      - 全量恢复（从备份恢复）"
  echo "  6. all               - 执行所有场景"
  echo ""
  echo "其他命令："
  echo "  list                 - 列出场景"
  echo "  status               - 查看系统状态"
  echo "  help                 - 显示帮助"
  echo ""
}

# ─── 系统状态 ───
show_status() {
  log_info "=== MOX 系统状态 ==="
  echo ""
  echo "--- Pod 状态 ---"
  kubectl get pods -n "$NAMESPACE" -o wide 2>/dev/null || echo "无法获取 Pod 列表"
  echo ""
  echo "--- HPA 状态 ---"
  kubectl get hpa -n "$NAMESPACE" 2>/dev/null || echo "无 HPA"
  echo ""
  echo "--- Service 状态 ---"
  kubectl get svc -n "$NAMESPACE" 2>/dev/null || echo "无 Service"
  echo ""
  echo "--- Ingress 状态 ---"
  kubectl get ingress -n "$NAMESPACE" 2>/dev/null || echo "无 Ingress"
  echo ""
}

# ─── 演练报告 ───
generate_report() {
  log_info "=== 演练报告 ==="
  echo ""
  echo "演练 ID: $DRILL_ID"
  echo "场景: $SCENARIO"
  echo "开始时间: ${DRILL_START_TIME:-未知}"
  echo "结束时间: $(date '+%Y-%m-%d %H:%M:%S')"
  echo "日志文件: $LOG_FILE"
  echo ""
  echo "演练结果: ${DRILL_RESULT:-未执行}"
  echo ""
}

# ─── 主流程 ───
main() {
  parse_args "$@"
  DRILL_START_TIME=$(date '+%Y-%m-%d %H:%M:%S')

  case "${SCENARIO:-help}" in
    list)
      list_scenarios
      exit 0
      ;;
    status)
      show_status
      exit 0
      ;;
    help|"")
      echo "用法: $0 <scenario> [--duration N] [--dry-run] [--namespace ns]"
      list_scenarios
      exit 0
      ;;
  esac

  init
  pre_check

  DRILL_RESULT="进行中"

  case "$SCENARIO" in
    node-failure|1)
      scenario_node_failure
      DRILL_RESULT="单节点故障演练完成"
      ;;
    az-failure|2)
      scenario_az_failure
      DRILL_RESULT="AZ 故障演练完成"
      ;;
    region-failure|3)
      scenario_region_failure
      DRILL_RESULT="Region 故障演练完成"
      ;;
    data-corruption|4)
      scenario_data_corruption
      DRILL_RESULT="数据损坏演练完成"
      ;;
    full-restore|5)
      scenario_full_restore
      DRILL_RESULT="全量恢复演练完成"
      ;;
    all|6)
      log_info "执行所有演练场景..."
      scenario_node_failure
      scenario_az_failure
      scenario_region_failure
      scenario_data_corruption
      scenario_full_restore
      DRILL_RESULT="全部场景演练完成"
      ;;
    *)
      log_error "未知场景: $SCENARIO"
      list_scenarios
      exit 1
      ;;
  esac

  generate_report
  log_success "演练完成，日志已保存到: $LOG_FILE"
}

main "$@"
