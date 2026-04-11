# Skills

[中文](./README.zh-CN.md) | [English](./README.en.md)

这个目录是 MemFold 对外暴露给 Codex 的 **canonical skill source**。

原则：

- skill 定义必须和实际部署态同步
- 其他项目接 MemFold 时，应直接引用这里，而不是去 `~/.codex/skills` 里复制
- 全局部署脚本应把 `~/.codex/skills/memfold-codex-memory` 链接到这里

## 当前 skill

- `memfold-codex-memory/`

## 作用

- 定义 Codex 什么时候该调用 `load/search/write-evidence/feedback/dream/repair`
- 约束 `hook + skill/tool` 的职责边界
