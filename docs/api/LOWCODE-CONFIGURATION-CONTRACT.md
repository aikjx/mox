# 低代码配置契约设计

> LC-API-01 · V1.0 · 2026-10-01 · 目标契约，不新增虚构HTTP端点。现行接口仍以 `docs/API-REGISTRY.md#1-总览` 为准。

<a id="contract"></a>
## 1. 契约对象与操作

统一配置字段语义引用 `docs/standards/lowcode-dynamic-configuration.md#model`。本目录负责传输契约、版本兼容、并发和错误；不维护页面组件实现、SQL文本或物理表定义。

| 操作 | 输入 | 输出/约束 |
|---|---|---|
| 读取编辑配置 | config_id、资源作用域 | 可见的配置与revision；秘密仅引用 |
| 保存草稿 | 配置+expected_revision | 新revision；并发变更冲突，不最后写入者覆盖 |
| 验证/预览 | 配置版本+目标引擎/作用域 | 字段错误、来源、依赖、风险、预览摘要 |
| 发布 | 精确revision+证据+幂等键 | 不可变release_id/hash及版本闭包 |
| 生效/回滚选择 | release_id+scope+expected_pointer_revision | 新指针版本、审计回执，不直接撤销业务副作用 |
| 运行读取 | 当前受信scope+release/任务引用 | 脱敏、裁剪后的运行模型，不发送内部SQL或密钥 |
| 执行命令 | capability_id+类型化参数+授权上下文 | 幂等接纳/拒绝、任务/结果回执 |
| 导入/导出 | 包manifest+版本/hash | 完整性/兼容性检查；秘密与运行数据分离 |

操作名是逻辑契约；实现前逐项匹配既有端点，确需新端点才登记API-REGISTRY并写OpenAPI。现有模块装配JSON Schema仍为数组型v1；业务配置包信封是另一对象，不给旧schema任意添加字段。

## 2. 稳定ID与映射

module_id按模块注册使用；module_code若是目标母版字段通过明确映射接入，不假定等名即等义。页面只能声明capability_ref，宿主把它映射到已登记API函数与可用能力，不能由服务端配置指定任意HTTP路径、模块导入地址或JavaScript函数。

服务端从身份得tenant/user，验证scope一致；客户端无法用body/header选择未授权租户。部分共享资源必须声明共享政策，不用空tenant代表全租户可读。

## 3. 错误与兼容

目标错误对象包含code、message、field_path、source_revision、retryable、trace_id；敏感内容不回显。结构非法/不支持版本/依赖缺失/权限拒绝/并发冲突/预算超限/发布验证失败分别表达，不用统一500掩盖。HTTP映射实施时保持既有客户端兼容，不在本轮改变现有响应。

新增可选字段需定义默认语义；必填或意义变化升契约版本，旧运行模型可由版本适配器保留。禁止运行面忽略不支持的安全/权限字段继续执行。数组补丁与null删除语义引用中央规范，不复制另一套merge规则。

## 4. 验收与消费者

对应LC-Q02/03/04/06/07；前端契约测试包含未知capability、冲突、跨租户、秘密脱敏、旧版本。后端兼容测试包含同键异输入、预期revision错误、全包发布引用缺失。消费者为meta控制面、页面宿主、flow/alliance运行适配与离线交付工具。各接口声明同一操作但不同URL时登记兼容别名，不让用户看到两个配置主源。
