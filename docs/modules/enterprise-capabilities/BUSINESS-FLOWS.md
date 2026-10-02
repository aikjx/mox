# 跨模块业务主链与异常恢复

> ENT-FLOW-01 · 2026-10-02。以下是统一验收目标，具体已实现段以各模块权威为准；每模块局部流程见 [模块卡](MODULES.md)，前置能力见 [关系图](RELATIONSHIPS.md)。本页不声称所有连线已接通。

## F01 可信请求与受权命令（EM-01–05）

```mermaid
flowchart LR
    U[用户或机器请求] --> G[网关协议与认证]
    G --> I[实际 IAM 身份与租户]
    I --> P{资源和业务权限通过?}
    P -->|否| X[拒绝且不创建业务事实]
    P -->|是| C[领域命令及事务]
    C --> A[同事务审计或持久化事件意图]
    A --> R[提交后的实际回执]
    C --> E[冲突失败或未知结果]
```

身份只从可信凭证得到；写授权与写入顺序按领域契约实施。冲突返回原版本，未知结果优先查原幂等请求。审计、事务和外部副作用跨存储时采用明确的 outbox/对账边界，不伪造跨库原子提交。[IAM](../iam/README.md)及[凭证](../iam/API-KEYS.md)为本链具体权威。

## F02 专家任务到成果（EM-06–12、23）

```mermaid
flowchart TD
    GOAL[受权业务目标] --> IN[规范化输入与硬约束]
    IN --> MATCH[受权可用专家匹配及理由]
    MATCH --> PLAN[冻结 DAG 与配置版本]
    PLAN --> CLAIM[持久化任务领取和租约]
    CLAIM --> EX[真实工具或模型执行]
    EX --> GOV{证据及治理通过?}
    GOV -->|是| RESULT[融合分析或执行结果]
    RESULT --> ACCEPT[用户接受并关联项目成果]
    GOV -->|否| FAIL[停止并记录实际失败]
    EX --> UNKNOWN[外部结果未知]
    UNKNOWN --> RECON[查原请求与副作用回执]
    RECON --> CLAIM
```

咨询分析、任务调度和工具执行不能被一个“完成”布尔值替代。租约过期者不能写终态；预算、审批、取消和真实工具适配是执行前置。详细状态与远程/本地形态见[联盟流程](../../expert-alliance/13-end-to-end-business-flow.md)，未验收工具和提供方保留不可用。

## F03 低代码业务包发布与流程运行（EM-12–14、24）

```mermaid
flowchart LR
    D[业务包或配置草稿] --> V[类型与引用依赖校验]
    V --> AUTH[实际发布权限]
    AUTH --> RELEASE[冻结 release 与迁移预检]
    RELEASE --> BIND[宿主绑定已有能力]
    BIND --> RUN[流程实例固定版本运行]
    RUN --> WAIT[审批或实际回执]
    WAIT --> DONE[领域命令提交]
    V --> REJECT[缺能力或未知组件拒绝]
    RUN --> RECOVER[持久化恢复或补偿]
```

只配置字段/菜单不等于已具有写权限。禁止远程字符串 eval；未知组件或版本拒绝。回滚新接纳任务的绑定不改变在途任务版本；卸载先检查依赖与在途引用。[业务包装配](../LOWCODE-MODULE-ASSEMBLY.md)、[配置标准](../../standards/lowcode-dynamic-configuration.md)和[工作流 ADR](../../enterprise/33-持久化工作流引擎设计文档-ADR-14.md)维护细节。

## F04 文件到知识源（EM-15–17）

```mermaid
flowchart LR
    F[受权文件或内联知识] --> O[真实对象写入与完整性]
    O --> META[提交文件及知识源版本引用]
    META --> ACL[发布共享与 ACL]
    ACL --> READ[授权读取指定版本]
    O --> ORPHAN[元数据失败或对象结果未知]
    ORPHAN --> CHECK[孤儿和回执对账]
    ACL --> REVOKE[撤权回收及旧引用可见性裁决]
```

对象拥有字节，文件拥有目录/资源元数据，知识拥有源/内容版本。移动不搬字节；删除策略处理仍被引用的对象。上传完成须记录真实提供方回执与版本绑定。[资源知识数据契约](../../database/RESOURCE-KNOWLEDGE-DATA-CONTRACT.md)和[实际流程](../resource-knowledge/BUSINESS-FLOWS.md)为细节权威。

## F05 加工、图投影与授权回答（EM-18–20）

```mermaid
flowchart TD
    SOURCE[已授权精确源版本] --> JOB[冻结加工配置与 generation]
    JOB --> WORK[租约下真实解析分块抽取]
    WORK --> CHECK{版本 ACL 与租约仍有效?}
    CHECK -->|否| OLD[拒绝旧作业提交]
    CHECK -->|是| FACT[候选事实及人工确认]
    FACT --> INDEX[可重建图和检索投影]
    QUERY[受权问题] --> CAND[授权范围召回]
    INDEX --> CAND
    CAND --> CIT[排序并验证版本化引用]
    CIT --> ANSWER[真实模型回答与证据]
    SOURCE --> DELETE[撤权删除传播与投影重建]
```

召回前授权，随后再次核对引用有效性。图与索引为投影，不能自称知识主源；源变更或删除传播需要 generation 和对账。引用缺失时展示证据不足，模型不可用时不返回模板答案。[资源知识台账](../resource-knowledge/IMPLEMENTATION-STATUS.md)记录实际已实现范围。

## F06 数据同步与企业事务（EM-21）

```mermaid
flowchart LR
    S[真实受权源与游标] --> MAP[类型化映射和校验]
    MAP --> COMMIT[目标幂等事务提交]
    COMMIT --> CURSOR[推进游标及保存回执]
    CURSOR --> DIFF[增量和删除对账]
    COMMIT --> UNKNOWN[结果未知查目标幂等记录]
    S --> MISSING[连接器缺失返回未实现]
```

不得在未接通源和目标时生成成功计数。目标提交与游标不在一库时明确重放边界；按真实目标回执判定成功。[平台台账](../REAL-IMPLEMENTATION-STATUS.md)记录当前未完成项。

## F07 消息交付与独立阅读（EM-22）

```mermaid
flowchart TD
    FORM[发送人填写真实收件人 ID] --> REQUEST[冻结内容和幂等键]
    REQUEST --> IAM[实际 IAM 权限与全部收件人校验]
    IAM --> TX[消息和独立回执批量事务]
    TX --> RECEIPT[实际 message_id]
    RECEIPT --> INBOX[通知读适配和独立已读]
    REQUEST --> NETWORK[网络结果未知]
    NETWORK --> RETRY[保留原内容和键显式重试]
    RETRY --> IAM
    IAM --> DENY[停用或越权拒绝]
```

站内提交不代表邮件或短信投递。未知结果重试不修改收件人/内容；新消息明确新建命令。跨身份切换废弃旧页面回执和草稿，不能撤销已提交服务端消息。授权/事务细节见[消息中心](../message-center/README.md)；外部 outbox 属后续工作。

## F08 产品交付与运营（EM-23–26）

```mermaid
flowchart TD
    PRODUCT[项目或独立产品目标] --> EVIDENCE[关联真实任务制品与接受记录]
    EVIDENCE --> VERIFY[用户流程及正反权限故障验证]
    VERIFY --> CAPACITY[实测容量成本 SLO]
    CAPACITY --> RELEASE[灰度部署与实际依赖探测]
    RELEASE --> MONITOR[指标 trace 与回执对账]
    MONITOR --> INCIDENT[故障或容量越界]
    INCIDENT --> RESTORE[回滚与灾备恢复验真]
    RESTORE --> VERIFY
```

语音/媒体输入单独验证格式、时长、预算、实际制品 ACL 与取消清理；模板/市场包验证来源、许可、依赖和迁移。跨主机运行、恢复演练和外部提供方成功链路要使用真实配置验证。用户满意度与最优性能需要持续测量，不由架构图直接推出。
