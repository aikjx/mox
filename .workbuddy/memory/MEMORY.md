# 项目长期记忆 (infotopograph · 关图/璇玑)

## 工程约定
- **Windows 沙箱跑 cargo 的铁律**：前台 Bash 调用 `cargo build/test` 写 `target/` 会被沙箱拦截（os error 5 Access denied）。必须用 `run_in_background=true` 跑 cargo 测试/构建，后台进程才有文件系统写权限。
- rustc 报 `rmeta::encoder.rs` 内部编译错误（ICE）通常是被中断的并行编译留下的 `target/debug/incremental` 缓存损坏。先 `taskkill /F /IM cargo.exe /IM rustc.exe` 清孤儿进程，再 `rm -rf target/debug/incremental` 重建即可恢复。
- 验证命令：`cargo test --workspace`（2026-08-17：625 passed / 0 failed，15 crate / 89 测试二进制）。
- **OUS 前后端运行约定（2026-08-18 新增）**：后端 `backend/` 为零依赖 Node.js（仅用内置 http/fs/path，因沙箱 `npm install` 不可用），30+ 文件 self-contained。启动：`cd backend && node src/server.js`（默认 3000，静态托管 `frontend/dist` 为系统统一入口）。**重建前端前必须先停后端**（后端持 `frontend/dist` 文件锁，否则 `vite build` 现 EPERM）。流程：停后端→`rm -rf frontend/dist`→`node node_modules/vite/bin/vite.js build`→重启后端。前端 api baseURL=`/api`，开发期令牌 `dev-secret-token`；独立 `npm run dev` 时 Vite 代理 `/api→localhost:3000` 需后端在跑。

## 系统术语
- **璇玑 = 璇玑 (Xuánjī) 系统** = `xuanji-expert` crate：归一化 IR 驱动、双璇玑十四维（业务7+开发7）并行诊断 → 裁决 → flow-ai 求解 → ⛨璇玑验证网关(最高权限) → 治理闸门 → 出码/出图。承载于关图 GR-STD（`REQ→FUN→BIZ→ALG→TSK→COD` 六维绑定）。
- 交付物统一放 `docs/`，编号体系：AA-STD-V1.0(流程图) / ISD-AA-V1.0(验收报告) / 机读 `璇玑-全维流水线.mmd`。

## 已知架构缺口（已修复，留痕）
- `xuanji-system/tests/integration.rs` 的 `temp_db()` 曾仅用 pid 致并行串扰（已改 AtomicU64+pid 唯一目录）。
- `primiflow-fusion/src/unified.rs` 的 `add_node` 用 HashMap 覆盖同 id 节点致 G3 重复 id 检测失效（已加 `node_dups` 字段修复）。
