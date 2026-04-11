# MemFold 开源借鉴与 License 说明

这份文档不属于主架构正文，而属于参考与合规说明。

主设计稿只需要说明系统怎么工作；这份文档才负责说明：

- 借鉴了哪些项目
- 借鉴了哪些部分
- 哪些部分不采用
- license 上有哪些边界

## 1. 采用原则

- 可以在设计上参考公开项目的架构思想
- 代码复用要看 license
- 对 `AGPL` 项目，只借架构思路，不直接复制代码、脚本、prompt 模板、文档原文

## 2. 借鉴映射

| 项目 | 借鉴内容 | 不采用部分 |
|------|----------|------------|
| `OpenClaw` | 分层记忆、dreaming phase、可审查记忆文件 | 冗余的文件层级 |
| `QMD` | 本地索引、collection/context tree、sidecar search | 把 QMD 当状态真相源 |
| `Claude-Mem` | observation、中间层、progressive disclosure | 代码实现、hook 产物 |
| `Hermes Agent` | memory provider 生命周期、trust/entity/contradiction 方向 | 具体 provider 绑定方式 |
| `Codex Autoresearch` | modify/verify/keep-discard/repeat、自进化闭环 | 直接把研究流程并入生产写路径 |
| `MemPalace` | archive-first 的追溯思路 | 默认暴露 raw archive |
| `llm-wiki-skill` / `llm-wiki-agent` | raw 与 wiki 分层、entity/concept/source/synthesis、lint/graph | 让 wiki 页面进入默认 boot |

## 3. License 记录

| 项目 | License | 处理方式 |
|------|---------|----------|
| `OpenClaw` | MIT | 可参考设计，可复用代码，需保留 notice |
| `QMD` | MIT | 可参考设计，可复用代码，需保留 notice |
| `Hermes Agent` | MIT | 可参考设计，可复用代码，需保留 notice |
| `Codex Autoresearch` | MIT | 可参考设计，可复用代码，需保留 notice |
| `MemPalace` | MIT | 可参考设计，可复用代码，需保留 notice |
| `llm-wiki-skill` | MIT | 可参考设计，可复用代码，需保留 notice |
| `llm-wiki-agent` | MIT | 可参考设计，可复用代码，需保留 notice |
| `Claude-Mem` | AGPL-3.0-or-later | 只借鉴架构思想，不直接复制代码或文档原文 |

## 4. 为什么单独成文

这部分放进主架构稿会打断主论证。更好的做法是：

- 主文档先讲系统是什么、如何工作
- 参考文档再讲系统借鉴了谁、为什么这样取舍

如果后续需要，可以再把这份文档扩成更正式的 literature review / prior-art review。
