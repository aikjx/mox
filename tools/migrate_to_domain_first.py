#!/usr/bin/env python3
"""
infotopograph 目录结构二次迁移：按层组织 → 按域组织（域内分层）
最优企业级架构：一个业务域的所有代码(api/svcapi/core/svc/sdk)在一个目录下
"""
import shutil
from pathlib import Path

ROOT = Path(r"D:\a10\aikjx\gitcode\infotopograph")
P = ROOT / "platform"

# 映射表: (当前路径, 目标路径)
# 跨域共享层(foundation/framework/gateway)保留在顶层
# 业务域代码全部移入 domains/<domain>/<layer>/
MIGRATIONS = [
    # === KG 知识图谱域 ===
    ("core/kg/mox-kg-algo-core", "domains/kg/core/mox-kg-algo-core"),
    ("core/kg/mox-kg-meta-core", "domains/kg/core/mox-kg-meta-core"),
    ("services/kg/mox-kg-storage-svc", "domains/kg/svc/mox-kg-storage-svc"),
    ("services/kg/mox-kg-service-svc", "domains/kg/svc/mox-kg-service-svc"),
    ("services/kg/mox-kg-streams-svc", "domains/kg/svc/mox-kg-streams-svc"),
    ("services/kg/mox-kg-spark-svc", "domains/kg/svc/mox-kg-spark-svc"),
    ("services/kg/mox-kg-hub-svc", "domains/kg/svc/mox-kg-hub-svc"),
    ("services/kg/mox-kg-fusion-svc", "domains/kg/svc/mox-kg-fusion-svc"),
    ("sdk/mox-kg-sdk", "domains/kg/sdk/mox-kg-sdk"),

    # === AI 智能域 ===
    ("core/ai/mox-ai-core", "domains/ai/core/mox-ai-core"),
    ("core/ai/mox-ai-intent-core", "domains/ai/core/mox-ai-intent-core"),
    ("services/ai/mox-ai-flow-svc", "domains/ai/svc/mox-ai-flow-svc"),
    ("services/ai/mox-ai-expert-svc", "domains/ai/svc/mox-ai-expert-svc"),
    ("services/ai/mox-ai-agent-svc", "domains/ai/svc/mox-ai-agent-svc"),

    # === Flow 流程自动化域 ===
    ("core/flow/mox-flow-operator-core", "domains/flow/core/mox-flow-operator-core"),
    ("core/flow/mox-flow-optimizer-core", "domains/flow/core/mox-flow-optimizer-core"),
    ("services/flow/mox-flow-operator-wasm-svc", "domains/flow/svc/mox-flow-operator-wasm-svc"),
    ("services/flow/mox-flow-primiflow-svc", "domains/flow/svc/mox-flow-primiflow-svc"),
    ("services/flow/mox-flow-fusion-svc", "domains/flow/svc/mox-flow-fusion-svc"),
    ("services/flow/mox-flow-bridge-svc", "domains/flow/svc/mox-flow-bridge-svc"),

    # === Data 数据治理域 ===
    ("core/data/mox-data-formula-core", "domains/data/core/mox-data-formula-core"),
    ("core/data/mox-data-norm-core", "domains/data/core/mox-data-norm-core"),
    ("core/data/mox-data-standards-core", "domains/data/core/mox-data-standards-core"),
    ("services/data/mox-data-plane-svc", "domains/data/svc/mox-data-plane-svc"),
    ("services/data/mox-data-etl-svc", "domains/data/svc/mox-data-etl-svc"),
    ("services/data/mox-data-compliance-svc", "domains/data/svc/mox-data-compliance-svc"),
    ("services/data/mox-data-catalog-svc", "domains/data/svc/mox-data-catalog-svc"),
    ("sdk/mox-data-formula-native", "domains/data/sdk/mox-data-formula-native"),
    ("sdk/mox-data-norm-intent-native", "domains/data/sdk/mox-data-norm-intent-native"),

    # === Cloud 云存储域 ===
    ("services/cloud/mox-cloud-master-svc", "domains/cloud/svc/mox-cloud-master-svc"),
    ("services/cloud/mox-cloud-volume-svc", "domains/cloud/svc/mox-cloud-volume-svc"),
    ("services/cloud/mox-cloud-s3-svc", "domains/cloud/svc/mox-cloud-s3-svc"),
    ("services/cloud/mox-cloud-filer-svc", "domains/cloud/svc/mox-cloud-filer-svc"),
    ("sdk/mox-cloud-sdk", "domains/cloud/sdk/mox-cloud-sdk"),

    # === Voice 语音域 ===
    ("core/voice/mox-voice-dsp-core", "domains/voice/core/mox-voice-dsp-core"),
    ("services/voice/mox-voice-core-svc", "domains/voice/svc/mox-voice-core-svc"),
    ("services/voice/mox-voice-asr-svc", "domains/voice/svc/mox-voice-asr-svc"),
    ("services/voice/mox-voice-intent-svc", "domains/voice/svc/mox-voice-intent-svc"),
    ("services/voice/mox-voice-operator-svc", "domains/voice/svc/mox-voice-operator-svc"),
    ("services/voice/mox-voice-desktop-app", "domains/voice/svc/mox-voice-desktop-app"),
    ("sdk/mox-voice-dsp-py", "domains/voice/sdk/mox-voice-dsp-py"),

    # === Market 市场域 ===
    ("services/market/mox-market-template-svc", "domains/market/svc/mox-market-template-svc"),

    # === Platform 平台基础域 ===
    ("core/platform/mox-platform-system-core", "domains/platform/core/mox-platform-system-core"),
    ("services/platform/mox-platform-orchestrator-svc", "domains/platform/svc/mox-platform-orchestrator-svc"),
    ("sdk/mox-platform-test-harness", "domains/platform/sdk/mox-platform-test-harness"),
]

# 保留在顶层的跨域共享层(不迁移)
KEPT_AT_TOP = [
    "foundation/mox-platform-foundation",
    "foundation/mox-cloud-foundation",
    "framework/mox-framework",
    "gateway/mox-platform-gateway-svc",
]


def main():
    print("=" * 70)
    print("infotopograph 目录结构二次迁移：按层组织 → 按域组织")
    print("=" * 70)
    print(f"\n📦 待迁移 crate 数: {len(MIGRATIONS)}")
    print(f"📌 保留顶层: {len(KEPT_AT_TOP)} (foundation/framework/gateway)")

    moved = 0
    skipped = 0
    errors = []

    for old_rel, new_rel in MIGRATIONS:
        old_path = P / old_rel
        new_path = P / new_rel
        if not old_path.exists():
            skipped += 1
            print(f"  ⏭️  跳过(不存在): {old_rel}")
            continue
        if new_path.exists():
            skipped += 1
            print(f"  ⏭️  跳过(目标已存在): {new_rel}")
            continue
        try:
            new_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(old_path), str(new_path))
            moved += 1
            print(f"  ✅ {old_rel} → {new_rel}")
        except Exception as e:
            errors.append((old_rel, str(e)))
            print(f"  ❌ {old_rel}: {e}")

    # 清理空的旧层目录
    print("\n🧹 清理空的旧层目录...")
    for old_layer in ["core", "services", "sdk", "api", "svcapi"]:
        old_dir = P / old_layer
        if old_dir.exists():
            # 检查是否还有内容
            remaining = list(old_dir.rglob("Cargo.toml"))
            if not remaining:
                shutil.rmtree(old_dir)
                print(f"  🗑️  已删除空目录: {old_layer}/")
            else:
                print(f"  ⚠️  {old_layer}/ 仍有 {len(remaining)} 个crate，保留")

    print(f"\n{'=' * 70}")
    print(f"📊 迁移完成: 成功={moved}, 跳过={skipped}, 错误={len(errors)}")
    if errors:
        print("❌ 错误列表:")
        for old, err in errors:
            print(f"   - {old}: {err}")
    print("=" * 70)

    # 输出最终目录结构
    print("\n📁 最终目录结构（按域组织，域内分层）:")
    print("platform/")
    print("├── foundation/          # 跨域共享基础层")
    print("├── framework/           # 企业级横切框架")
    print("├── gateway/             # 统一接入网关")
    print("└── domains/             # 业务域（模块优先）")
    for domain in sorted(["kg", "ai", "flow", "data", "cloud", "voice", "market", "platform"]):
        dpath = P / "domains" / domain
        if dpath.exists():
            layers = [d.name for d in dpath.iterdir() if d.is_dir() and any(d.iterdir())]
            crates = list(dpath.rglob("Cargo.toml"))
            print(f"    ├── {domain}/  ({len(crates)} crates, 层: {', '.join(sorted(layers))})")


if __name__ == "__main__":
    main()
