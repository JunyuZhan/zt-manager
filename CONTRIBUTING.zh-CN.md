# 参与贡献

欢迎提交 Bug 报告、功能建议、文档改进和代码贡献。英文版请见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 提交 Issue 前

- 搜索现有 Issue，确认没有相同的问题或建议。
- 对范围较大的改动，先开 Issue 讨论预期行为和实现范围。
- 安全漏洞请按 [SECURITY.md](SECURITY.md) 私下报告，不要在公开 Issue 中披露。

## Bug 报告和功能建议

报告 Bug 时，请提供应用版本、Windows 版本、复现步骤、预期和实际行为，以及必要且已脱敏的日志。

提出功能建议时，请说明要解决的问题、期望的使用方式和相关限制。

不要公开密码、API Token、节点或网络 ID、私有 IP 地址等敏感信息。分享日志和截图前请先脱敏。

## 配置开发环境

应用面向 Windows 10/11。按 [README.zh-CN.md](README.zh-CN.md) 安装所需环境，然后在仓库根目录运行：

```powershell
npm ci
npm run tauri -- dev
```

## 运行检查

根据改动范围运行相关检查：

```powershell
# 前端类型检查和生产构建
npm run build

# Rust 测试
cargo test --workspace

# Rust lint
cd src-tauri
cargo clippy --all-targets -- -D warnings
```

CI 会在针对 `main` 的推送和 Pull Request 上运行前端构建、Rust 测试和 Clippy。

## 提交 Pull Request

- 保持改动聚焦，无关内容请分开提交。
- 说明要解决的问题和采用的方案；适用时关联相关 Issue。
- 附上相关测试结果；界面改动请提供截图。
- 改动影响功能、安装或配置时，同步更新文档。
- 在可行时添加或更新测试。
- 不要提交构建产物、本地配置、凭据或私有网络信息。

请遵循代码库现有风格，使用清晰的提交信息。提交贡献即表示你同意该贡献依照本仓库的 [MIT License](LICENSE) 发布。目前无需签署单独的贡献者许可协议。
