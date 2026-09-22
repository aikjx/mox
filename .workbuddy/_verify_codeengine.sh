#!/bin/bash
# verify codeengine stack: unit tests + alliance config + clippy + port gate
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64:/c/Program Files (x86)/Windows Kits/10/bin/10.0.19041.0/x64:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64/link.exe"
export LIB="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/lib/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/um/x64;C:/Program Files (x86)/Windows Kits/10/Lib/10.0.19041.0/ucrt/x64"
export INCLUDE="C:/Program Files (x86)/Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/include;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/ucrt;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/um;C:/Program Files (x86)/Windows Kits/10/Include/10.0.19041.0/shared"
export CARGO_TARGET_DIR="D:/cargo-target"
export CARGO_INCREMENTAL=0
cd "D:/a10/aikjx/gitcode/infotopograph"
echo "=== test codeengine core+svc ==="
cargo test -p mox-codeengine-core -p mox-codeengine-svc 2>&1 | tail -25
echo "=== check alliance config ==="
cargo check -p mox-alliance-config-core --message-format short 2>&1 | tail -8
echo "=== clippy new crates ==="
cargo clippy -p mox-codeengine-core -p mox-codeengine-svc --all-targets --message-format short 2>&1 | tail -15
echo "=== port gate ==="
python scripts/verify-ports.py 2>&1 | tail -8
