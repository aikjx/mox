#!/usr/bin/env python3
"""
infotopograph 架构归一化迁移脚本
一键执行：48 crate 重命名 + 目录重组 + 依赖更新 + Cargo.toml 同步

用法:
  python tools/migrate_architecture.py --dry-run    # 预览（不实际修改）
  python tools/migrate_architecture.py --execute     # 执行迁移
  python tools/migrate_architecture.py --verify      # 验证迁移结果
"""

import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Tuple

ROOT = Path(r"D:\a10\aikjx\gitcode\infotopograph")
CARGO_TOML = ROOT / "Cargo.toml"

# 48 crate 归一化映射表
# (当前路径, 当前crate名, 目标路径, 目标crate名, 目标层, 目标域)
MIGRATION_MAP = [
    # === Foundation 层 ===
    ("platform/services/mox-common-meta", "mox-common-meta", "platform/foundation/mox-platform-foundation", "mox-platform-foundation", "foundation", "platform"),
    ("platform/services/mox-domain-abstractions", "mox-domain-abstractions", "platform/foundation/mox-cloud-foundation", "mox-cloud-foundation", "foundation", "cloud"),

    # === Core 层 ===
    ("platform/crates/mox-formulas-core", "mox-formulas-core", "platform/core/data/mox-data-formula-core", "mox-data-formula-core", "core", "data"),
    ("platform/crates/mox-norm-core", "mox-norm-core", "platform/core/data/mox-data-norm-core", "mox-data-norm-core", "core", "data"),
    ("platform/crates/mox-intent-core", "mox-intent-core", "platform/core/ai/mox-ai-intent-core", "mox-ai-intent-core", "core", "ai"),
    ("platform/crates/xiaobai-dsp", "xiaobai-dsp", "platform/core/voice/mox-voice-dsp-core", "mox-voice-dsp-core", "core", "voice"),
    ("platform/services/operator-core", "operator-core", "platform/core/flow/mox-flow-operator-core", "mox-flow-operator-core", "core", "flow"),
    ("platform/services/optimizer", "optimizer", "platform/core/flow/mox-flow-optimizer-core", "mox-flow-optimizer-core", "core", "flow"),
    ("platform/services/graph-algorithms", "graph-algorithms", "platform/core/kg/mox-kg-algo-core", "mox-kg-algo-core", "core", "kg"),
    ("platform/services/mox-graph-meta", "mox-graph-meta", "platform/core/kg/mox-kg-meta-core", "mox-kg-meta-core", "core", "kg"),
    ("platform/services/mox-standards", "mox-standards", "platform/core/data/mox-data-standards-core", "mox-data-standards-core", "core", "data"),
    ("platform/services/mox-system", "mox-system", "platform/core/platform/mox-platform-system-core", "mox-platform-system-core", "core", "platform"),
    ("platform/services/mox-ai-core", "mox-ai-core", "platform/core/ai/mox-ai-core", "mox-ai-core", "core", "ai"),

    # === Service 层 (kg域) ===
    ("platform/services/mox-graph-storage", "mox-graph-storage", "platform/services/kg/mox-kg-storage-svc", "mox-kg-storage-svc", "service", "kg"),
    ("platform/services/mox-graph-service", "mox-graph-service", "platform/services/kg/mox-kg-service-svc", "mox-kg-service-svc", "service", "kg"),
    ("platform/services/mox-graph-streams", "mox-graph-streams", "platform/services/kg/mox-kg-streams-svc", "mox-kg-streams-svc", "service", "kg"),
    ("platform/services/mox-graph-spark", "mox-graph-spark", "platform/services/kg/mox-kg-spark-svc", "mox-kg-spark-svc", "service", "kg"),
    ("platform/services/kg-hub", "kg-hub", "platform/services/kg/mox-kg-hub-svc", "mox-kg-hub-svc", "service", "kg"),
    ("platform/services/mox-fusion", "mox-fusion", "platform/services/kg/mox-kg-fusion-svc", "mox-kg-fusion-svc", "service", "kg"),

    # === Service 层 (ai域) ===
    ("platform/services/flow-ai", "flow-ai", "platform/services/ai/mox-ai-flow-svc", "mox-ai-flow-svc", "service", "ai"),
    ("platform/services/mox-expert", "mox-expert", "platform/services/ai/mox-ai-expert-svc", "mox-ai-expert-svc", "service", "ai"),
    ("platform/services/ai-agent", "ai-agent", "platform/services/ai/mox-ai-agent-svc", "mox-ai-agent-svc", "service", "ai"),

    # === Service 层 (flow域) ===
    ("platform/services/operator-wasm", "operator-wasm", "platform/services/flow/mox-flow-operator-wasm-svc", "mox-flow-operator-wasm-svc", "service", "flow"),
    ("platform/services/primiflow-core", "primiflow-core", "platform/services/flow/mox-flow-primiflow-svc", "mox-flow-primiflow-svc", "service", "flow"),
    ("platform/services/primiflow-fusion", "primiflow-fusion", "platform/services/flow/mox-flow-fusion-svc", "mox-flow-fusion-svc", "service", "flow"),
    ("platform/services/hermes-flow-bridge", "hermes-flow-bridge", "platform/services/flow/mox-flow-bridge-svc", "mox-flow-bridge-svc", "service", "flow"),

    # === Service 层 (data域) ===
    ("platform/services/mox-data-plane", "mox-data-plane", "platform/services/data/mox-data-plane-svc", "mox-data-plane-svc", "service", "data"),
    ("platform/services/mox-etl-wasm", "mox-etl-wasm", "platform/services/data/mox-data-etl-svc", "mox-data-etl-svc", "service", "data"),
    ("platform/services/mox-compliance", "mox-compliance", "platform/services/data/mox-data-compliance-svc", "mox-data-compliance-svc", "service", "data"),
    ("platform/services/business-catalog", "business-catalog", "platform/services/data/mox-data-catalog-svc", "mox-data-catalog-svc", "service", "data"),

    # === Service 层 (cloud域) ===
    ("platform/services/mox-cloud-drive-master", "mox-cloud-drive-master", "platform/services/cloud/mox-cloud-master-svc", "mox-cloud-master-svc", "service", "cloud"),
    ("platform/services/mox-cloud-drive-volume", "mox-cloud-drive-volume", "platform/services/cloud/mox-cloud-volume-svc", "mox-cloud-volume-svc", "service", "cloud"),
    ("platform/services/mox-cloud-drive-s3", "mox-cloud-drive-s3", "platform/services/cloud/mox-cloud-s3-svc", "mox-cloud-s3-svc", "service", "cloud"),
    ("platform/services/mox-cloud-drive-filer", "mox-cloud-drive-filer", "platform/services/cloud/mox-cloud-filer-svc", "mox-cloud-filer-svc", "service", "cloud"),

    # === Service 层 (voice域) ===
    ("platform/crates/xiaobai-core", "xiaobai-core", "platform/services/voice/mox-voice-core-svc", "mox-voice-core-svc", "service", "voice"),
    ("platform/crates/xiaobai-asr", "xiaobai-asr", "platform/services/voice/mox-voice-asr-svc", "mox-voice-asr-svc", "service", "voice"),
    ("platform/crates/xiaobai-intent", "xiaobai-intent", "platform/services/voice/mox-voice-intent-svc", "mox-voice-intent-svc", "service", "voice"),
    ("platform/crates/xiaobai-operators", "xiaobai-operators", "platform/services/voice/mox-voice-operator-svc", "mox-voice-operator-svc", "service", "voice"),

    # === Service 层 (market域) ===
    ("platform/services/template-market", "template-market", "platform/services/market/mox-market-template-svc", "mox-market-template-svc", "service", "market"),

    # === Application 层 ===
    ("platform/gateway/runtime", "runtime", "platform/services/platform/mox-platform-orchestrator-svc", "mox-platform-orchestrator-svc", "application", "platform"),
    ("platform/services/mox-server", "mox-server", "platform/gateway/mox-platform-gateway-svc", "mox-platform-gateway-svc", "application", "platform"),
    ("platform/services/mox-t21-harness", "mox-t21-harness", "platform/sdk/mox-platform-test-harness", "mox-platform-test-harness", "sdk", "platform"),
    ("platform/crates/xiaobai-desktop", "xiaobai-desktop", "platform/services/voice/mox-voice-desktop-app", "mox-voice-desktop-app", "application", "voice"),

    # === SDK 层 ===
    ("platform/sdk/rust/mox-sdk-cloud", "mox-sdk-cloud", "platform/sdk/mox-cloud-sdk", "mox-cloud-sdk", "sdk", "cloud"),
    ("platform/sdk/rust/mox-sdk-graph", "mox-sdk-graph", "platform/sdk/mox-kg-sdk", "mox-kg-sdk", "sdk", "kg"),
    ("platform/crates/bindings/mox-formulas-native", "mox-formulas-native", "platform/sdk/mox-data-formula-native", "mox-data-formula-native", "sdk", "data"),
    ("platform/crates/bindings/mox-norm-intent-native", "mox-norm-intent-native", "platform/sdk/mox-data-norm-intent-native", "mox-data-norm-intent-native", "sdk", "data"),
    ("platform/crates/bindings/xiaobai-dsp-py", "xiaobai-dsp-py", "platform/sdk/mox-voice-dsp-py", "mox-voice-dsp-py", "sdk", "voice"),
]

# 构建旧名→新名映射
OLD_TO_NEW = {old_name: new_name for _, old_name, _, new_name, _, _ in MIGRATION_MAP}
OLD_TO_NEW_PATH = {old_path: new_path for old_path, _, new_path, _, _, _ in MIGRATION_MAP}


def read_cargo_toml() -> str:
    return CARGO_TOML.read_text(encoding="utf-8")


def write_cargo_toml(content: str):
    CARGO_TOML.write_text(content, encoding="utf-8")


def update_workspace_members(content: str) -> str:
    """更新 workspace members 列表为新路径"""
    new_members = []
    for _, _, new_path, _, _, _ in MIGRATION_MAP:
        new_members.append(f'    "{new_path}",')
    new_members.append('    "platform/framework",')

    # 替换 members 块
    pattern = r'members = \[(.*?)\]'
    replacement = "members = [\n" + "\n".join(new_members) + "\n]"
    content = re.sub(pattern, replacement, content, flags=re.DOTALL)
    return content


def update_default_members(content: str) -> str:
    """更新 default-members 为新路径"""
    new_defaults = [
        '    "platform/core/data/mox-data-formula-core",',
        '    "platform/core/data/mox-data-norm-core",',
        '    "platform/core/ai/mox-ai-intent-core",',
        '    "platform/core/voice/mox-voice-dsp-core",',
        '    "platform/services/voice/mox-voice-core-svc",',
        '    "platform/services/voice/mox-voice-operator-svc",',
        '    "platform/services/voice/mox-voice-intent-svc",',
        '    "platform/services/voice/mox-voice-asr-svc",',
    ]
    pattern = r'default-members = \[(.*?)\]'
    replacement = "default-members = [\n" + "\n".join(new_defaults) + "\n]"
    content = re.sub(pattern, replacement, content, flags=re.DOTALL)
    return content


def move_crate(old_path: str, new_path: str, dry_run: bool = True) -> Tuple[bool, str]:
    """移动 crate 目录"""
    old_abs = ROOT / old_path
    new_abs = ROOT / new_path
    if not old_abs.exists():
        return False, f"源目录不存在: {old_path}"
    if new_abs.exists():
        return False, f"目标目录已存在: {new_path}"
    if dry_run:
        return True, f"[DRY-RUN] 移动: {old_path} → {new_path}"
    new_abs.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(old_abs), str(new_abs))
    return True, f"已移动: {old_path} → {new_path}"


def rename_crate_in_toml(crate_path: str, old_name: str, new_name: str, dry_run: bool = True) -> Tuple[bool, str]:
    """重命名 crate 目录下的 Cargo.toml 中的 name 字段"""
    toml_path = ROOT / crate_path / "Cargo.toml"
    if not toml_path.exists():
        return False, f"Cargo.toml 不存在: {toml_path}"
    content = toml_path.read_text(encoding="utf-8")
    new_content = re.sub(r'^name\s*=\s*"[^"]+"', f'name = "{new_name}"', content, count=1, flags=re.MULTILINE)
    if dry_run:
        return True, f"[DRY-RUN] 重命名: {old_name} → {new_name} ({crate_path}/Cargo.toml)"
    toml_path.write_text(new_content, encoding="utf-8")
    return True, f"已重命名: {old_name} → {new_name}"


def update_dependencies_in_all_crates(dry_run: bool = True) -> List[str]:
    """更新所有 crate 中的依赖引用（旧crate名→新crate名）"""
    results = []
    for _, _, new_path, _, _, _ in MIGRATION_MAP:
        toml_path = ROOT / new_path / "Cargo.toml"
        if not toml_path.exists():
            continue
        content = toml_path.read_text(encoding="utf-8")
        original = content
        for old_name, new_name in OLD_TO_NEW.items():
            # 替换依赖声明中的 crate 名
            content = re.sub(
                rf'^{old_name}\s*=\s*\{{',
                f'{new_name} = {{',
                content,
                flags=re.MULTILINE,
            )
            content = re.sub(
                rf'^{old_name}\s*=\s*"',
                f'{new_name} = "',
                content,
                flags=re.MULTILINE,
            )
        if content != original:
            if dry_run:
                results.append(f"[DRY-RUN] 更新依赖: {new_path}/Cargo.toml")
            else:
                toml_path.write_text(content, encoding="utf-8")
                results.append(f"已更新依赖: {new_path}/Cargo.toml")
    return results


def update_src_imports(dry_run: bool = True) -> List[str]:
    """更新所有 src/*.rs 文件中的 use 引用（旧crate名→新crate名，Rust crate名用下划线）"""
    results = []
    for _, _, new_path, old_name, new_name, _ in MIGRATION_MAP:
        src_dir = ROOT / new_path / "src"
        if not src_dir.exists():
            continue
        old_snake = old_name.replace("-", "_")
        new_snake = new_name.replace("-", "_")
        for rs_file in src_dir.rglob("*.rs"):
            content = rs_file.read_text(encoding="utf-8")
            original = content
            # 替换 use 引用
            content = re.sub(
                rf'\buse\s+{old_snake}\b',
                f'use {new_snake}',
                content,
            )
            # 替换 extern crate
            content = re.sub(
                rf'\bextern\s+crate\s+{old_snake}\b',
                f'extern crate {new_snake}',
                content,
            )
            if content != original:
                if dry_run:
                    results.append(f"[DRY-RUN] 更新引用: {rs_file.relative_to(ROOT)}")
                else:
                    rs_file.write_text(content, encoding="utf-8")
                    results.append(f"已更新引用: {rs_file.relative_to(ROOT)}")
    return results


def verify_migration() -> Dict:
    """验证迁移结果"""
    result = {
        "total_crates": 0,
        "moved": 0,
        "renamed": 0,
        "missing": [],
        "cargo_check": "未执行",
    }
    for _, _, new_path, new_name, _, _ in MIGRATION_MAP:
        result["total_crates"] += 1
        toml_path = ROOT / new_path / "Cargo.toml"
        if toml_path.exists():
            result["moved"] += 1
            content = toml_path.read_text(encoding="utf-8")
            if f'name = "{new_name}"' in content:
                result["renamed"] += 1
            else:
                result["missing"].append(f"名称未更新: {new_path} (期望 {new_name})")
        else:
            result["missing"].append(f"目录缺失: {new_path}")
    return result


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    mode = sys.argv[1]
    dry_run = mode == "--dry-run"
    execute = mode == "--execute"
    verify = mode == "--verify"

    if dry_run or execute:
        print("=" * 70)
        print(f"infotopograph 架构归一化迁移 ({'预览模式' if dry_run else '执行模式'})")
        print("=" * 70)
        print(f"\n📦 待迁移 crate 数: {len(MIGRATION_MAP)}")

        # 1. 移动目录
        print("\n📂 步骤1: 移动 crate 目录")
        for old_path, old_name, new_path, new_name, layer, domain in MIGRATION_MAP:
            ok, msg = move_crate(old_path, new_path, dry_run=dry_run)
            print(f"  {'✅' if ok else '❌'} {msg}")

        # 2. 重命名 crate
        print("\n✏️  步骤2: 重命名 crate (Cargo.toml name字段)")
        for _, old_name, new_path, new_name, _, _ in MIGRATION_MAP:
            ok, msg = rename_crate_in_toml(new_path, old_name, new_name, dry_run=dry_run)
            print(f"  {'✅' if ok else '❌'} {msg}")

        # 3. 更新 workspace Cargo.toml
        print("\n📝 步骤3: 更新 workspace Cargo.toml (members + default-members)")
        if not dry_run:
            content = read_cargo_toml()
            content = update_workspace_members(content)
            content = update_default_members(content)
            write_cargo_toml(content)
            print("  ✅ workspace Cargo.toml 已更新")
        else:
            print("  [DRY-RUN] workspace Cargo.toml 将更新")

        # 4. 更新所有 crate 的依赖引用
        print("\n🔗 步骤4: 更新所有 crate 的依赖引用")
        dep_results = update_dependencies_in_all_crates(dry_run=dry_run)
        for r in dep_results:
            print(f"  ✅ {r}")

        # 5. 更新 src 中的 use 引用
        print("\n📚 步骤5: 更新 src/*.rs 中的 use 引用")
        import_results = update_src_imports(dry_run=dry_run)
        for r in import_results:
            print(f"  ✅ {r}")

        print("\n" + "=" * 70)
        if dry_run:
            print("✅ 预览完成。使用 --execute 执行实际迁移")
        else:
            print("✅ 迁移执行完成。使用 --verify 验证结果")
        print("=" * 70)

    elif verify:
        print("=" * 70)
        print("infotopograph 架构迁移验证")
        print("=" * 70)
        result = verify_migration()
        print(f"\n📊 总 crate 数: {result['total_crates']}")
        print(f"✅ 已移动: {result['moved']}")
        print(f"✅ 已重命名: {result['renamed']}")
        if result['missing']:
            print(f"\n❌ 问题 ({len(result['missing'])}):")
            for m in result['missing']:
                print(f"   - {m}")
        else:
            print("\n✅ 所有 crate 迁移验证通过!")
        print("=" * 70)


if __name__ == "__main__":
    main()
