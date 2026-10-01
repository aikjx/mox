# 云盘、知识、图谱与对象提供方接口契约

> RK-API-01 · V1.0 · 2026-10-01 · 目标逻辑操作；不虚构新端点已存在。

实施说明：本文现状取自本轮实施前的代码基线，目标契约保持独立；已落地变化及兼容性以[实施台账](docs/modules/resource-knowledge/IMPLEMENTATION-STATUS.md#compatibility)为准。
<a id="operations"></a>
## 1. 业务操作

| 逻辑操作 | 输入 | 输出与关键约束 |
|---|---|---|
| 查询目录/资源/版本 | 受信scope、parent/filter、cursor | 授权投影、分页；不能客户端tenant过滤代替授权 |
| 开始/续传/完成上传 | 幂等键、size/MIME/digest、目标folder、expected revision | upload_id/part回执/精确file_version；完成前不发布 |
| 下载/预览 | resource/version、Range意图 | 内容或短期受权读取；读前授权、无本机路径泄露 |
| 移动/重命名/分享 | resource、目标、expected revision、有效范围 | 元数据版本/授权回执；不搬字节/不扩权 |
| 回收/恢复/请求彻底删除 | resource、expected revision、原因 | 受控状态或删除job；锁定/引用冲突显式返回 |
| 导入/更新知识源 | space、精确源版本、pipeline_ref、幂等键 | document revision与加工job，不同步冒认全部挂图完成 |
| 预览/发布/撤回知识 | 精确revision、expected published revision、证据 | 发布指针/风险/质量回执，审计可追溯 |
| 重加工/重建索引 | 精确源与目标index generation、预算 | job/attempt、阶段回执；旧结果不能覆盖新版本 |
| 关联/确认/撤销图关系 | 实体引用、谓词、证据、expected revision | 关系断言版本与审核；不是直接写源业务状态 |
| 搜索与关系追踪 | 查询、授权空间、版本策略、cursor/limit | 命中、来源、实际渠道/索引代际、可见关系路径 |
| 提供方验证/新写默认/迁移 | backend revision、能力集、scope、manifest | 验证证据/生效指针/迁移job；三个操作分离 |

owner与标识由[数据契约](docs/database/RESOURCE-KNOWLEDGE-DATA-CONTRACT.md#ownership)定义。字段结构与覆盖复用[配置契约](docs/api/LOWCODE-CONFIGURATION-CONTRACT.md#contract)。命令幂等键按tenant+operation+key原子接纳，同键异payload报冲突；接纳中返回待处理状态，不能因超时自动执行第二次副作用。

<a id="providers"></a>
## 2. 对象提供方能力矩阵

提供方声明backend_id、协议、地址模式、认证secret_ref、允许endpoint、容量限制、revision与capabilities。最小必测put/get/head/delete、checksum语义与list分页；大文件再验multipart/resume/abort、Range；版本保留、条件写、预签名、对象锁和归档restore分别验收。unsupported不得映射成成功空回执。

| 提供方 | 当前来源与限制 | 目标适配要求 |
|---|---|---|
| local FS | 网关CloudState扁平key；store-core内容寻址是另一条基础存储路径 | 统一BlobLocation门面，内部适配，不立刻混合两种布局 |
| S3/MinIO | 网关与store-core有各自SigV4客户端；实现声明不代表本环境已往返 | 先共用能力/认证配置和契约测试，再迁移内部实现；endpoint/分页/条件请求逐项验 |
| 阿里云OSS | 存在Oss枚举与复用path-style客户端的代码 | 按官方兼容范围、地址模式与真实bucket验证，未验不标ready |

阿里云官方文档说明S3兼容请求仅支持virtual-hosted style，并列出ACL、存储类别、ETag等差异。现有path-style实现不能仅改kind=Oss就宣称适配完成：[OSS官方兼容范围](https://www.alibabacloud.com/help/en/oss/developer-reference/compatibility-with-amazon-s3)。额外供应商适配按其官方契约与真实测试独立登记，不套用同一个“兼容所有OSS”声明。

<a id="compatibility"></a>
## 3. 既有接口兼容与错误

网关当前/api/kb、/cloud/v1与/api/storage沿用现行注册表。kb-server的/api/v1/kb和网关KbDocument不直接互换；建立字段/版本/来源映射和消费者回归后再接线。不得删旧API后只留下新文档。

现有storage/switch响应switched=true没有active写入指针，设计上保留兼容响应但新增语义说明，未来将“验证可达”和“切新写默认”分别登记；切换不会自动迁移旧对象。实施前确定是否兼容字段增量或版本化新端点，不能本轮默改HTTP行为。

目标错误区分unauthorized/forbidden/not_found、revision_conflict、unsupported_capability、integrity_mismatch、quota_exceeded、dependency_missing、processing_failed、provider_unavailable、effect_unknown。内部详因进入脱敏诊断，用户不见密钥/路径/无权资源是否存在。跨域操作返回job和分阶段结果，不能一处失败返回500后丢失前面成功回执。

<a id="retrieval"></a>
## 4. 授权检索与证据引用

返回至少包含document_id/revision、chunk_id/source_locator、片段、来源可用状态、实际召回渠道与索引代际。向量模型不可用时，只有明确允许的关键词降级才能返回，并披露实际渠道；不能伪造向量分数。实体邻居/关系扩展同样过滤租户与源资源授权，图统计与缓存也不可泄露隐藏对象。

RAG将来源内容视为数据，文档指令不能覆盖系统权限或调用工具；送往模型前再次授权及敏感字段裁剪。发布文档与文件下载权限可不同，引用文件不自动分享原件；返回引用内容需有相应权利。出口RK-Q01/02/04/05/07/10。


## 网关 KB 只读共享增量契约

`GET /api/kb/documents/:id/shares` 返回 `{readers, acl_revision}`；`POST` 同一路径接受 `{user_id, expected_acl_revision}`；`DELETE /api/kb/documents/:id/shares/:user_id?expected_acl_revision=N` 撤销该接收者。只有当前租户内的所有者/管理员可管理，接收者不能转授权或修改文档。当前只提供用户只读共享，用户 ID 绑定文档租户，未对接目录用户存在性验证。空白/控制字符/超长 ID 或超过 256 个接收者返回 400；ACL 版本冲突返回 409；无权与不存在均返回 404。重复授权/撤权在版本匹配时幂等。

ACL 独立于内容版本，保存在当前文档主源；版本回滚不恢复历史 ACL。检索和图谱按当前 ACL 筛选，撤权提交后的新请求不可见；已开始的读取可能完成，不承诺收回已经下载的内容。并发修改以同一状态实例的互斥锁和 expected_acl_revision 防止丢失更新；跨进程并发仍需存储级 CAS。迁移、编辑共享、群组、过期链接、完整安全审计不属于此增量。


## 当前网关 KB 条件编辑契约

`PUT /api/kb/documents/:id` 增量接受 title/content/category/tags，并可携带 `expected_current_version` 和 `version_note`。当前版本不匹配返回 409；归属/写权限失败返回 404；存储失败返回通用 500。修改正文/标题或显式提供非空备注时，旧正文只归档一次，当前版本只推进一次。没有正文变化且没有备注的元数据修改不推进内容版本。此预期版本保护针对内容版本，不能代替完整元数据 revision/CAS。

前端编辑入口先读取完整文档再填写表单，禁止以列表摘要的空正文直接覆盖源文档；提交只发一个 PUT，冲突保留尚未保存的表单。旧客户端不提供 expected_current_version 的兼容写入仍存在。内部分析/建版/回滚使用完整快照比较保存，失败返回 409；不同进程并发仍需存储级 CAS。


## 当前 KB 检索引用与请求边界

`POST /api/kb/search` 要求非空查询且不超过 512 个 Unicode 字符、limit 在 1–100；越界返回 400。文档命中新增 citation：document_id、version、field（content/title）、start_char、end_char。区间为原始字段 Unicode 标量字符的零基半开区间，不是 UTF-8 字节或 UTF-16 码元；按该版本字段切片得到 snippet。大小写转换后仍映射到原始字符，避免 Unicode 展开偏移越界。正文未命中时引用版本化标题，不把未版本化分析摘要伪装成稳定来源。

引用不携带永久访问权：查看当前/历史文档仍按当前 ACL；旧引用随撤权不可继续取源。此结构只覆盖网关 KB 文本来源，不等同于文件页码、解析块、向量证据或真实模型答案引用。
