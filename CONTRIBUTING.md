# Contributing

Thanks for helping improve ZeroTier Manager! Bug reports, thoughtful feature requests, documentation fixes, and code contributions are welcome.

## Development setup

The current application targets Windows 10/11. Install the prerequisites in [README.md](./README.md), then run:

```powershell
npm ci
npm run tauri -- dev
```

## Before submitting a change

Run the checks relevant to your change:

```powershell
npm run build
cargo test --workspace
```

Please:

- Keep pull requests focused and explain the user-visible change.
- Add or update tests when practical.
- Update user or developer documentation when behavior changes.
- Do not include credentials, `.env` files, build outputs, personal configuration, or private network details.
- Redact tokens, node/network IDs, IP addresses, and other sensitive details from logs and screenshots.

## Reporting bugs

Include the application version, Windows version, steps to reproduce, and relevant **redacted** errors. Do not post credentials or private network information.

## License

By submitting a contribution, you agree that it may be distributed under this repository's [MIT License](./LICENSE).

---

## 中文贡献指南

欢迎参与 ZeroTier Manager！我们欢迎 Bug 反馈、功能建议、文档改进和代码贡献。

### 开发环境

当前应用面向 Windows 10/11。请先按 [README](./README.md) 安装所需环境，再运行：

```powershell
npm ci
npm run tauri -- dev
```

### 提交前检查

根据改动范围运行相关检查：

```powershell
npm run build
cargo test --workspace
```

请保持改动聚焦，说明用户可见的变化，并在适当时补充或更新测试和文档。不要提交凭据、`.env` 文件、构建产物、个人配置或私有网络信息；分享日志和截图前，请先删除 Token、节点/网络 ID、IP 地址等敏感信息。

### 报告 Bug

请提供应用版本、Windows 版本、复现步骤和已脱敏的错误信息。请勿公开凭据或私有网络数据。

### 许可证

提交贡献即表示你同意该贡献依照本仓库的 [MIT License](./LICENSE) 发布。
