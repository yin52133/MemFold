# 04 Dreaming And Retention

## 1. 这份文档讲什么

工作记录里的信息，什么时候能进入长期记忆，什么时候继续观察，什么时候必须隔离，什么时候直接丢掉。

如果这个问题说不清，dreaming 就会从"整理器"变成"噪音制造器"。

## 2. Dreaming 的触发

Dreaming 有三种触发方式：

| 触发 | 说明 |
|------|------|
| `manual` | 用户或宿主显式触发（`memfold dream run --trigger manual`） |
| `session_end` | 宿主在 session 结束时触发（第一版可选实现） |
| `scheduled` | 门控触发：距上次 ≥24h 且累计新 session ≥5 |

V1 优先实现 `manual`，门控 `scheduled` 逻辑次之，`session_end` 最后。

门控规则来自 ccb auto-dream 的设计：高频触发 dreaming 没有价值，信号需要积累到足够数量才有整合意义。

## 3. Dreaming 的职责

Dreaming 只做四选一决策：

```
1. 保留  → 进入长期记忆（stable/*.md）
2. 暂存  → 继续留在工作记录，等待复现
3. 隔离  → 进入隔离区，写 tombstone，不参与后续晋升
4. 丢弃  → 不保留任何记录
```

Dreaming 不负责：
- 自由总结
- 生成新事实
- 把所有工作记录都升成长期记忆

## 4. 四阶段流程

```
Phase 1: Orient（定位）
  │  读现有 stable/*.md
  │  了解长期记忆现状
  │  避免重复晋升相同内容
  ▼

Phase 2: Gather（采集信号）
  │  优先：archive/memory-YYYY-MM-DD.md（人可读的日志）
  │  次：过时记忆（与当前文件状态矛盾的条目）
  │  最后：窄关键词 grep sessions/*/evidence.jsonl（不全文读）
  ▼

Phase 3: Consolidate（整合）
  │  四选一决策（见第 5 节）
  │  合并到现有主题文件，不创建近似重复
  │  相对日期（"昨天"/"上周"）转绝对日期
  │  删除被推翻的事实
  ▼

Phase 4: Prune（修剪与索引）
     重编 boot/bundle.md
     同步 QMD 索引
     更新 SQLite 投影
     每个主题文件保持合理大小
```

## 5. 四选一决策规则

```
候选条目
      │
      ▼
claim_fingerprint 命中 tombstone？
      │
     是│                    │否
      ▼                     ▼
   直接丢弃        满足以下任一条件？
                  a. 用户明确说"记住这个"
                  b. 稳定用户偏好（如语言、风格）
                  c. 稳定项目约束（如禁止某类操作）
                  d. 跨多 session 复现且从未被纠正
                       │
                      是│                    │否
                       ▼                     ▼
                保留，写入 stable     只出现过一次 / 证据不足？
                                           │
                                          是│                │否
                                           ▼                 ▼
                                         暂存          与长期记忆冲突？
                                      继续观察         或用户说"这个不对"？
                                                           │
                                                          是│     │否
                                                           ▼      ▼
                                                          隔离   丢弃
                                                      写 tombstone
```

## 6. 输入边界

Dreaming 只能消费：
- 可晋升的工作记录（`promotable=1`）
- 安全清洗后的历史档案摘要
- 现有长期记忆的比较项

不能直接消费：
- 分析草稿
- 原始敏感正文（含 token / key / cookie）
- `sterile` 模式下产生的临时记录（`origin_mode=sterile`）

## 7. 为什么分析草稿不能参与晋升

```
系统猜了一次（分析草稿）
      │
      ▼
  允许进工作记录？
      │
  允许 ▼
把自己的猜测记下来
      │
      ▼
  允许晋升？
      │
  允许 ▼
猜测变成长期记忆（自我污染）
```

规则写死：
- 分析草稿不能进入工作记录
- 分析草稿不能直接进入长期记忆
- 分析草稿只能帮助解释，不帮助记住

## 8. 为什么只靠 status 挡不住错误记忆

```
原错误记忆 A（status=rejected）
      │
      ▼
换一个 summary，变成 B
      │
      ▼
系统把 B 当新东西
      │
      ▼
B 又自动晋升进长期记忆
```

解决方案：tombstone + claim_fingerprint

tombstone 封的不是某条记录，而是**语义断言本身**。换 summary 换 title 只要 claim_fingerprint 相同，都会被命中。

## 9. Claim Fingerprint 生成规则

`claim_fingerprint` 是对语义断言的稳定标识，目的是识别"换壳复活"。

第一版实现规则：

1. 从 `summary` 提取核心断言词（去掉修饰词、连接词、时间状语）
2. 排序后拼接成规范字符串
3. 取 SHA-256 前 16 字节，hex 编码

示例：
```
summary: "用户明确要求默认用中文回答所有问题"
核心词提取: ["中文", "回答", "默认", "用户"]
排序拼接: "中文-回答-默认-用户"
fingerprint: cfp_3a7f2e1b4c9d8e6f
```

规则不追求完美语义相似度，只追求：
- 相同断言 → 相同 fingerprint（稳定性）
- 换壳后核心断言不变 → 仍然命中

如果第一版规则太弱（漏判），在 experiments 阶段用离线数据集调整，不在生产里试错。

## 10. 验收

### 10.1 错误记忆不可复活

失败样例：
- 被 tombstone 覆盖的 claim 换 summary 后重新晋升
- 被 rejected 的条目在下一次 dreaming 时再次出现在候选列表

通过门槛：
- 被 tombstone 命中的 claim 自动晋升率为 0

### 10.2 Dreaming 不自我污染

失败样例：
- 分析草稿内容出现在 stable/*.md
- `origin_mode=sterile` 的 evidence 被标记为 `promotable=1`

通过门槛：
- 纯分析输入的晋升率为 0
- sterile 来源的晋升率为 0

### 10.3 敏感信息不入库

失败样例：
- token / cookie / private key 出现在任意存储层

通过门槛：
- 原文泄漏率为 0

## 11. 如何持续改进规则

只能靠离线验证来改，不能靠生产里自由试错。

正确顺序：
1. 定义失败样例
2. 改规则
3. 验证错误率、复活率、污染率是否下降

改进对象是决策规则（保留/暂存/隔离/丢弃的边界），不是让系统在生产里自由尝试。
