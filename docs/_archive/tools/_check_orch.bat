@echo off
set CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\14.29.30133\bin\Hostx64\x64\link.exe
set LIB=C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\14.29.30133\lib\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.19041.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.19041.0\ucrt\x64
set INCLUDE=C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\14.29.30133\include;C:\Program Files (x86)\Windows Kits\10\Include\10.0.19041.0\ucrt;C:\Program Files (x86)\Windows Kits\10\Include\10.0.19041.0\um;C:\Program Files (x86)\Windows Kits\10\Include\10.0.19041.0\shared
set PATH=C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\14.29.30133\bin\Hostx64\x64;C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64;%PATH%
cd /d D:\a10\aikjx\gitcode\infotopograph
cargo check -p mox-platform-orchestrator-svc --all-targets > %TEMP%\orch_check.log 2>&1
echo EXIT=%ERRORLEVEL%