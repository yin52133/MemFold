# MemFold For Codex

[中文](./README.zh-CN.md) | [English](./README.en.md)

这一组文档用于说明如何把 MemFold 集成到 Codex。

## 文档入口

- [全局部署指南](./deployment.zh-CN.md)
- [AGENTS 示例](./AGENTS.example.md)

## 结构约定

- `hooks/` 是 canonical hook source
- `skills/` 是 canonical skill source
- `scripts/` 提供安装和验证脚本

这样其他项目在接入时，只需要：

1. 引用本仓库的 `hooks/` 和 `skills/`
2. 运行部署脚本
3. 按验证脚本确认 Codex 能直接调用 MemFold
