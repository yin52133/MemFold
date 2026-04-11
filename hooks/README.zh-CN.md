# Hooks

[中文](./README.zh-CN.md) | [English](./README.en.md)

这个目录是 MemFold 的 **canonical hook source**。

原则：

- 所有对 Codex 的 hook 接入都从这里派生
- 本地自测版和全局部署版都受 Git 管理
- 部署态不应该手改 `~/.codex/.../hooks` 里的脚本
- 部署脚本应通过符号链接把全局 hook 指向这里

## 子目录

- `local/`
  - 仓库内自测版 hook
- `codex-global/`
  - 全局 Codex 部署态的 canonical hook

## 设计要求

- hook 只负责兜底和过滤后的默认记录
- skill/tool 负责高门槛记忆操作
- 任何对 hook 行为的修改，都必须先改这里，再重新部署
