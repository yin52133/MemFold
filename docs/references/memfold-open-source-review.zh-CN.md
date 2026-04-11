# MemFold 开源借鉴与 License 说明

这份文档只保留与 MemFold 直接相关的外部参考。

## 1. 采用原则

- 可以参考公开项目的架构思想
- 代码复用必须遵守原项目 license

## 2. 借鉴映射

| 项目 | 借鉴内容 | 不采用部分 |
|------|----------|------------|
| `OpenClaw` | 分层记忆、dreaming phase、可审查记忆文件 | 冗余的文件层级 |
| `QMD` | 本地索引、collection/context tree、sidecar search | 把 QMD 当状态真相源 |
| `Hermes Agent` | memory provider 生命周期、trust/entity/contradiction 方向 | 具体 provider 绑定方式 |
| `Codex Autoresearch` | modify/verify/keep-discard/repeat、自进化闭环 | 直接把研究流程并入生产写路径 |
| `MemPalace` | archive-first 的追溯思路 | 默认暴露 raw archive |
| `llm-wiki-skill` / `llm-wiki-agent` | raw 与 wiki 分层、entity/concept/source/synthesis、lint/graph | 让 wiki 页面进入默认 boot |

## 3. License

| 项目 | License | 处理方式 |
|------|---------|----------|
| `OpenClaw` | MIT | 可参考设计，可复用代码，需保留 notice |
| `QMD` | MIT | 可参考设计，可复用代码，需保留 notice |
| `Hermes Agent` | MIT | 可参考设计，可复用代码，需保留 notice |
| `Codex Autoresearch` | MIT | 可参考设计，可复用代码，需保留 notice |
| `MemPalace` | MIT | 可参考设计，可复用代码，需保留 notice |
| `llm-wiki-skill` | MIT | 可参考设计，可复用代码，需保留 notice |
| `llm-wiki-agent` | MIT | 可参考设计，可复用代码，需保留 notice |
