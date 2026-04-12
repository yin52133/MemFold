# MemFold 系统概览

> 状态：目标态设计 Draft。当前已发货宿主接口仍以 `scope-type user|project` 和 `write-evidence` 等现有命令为准。

## 1. MemFold 是什么

MemFold 是一个分层记忆筛选器，不是全量历史仓库。它的目标是在 agent 多仓库、多 session 工作时，默认只注入最小稳定上下文；需要回溯时，再按层展开到会话日志、知识库和历史总结。

## 2. 五层结构

```text
Layer 1  启动包（boot）
  来源：stable 编译结果
  用途：启动时默认注入

Layer 2  长期记忆（stable）
  来源：dreaming 提升后的稳定事实
  用途：默认决策层

Layer 3  会话日志（session_log）
  来源：每次 session 工作过程
  用途：原始记录与追溯验证

Layer 4  知识库（wiki）
  来源：背景知识与术语页
  用途：知识检索

Layer 5  历史总结（history）
  来源：从 session_log 和 runtime logs 整理出的关键总结
  用途：dreaming 采集信号、人读回顾
```

## 3. 每层该放什么

| 层 | 放什么 | 不放什么 |
|----|--------|----------|
| `boot` | 启动时真正要注入的正文 | 追溯字段、长 hash、调试信息 |
| `stable` | 用户稳定偏好、repo 长期规则、长期约束 | 单次结论、临时路径、原始流水 |
| `session_log` | 用户原话、关键系统动作、验证结果 | 全量 tool 原文、长期规则结论 |
| `wiki` | 项目背景、术语、框架知识 | 会话流水、运行日志 |
| `history` | 某日/某阶段的关键结论、阻塞、修复、反馈 | `session_log` 的逐条镜像 |

## 4. 读取顺序

默认顺序：

```text
boot -> stable -> history -> wiki -> session_log
```

知识检索顺序：

```text
boot -> stable -> wiki -> history -> session_log
```

说明：
- `history` 先于 `session_log`，因为它是面向 dreaming 和人工回顾的压缩信号层。
- `session_log` 更原始，但成本更高，只在需要证据或精确回查时作为最后的原始层读取。

## 5. 启动时实际读什么

启动时只读：

```text
memory/user/boot/bundle.md
memory/repos/<repo_id>/boot/bundle.md
```

不会自动读：
- `family` 共享记忆
- `session_log`
- `wiki`
- `history`

## 6. Scope 结构

| Scope | 用途 | 是否启动自动加载 |
|-------|------|------------------|
| `user` | 用户长期偏好 | Yes |
| `repo` | 某个具体工程 | Yes |
| `family` | 多 repo 共享框架记忆 | No |

## 7. 自动路由规则

- `repo_id` 必须来自稳定 git 身份，不能继续用目录 basename。
- `family scope` 由全局路径/仓库路由规则自动识别，例如 `sata_coin`、`sata_stock`。
- 当用户提到某个目录、repo、项目名时，检索联查 `repo + family + user`。

## 8. 一句话分类标准

| 信息类型 | 去哪里 |
|---------|--------|
| 默认影响未来行为的稳定事实 | `stable` |
| 验证“是不是你真说过”的原始记录 | `session_log` |
| 某日/某阶段的关键总结 | `history` |
| 启动时最小注入内容 | `boot` |
| 背景说明和框架知识 | `wiki` |
