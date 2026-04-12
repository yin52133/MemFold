# Plugins

[中文](./README.zh-CN.md) | [English](./README.en.md)

这个目录是 MemFold 面向 Codex 的唯一插件来源。

当前约定：

- 运行态 skill 只放在 `plugins/memfold/skills/`
- 命令定义只放在 `plugins/memfold/commands/`
- 其他项目接入时，应安装整个 plugin，而不是复制单独 skill
