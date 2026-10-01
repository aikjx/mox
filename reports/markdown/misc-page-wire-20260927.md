# misc 族分页 wire 读数（2026-09-27）— 供 FRONTEND-MODULE-GOVERNANCE-v1.0.md §5.17 引用

本文件由 `%TEMP%/mox-paged-mut/print-readings.sh` 现场打印生成（命令原样列出，输出未改写）。
网关＝:3080（带客户端 JWT，令牌从 `D:/a10/aikjx/gitcode/infotopograph/frontend-ui/.env.local` 一侧的登录取得，本文件不含令牌）。

## A. `curl` 探针：`/api/tasks` 与 `/api/projects` 的出参壳与查询串语义

```
$ curl -s -H 'Authorization: Bearer <JWT>' 'http://127.0.0.1:3080/api/tasks?page=1&page_size=3'
HTTP 200
{"code":0,"msg":"ok","data":{"filters":{"keyword":null,"sort_by":"created_at","sort_order":"desc","status":null},"has_next":false,"has_prev":false,"items":[],"page":1,"page_size":3,"total":0,"total_pages":0}}

$ curl -s -H 'Authorization: Bearer <JWT>' 'http://127.0.0.1:3080/api/tasks?page=2&page_size=5&sort_by=title'
HTTP 200
{"code":0,"msg":"ok","data":{"filters":{"keyword":null,"sort_by":"title","sort_order":"desc","status":null},"has_next":false,"has_prev":true,"items":[],"page":2,"page_size":5,"total":0,"total_pages":0}}

$ curl -s -H 'Authorization: Bearer <JWT>' 'http://127.0.0.1:3080/api/tasks?pageSize=5&sortBy=title'
HTTP 200
{"code":0,"msg":"ok","data":{"filters":{"keyword":null,"sort_by":"created_at","sort_order":"desc","status":null},"has_next":false,"has_prev":false,"items":[],"page":1,"page_size":20,"total":0,"total_pages":0}}

$ curl -s -H 'Authorization: Bearer <JWT>' 'http://127.0.0.1:3080/api/tasks?page_size=9999'
HTTP 200
{"code":0,"msg":"ok","data":{"filters":{"keyword":null,"sort_by":"created_at","sort_order":"desc","status":null},"has_next":false,"has_prev":false,"items":[],"page":1,"page_size":100,"total":0,"total_pages":0}}

$ curl -s -H 'Authorization: Bearer <JWT>' 'http://127.0.0.1:3080/api/tasks?sort_by=zzz'
HTTP 200
{"code":0,"msg":"ok","data":{"filters":{"keyword":null,"sort_by":"zzz","sort_order":"desc","status":null},"has_next":false,"has_prev":false,"items":[],"page":1,"page_size":20,"total":0,"total_pages":0}}

$ curl -s -H 'Authorization: Bearer <JWT>' 'http://127.0.0.1:3080/api/tasks/paginated?page=1'
HTTP 404


$ curl -s -H 'Authorization: Bearer <JWT>' 'http://127.0.0.1:3080/api/projects?page_size=3'
HTTP 200
{"code":0,"msg":"ok","data":{"filters":{"keyword":null,"sort_by":"created_at","sort_order":"desc","status":null},"has_next":false,"has_prev":false,"items":[],"page":1,"page_size":3,"total":0,"total_pages":0}}

$ curl -s -H 'Authorization: Bearer <JWT>' 'http://127.0.0.1:3080/api/projects/paginated?page=1&page_size=2'
HTTP 502
{"success":false,"code":"UPSTREAM_UNREACHABLE","error":"上游服务不可达: error sending request for url (http://127.0.0.1:8000/api/projects/paginated?page=1&page_size=2)","target":"http://127.0.0.1:8000"}

```
## B. `npx vite build`（改导出名后的验收口径＝rollup）

```
[2mdist/[22m[36massets/js/vendor-echarts-oZaowbgE.js               [39m[1m[2m1,036.07 kB[22m[1m[22m
[2mdist/[22m[36massets/js/vendor-vexflow-DQ0qkijY.js               [39m[1m[2m1,123.20 kB[22m[1m[22m
[2mdist/[22m[36massets/js/3d-force-graph-CPA8Mdj7.js               [39m[1m[2m1,350.68 kB[22m[1m[22m
[32m✓ built in 37.09s[39m
```
## C. `npx vitest run`（全量用例分母）

```
[2m Test Files [22m [1m[32m67 passed[39m[22m[90m (67)[39m
[2m      Tests [22m [1m[32m955 passed[39m[22m[90m (955)[39m
[2m   Start at [22m 01:49:05
[2m   Duration [22m 38.92s[2m (transform 14.38s, setup 0ms, collect 204.25s, tests 16.87s, environment 218.36s, prepare 53.06s)[22m

```
## D. 四把门禁

以下四条命令的 cwd＝`frontend-ui/`（四把脚本都在 `frontend-ui/scripts/gate/` 下，根 `scripts/gate/` 没有它们）；输出原样抄录。

```
$ python scripts/gate/check-api-binding-kinds.py --check
  豁免 E-fnprop   0 处
verdict=PASS（零容忍：函数型导出被当对象用 0 处）
$ python scripts/gate/check-framework-imports.py --check
测试文件命中（单列，不计账；正则字面量已遮蔽，见 §11.15）：0 个 / 0 处
verdict=PASS (探针：非测试源文件 0 处用而未绑)
$ python scripts/gate/check-ep-feedback-imports.py --check
对照：clean 侧 6/6 通过，flag 侧 通过
verdict=PASS (探针：债务多少不改 rc)
$ python scripts/gate/check-frontend-module.py
--- 前端模块化门禁：ERROR=0 ---
$ python scripts/gate/check-doc-links.py | 本文件所在新增段落是否引入死链
0
```
## E. `paged-list.test.js` 变异电池（10 枚 + 基线 + 逐字节还原）（2026-09-27 第三次重跑，驱动与备份在仓库外 `%TEMP%\mox-paged-mut\`）

```
备份写前图像：5 个文件 -> C:\Users\mo\AppData\Local\Temp\mox-paged-mut\bak
基线 sha：
  paged-list.test.js     5e0bb5a178f8baef3dfa3f904ee22d2e4185a6a23fc19ac3c654e106d6a5af23
  paged-list.js          beed630aa462cb6c41074b815af9b29804658d0302db944c7772b4c1a1bf86ca
  ProjectsView.vue       b730b9754570f7482196e01623ae24216657e59e1aa5b1485bd3d52e1211210c
  TaskView.vue           40c24531eb613964b08cfbb384036d4ffbe17837130f9df9e7ae681182dd084d
  projects.api.js        21171eb9366393b2c972f82721b8d179c942c3dfbe3276075bec55c4611e7808

基线（未变异）：rc=0 PASS
[RED] M1 列表键换成猜的 list -> rc=1 红格=1 :: 
[RED] M2 tasks 排序表混进 projects 独有字段 -> rc=1 红格=2,5 :: 
[RED] M3 上限写错（500 而非 clamp 的 100） -> rc=1 红格=2 :: 
[RED] M4 normPage 又给猜键开兜底 -> rc=1 红格=4 :: 
[RED] M5 查询键退回 camelCase -> rc=1 红格=2,5 :: 
[RED] M6 视图撤掉归一化（还原成猜键链） -> rc=1 红格=8 :: 
[RED] M7 幻影端点复活 -> rc=1 红格=8 :: 
[RED] M8 事实源路径被改错（读不到 misc.rs） -> rc=1 红格=- :: 
[RED] M9 TaskView 绕过归一化就地取 items -> rc=1 红格=8 :: 
[RED] M10 ProjectsView 的 refreshList 撤掉归一化 -> rc=1 红格=8 :: 

还原核验：
  [OK]   paged-list.test.js     5e0bb5a178f8baef3dfa3f904ee22d2e4185a6a23fc19ac3c654e106d6a5af23
  [OK]   paged-list.js          beed630aa462cb6c41074b815af9b29804658d0302db944c7772b4c1a1bf86ca
  [OK]   ProjectsView.vue       b730b9754570f7482196e01623ae24216657e59e1aa5b1485bd3d52e1211210c
  [OK]   TaskView.vue           40c24531eb613964b08cfbb384036d4ffbe17837130f9df9e7ae681182dd084d
  [OK]   projects.api.js        21171eb9366393b2c972f82721b8d179c942c3dfbe3276075bec55c4611e7808
PAGED-LIST MUTANTS: PASS (10 枚 + 基线)

```
