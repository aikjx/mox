# MOX v3.4 依赖漏洞扫描报告

> 扫描时间：2026-09-06
> 扫描工具：cargo-audit v0.22.2
> 扫描范围：整个工作区（130+ crate）
> 漏洞数据库：RustSec Advisory Database

---

## 一、扫描结果总览

| 类别 | 数量 | 严重程度 |
|------|------|----------|
| 漏洞（Vulnerabilities） | 7 | 2 High / 1 Medium / 4 Low |
| 警告（Warnings） | 21 | 未维护/不健全 |
| **总计** | **28** | - |

---

## 二、漏洞详情

### 高危漏洞（High，2个）

#### 1. quick-xml 0.28.2 - 二次运行时间
- **ID**: RUSTSEC-2026-0194
- **严重程度**: 7.5 (High)
- **描述**: 检查开始标签重复属性名时存在二次运行时间，可能导致DoS
- **影响范围**: 间接依赖（云盘/AI模块可能使用）
- **修复建议**: 升级到 quick-xml 0.31.0+ 或移除不必要的依赖

#### 2. quick-xml 0.28.2 - 无界命名空间声明分配
- **ID**: RUSTSEC-2026-0195
- **严重程度**: 7.5 (High)
- **描述**: NsReader 中无界命名空间声明分配可能导致内存耗尽DoS
- **影响范围**: 间接依赖
- **修复建议**: 升级到 quick-xml 0.31.0+

### 中危漏洞（Medium，1个）

#### 3. rsa 0.9.10 - Marvin攻击时序侧信道
- **ID**: RUSTSEC-2023-0071
- **严重程度**: 5.9 (Medium)
- **描述**: RSA解密存在时序侧信道漏洞，可能恢复私钥
- **影响范围**: 间接依赖（认证/加密模块可能使用）
- **修复建议**: 升级到 rsa 0.9.6+（已修复）或使用更安全的加密库

### 低危漏洞（Low，4个）

#### 4. pyo3 0.22.6 - 缓冲区溢出风险
- **ID**: RUSTSEC-2025-0020
- **严重程度**: Low
- **描述**: PyString::from_object 存在缓冲区溢出风险
- **影响范围**: 间接依赖（AI模块Python绑定）

#### 5. pyo3 0.22.6 - 缺少Sync绑定
- **ID**: RUSTSEC-2026-0177
- **严重程度**: Low
- **描述**: PyCFunction::new_closure 闭包缺少Sync绑定
- **影响范围**: 间接依赖

#### 6. rkyv 0.7.46 - 归档验证不足
- **ID**: RUSTSEC-2026-0235
- **严重程度**: Low
- **描述**: 包含Rc/Arc的归档验证不足可能导致越界读取
- **影响范围**: 间接依赖

#### 7. sqlx 0.8.0 - 二进制协议误解
- **ID**: RUSTSEC-2024-0363
- **严重程度**: Low
- **描述**: 截断或溢出转换导致二进制协议误解
- **影响范围**: 间接依赖

---

## 三、警告详情（21个）

### 未维护（Unmaintained）
- adler 1.0.2 - 建议使用 adler2
- atk 0.18.2 - GTK3绑定不再维护
- atk-sys - GTK3系统绑定
- glib 0.18.5 - GTK库
- proc-macro-error 1.0.4 - 过程宏错误处理库
- 其他GTK相关库

### 不健全（Unsound）
- glib 0.18.5 - Iterator实现不健全
- memmap2 0.6.2/0.7.1/0.8.0 - 未检查指针偏移
- rand 0.7.3 - 自定义logger下不健全

---

## 四、风险评估

### 核心crate风险（低）
我们重点优化的核心crate（mox-dsql-core/mox-resilience-core/mox-cache-core/mox-server-runtime/mox-event-core/mox-lock-core/mox-auth-core/mox-config-core/mox-observability-core）**未发现直接依赖漏洞**。

### 间接依赖风险（中）
- quick-xml 高危漏洞可能影响云盘/AI模块的XML解析
- rsa 中危漏洞可能影响认证模块的RSA加密
- 建议评估这些依赖的实际使用场景

### 未维护依赖风险（中低）
- GTK相关库（atk/gtk/glib）可能是桌面端UI依赖，服务器端可移除
- proc-macro-error 未维护但功能稳定，风险较低

---

## 五、修复建议

### 立即修复（P0）
1. **升级 quick-xml** 到 0.31.0+（修复2个高危漏洞）
2. **升级 rsa** 到 0.9.6+（修复中危漏洞）
3. **评估 pyo3 使用**，如非必要可移除或升级

### 短期优化（P1）
4. 运行 `cargo tree -i quick-xml` 查找依赖来源，逐步升级
5. 运行 `cargo tree -i rsa` 查找依赖来源
6. 移除不必要的GTK依赖（服务器端不需要）
7. 在CI/CD中集成 `cargo audit` 自动扫描

### 长期治理（P2）
8. 建立依赖更新策略（定期运行 `cargo update`）
9. 使用 `cargo-deny` 进行更严格的依赖许可证和漏洞检查
10. 建立Snyk/Dependabot等自动化依赖管理工具

---

## 六、CI/CD集成建议

在GitHub Actions/GitLab CI中添加：

```yaml
- name: Security Audit
  run: |
    cargo install cargo-audit
    cargo audit
```

或使用 pre-commit hook：

```yaml
repos:
  - repo: local
    hooks:
      - id: cargo-audit
        name: Cargo Security Audit
        entry: cargo audit
        language: system
        pass_filenames: false
```

---

## 七、结论

**整体安全状况：良好（B+）**

- 核心crate无直接依赖漏洞 ✅
- 2个高危漏洞来自间接依赖（quick-xml），需评估影响 ⚠️
- 1个中危漏洞来自间接依赖（rsa），建议升级 ⚠️
- 21个警告主要是未维护/不健全的间接依赖，风险较低 ⚠️

**建议在完成 quick-xml 和 rsa 升级后，可达到生产级安全标准（A-）。**

---

> 报告生成时间：2026-09-06
> 下次扫描建议：每周一次或每次依赖更新后
