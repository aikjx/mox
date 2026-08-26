#!/usr/bin/env python3
"""更新 workspace Cargo.toml 路径为按域组织"""
import re
from pathlib import Path

ROOT = Path(r"D:\a10\aikjx\gitcode\infotopograph")
cargo = ROOT / "Cargo.toml"
c = cargo.read_text(encoding="utf-8")

PATH_MAP = {
    "platform/core/kg/": "platform/domains/kg/core/",
    "platform/core/ai/": "platform/domains/ai/core/",
    "platform/core/flow/": "platform/domains/flow/core/",
    "platform/core/data/": "platform/domains/data/core/",
    "platform/core/voice/": "platform/domains/voice/core/",
    "platform/core/platform/": "platform/domains/platform/core/",
    "platform/services/kg/": "platform/domains/kg/svc/",
    "platform/services/ai/": "platform/domains/ai/svc/",
    "platform/services/flow/": "platform/domains/flow/svc/",
    "platform/services/data/": "platform/domains/data/svc/",
    "platform/services/cloud/": "platform/domains/cloud/svc/",
    "platform/services/voice/": "platform/domains/voice/svc/",
    "platform/services/market/": "platform/domains/market/svc/",
    "platform/services/platform/": "platform/domains/platform/svc/",
    "platform/sdk/mox-kg-sdk": "platform/domains/kg/sdk/mox-kg-sdk",
    "platform/sdk/mox-cloud-sdk": "platform/domains/cloud/sdk/mox-cloud-sdk",
    "platform/sdk/mox-data-formula-native": "platform/domains/data/sdk/mox-data-formula-native",
    "platform/sdk/mox-data-norm-intent-native": "platform/domains/data/sdk/mox-data-norm-intent-native",
    "platform/sdk/mox-voice-dsp-py": "platform/domains/voice/sdk/mox-voice-dsp-py",
    "platform/sdk/mox-platform-test-harness": "platform/domains/platform/sdk/mox-platform-test-harness",
}

for old, new in PATH_MAP.items():
    c = c.replace(old, new)

cargo.write_text(c, encoding="utf-8")
print("workspace Cargo.toml paths updated")

members = re.findall(r'platform/domains/[^"]+', c)
print(f"  paths with domains/: {len(members)}")
