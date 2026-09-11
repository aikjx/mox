#!/usr/bin/env python3
"""
批量修复：将所有内部crate的直接路径依赖改为 workspace = true
因为所有48个内部crate已在根Cargo.toml的workspace.dependencies中注册
"""

import re
from pathlib import Path

ROOT = Path(r"D:\a10\aikjx\gitcode\infotopograph")

# 所有内部crate名（归一化后）
INTERNAL_CRATES = {
    "mox-platform-foundation", "mox-cloud-foundation",
    "mox-data-formula-core", "mox-data-norm-core", "mox-data-standards-core",
    "mox-ai-intent-core", "mox-ai-core", "mox-voice-dsp-core",
    "mox-flow-operator-core", "mox-flow-optimizer-core",
    "mox-kg-algo-core", "mox-kg-meta-core", "mox-platform-system-core",
    "mox-kg-storage-svc", "mox-kg-service-svc", "mox-kg-streams-svc",
    "mox-kg-spark-svc", "mox-kg-hub-svc", "mox-kg-fusion-svc",
    "mox-ai-flow-svc", "mox-ai-expert-svc", "mox-ai-agent-svc",
    "mox-flow-operator-wasm-svc", "mox-flow-primiflow-svc",
    "mox-flow-fusion-svc", "mox-flow-bridge-svc",
    "mox-data-plane-svc", "mox-data-etl-svc", "mox-data-compliance-svc",
    "mox-data-catalog-svc",
    "mox-cloud-master-svc", "mox-cloud-volume-svc", "mox-cloud-s3-svc",
    "mox-cloud-filer-svc",
    "mox-voice-core-svc", "mox-voice-asr-svc", "mox-voice-intent-svc",
    "mox-voice-operator-svc",
    "mox-market-template-svc",
    "mox-platform-orchestrator-svc", "mox-platform-gateway-svc",
    "mox-voice-desktop-app",
    "mox-cloud-sdk", "mox-kg-sdk", "mox-platform-test-harness",
    "mox-data-formula-native", "mox-data-norm-intent-native",
    "mox-voice-dsp-py", "mox-framework",
    # 旧名（可能还存在于某些Cargo.toml中）
    "mox-common-meta", "mox-domain-abstractions",
    "mox-formulas-core", "mox-norm-core", "mox-intent-core",
    "xiaobai-dsp", "xiaobai-core", "xiaobai-operators", "xiaobai-intent", "xiaobai-asr",
    "operator-core", "operator-wasm", "optimizer",
    "graph-algorithms", "mox-graph-meta", "mox-standards", "mox-system", "mox-ai-core",
    "mox-graph-storage", "mox-graph-service", "mox-graph-streams", "mox-graph-spark",
    "kg-hub", "mox-fusion",
    "flow-ai", "mox-expert", "ai-agent",
    "primiflow-core", "primiflow-fusion", "hermes-flow-bridge",
    "mox-data-plane", "mox-etl-wasm", "mox-compliance", "business-catalog",
    "mox-cloud-drive-master", "mox-cloud-drive-volume", "mox-cloud-drive-s3", "mox-cloud-drive-filer",
    "template-market", "runtime", "mox-server", "mox-t21-harness", "xiaobai-desktop",
    "mox-sdk-cloud", "mox-sdk-graph", "mox-formulas-native", "mox-norm-intent-native", "xiaobai-dsp-py",
}

# 旧名→新名映射
OLD_TO_NEW = {
    "mox-common-meta": "mox-platform-foundation",
    "mox-domain-abstractions": "mox-cloud-foundation",
    "mox-formulas-core": "mox-data-formula-core",
    "mox-norm-core": "mox-data-norm-core",
    "mox-intent-core": "mox-ai-intent-core",
    "xiaobai-dsp": "mox-voice-dsp-core",
    "xiaobai-core": "mox-voice-core-svc",
    "xiaobai-operators": "mox-voice-operator-svc",
    "xiaobai-intent": "mox-voice-intent-svc",
    "xiaobai-asr": "mox-voice-asr-svc",
    "operator-core": "mox-flow-operator-core",
    "operator-wasm": "mox-flow-operator-wasm-svc",
    "optimizer": "mox-flow-optimizer-core",
    "graph-algorithms": "mox-kg-algo-core",
    "mox-graph-meta": "mox-kg-meta-core",
    "mox-standards": "mox-data-standards-core",
    "mox-system": "mox-platform-system-core",
    "mox-graph-storage": "mox-kg-storage-svc",
    "mox-graph-service": "mox-kg-service-svc",
    "mox-graph-streams": "mox-kg-streams-svc",
    "mox-graph-spark": "mox-kg-spark-svc",
    "kg-hub": "mox-kg-hub-svc",
    "mox-fusion": "mox-kg-fusion-svc",
    "flow-ai": "mox-ai-flow-svc",
    "mox-expert": "mox-ai-expert-svc",
    "ai-agent": "mox-ai-agent-svc",
    "primiflow-core": "mox-flow-primiflow-svc",
    "primiflow-fusion": "mox-flow-fusion-svc",
    "hermes-flow-bridge": "mox-flow-bridge-svc",
    "mox-data-plane": "mox-data-plane-svc",
    "mox-etl-wasm": "mox-data-etl-svc",
    "mox-compliance": "mox-data-compliance-svc",
    "business-catalog": "mox-data-catalog-svc",
    "mox-cloud-drive-master": "mox-cloud-master-svc",
    "mox-cloud-drive-volume": "mox-cloud-volume-svc",
    "mox-cloud-drive-s3": "mox-cloud-s3-svc",
    "mox-cloud-drive-filer": "mox-cloud-filer-svc",
    "template-market": "mox-market-template-svc",
    "runtime": "mox-platform-orchestrator-svc",
    "mox-server": "mox-platform-gateway-svc",
    "mox-t21-harness": "mox-platform-test-harness",
    "xiaobai-desktop": "mox-voice-desktop-app",
    "mox-sdk-cloud": "mox-cloud-sdk",
    "mox-sdk-graph": "mox-kg-sdk",
    "mox-formulas-native": "mox-data-formula-native",
    "mox-norm-intent-native": "mox-data-norm-intent-native",
    "xiaobai-dsp-py": "mox-voice-dsp-py",
}


def fix_cargo_toml(toml_path: Path) -> int:
    """修复单个Cargo.toml中的内部依赖，返回修改数量"""
    content = toml_path.read_text(encoding="utf-8")
    original = content
    fixes = 0

    # 1. 替换旧名为新名（在依赖声明中）
    for old_name, new_name in OLD_TO_NEW.items():
        # 匹配: old_name = { ... } 或 old_name = "version"
        # 只替换依赖名，不替换注释
        pattern = rf'(?m)^(\s*){re.escape(old_name)}(\s*=\s*)'
        if re.search(pattern, content):
            content = re.sub(pattern, rf'\1{new_name}\2', content)
            fixes += 1

    # 2. 将内部crate的直接路径依赖改为 workspace = true
    for crate_name in INTERNAL_CRATES:
        # 匹配: crate_name = { path = "...", ... }  → crate_name = { workspace = true }
        # 保留 features 等其他属性
        pattern = rf'(?m)^(\s*){re.escape(crate_name)}(\s*=\s*)\{{[^}}]*path\s*=\s*"[^"]*"[^}}]*\}}'
        match = re.search(pattern, content)
        if match:
            full_match = match.group(0)
            # 提取 features
            features_match = re.search(r'features\s*=\s*(\[[^\]]*\])', full_match)
            features = features_match.group(1) if features_match else None
            # 提取 optional
            optional_match = re.search(r'optional\s*=\s*(true|false)', full_match)
            optional = optional_match.group(1) if optional_match else None

            # 构建新的依赖声明
            parts = ["workspace = true"]
            if features:
                parts.append(f"features = {features}")
            if optional and optional == "true":
                parts.append("optional = true")
            new_dep = f"{match.group(1)}{crate_name}{match.group(2)}{{ {', '.join(parts)} }}"
            content = content[:match.start()] + new_dep + content[match.end():]
            fixes += 1

        # 匹配: crate_name = { workspace = true, path = "..." } → 移除 path
        pattern2 = rf'(?m)^(\s*){re.escape(crate_name)}(\s*=\s*\{{[^}}]*?)path\s*=\s*"[^"]*",?\s*([^}}]*\}})'
        if re.search(pattern2, content):
            content = re.sub(pattern2, rf'\1{crate_name}\2\3', content)
            fixes += 1

    if content != original:
        toml_path.write_text(content, encoding="utf-8")
    return fixes


def main():
    print("=" * 70)
    print("批量修复：内部crate直接路径依赖 → workspace = true")
    print("=" * 70)

    total_fixes = 0
    files_fixed = 0

    # 遍历所有Cargo.toml（排除根Cargo.toml）
    for toml_path in ROOT.rglob("Cargo.toml"):
        if toml_path == ROOT / "Cargo.toml":
            continue
        # 跳过 target 目录
        if "target" in toml_path.parts:
            continue
        fixes = fix_cargo_toml(toml_path)
        if fixes > 0:
            rel = toml_path.relative_to(ROOT)
            print(f"  ✅ {rel}: {fixes} 处修复")
            total_fixes += fixes
            files_fixed += 1

    print(f"\n📊 总计: {files_fixed} 个文件, {total_fixes} 处修复")
    print("=" * 70)


if __name__ == "__main__":
    main()
