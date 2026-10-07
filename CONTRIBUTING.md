# Contributing to ZeroTier Manager

Thank you for taking the time to contribute. Bug reports, feature requests, documentation improvements, and code changes are welcome.

## Before you start

- Search existing issues before opening a new one.
- For a significant change, open an issue first to discuss the proposed behavior and scope.
- For security vulnerabilities, follow [SECURITY.md](./SECURITY.md) instead of opening a public issue.

## Report a bug or request a feature

When filing an issue, include the information needed to understand the report:

- For a bug: application version, Windows version, steps to reproduce, expected behavior, actual behavior, and relevant redacted logs.
- For a feature request: the problem to solve, the expected user experience, and any relevant constraints.

Do not include passwords, API tokens, node or network IDs, private IP addresses, or other sensitive information. Redact these details from logs and screenshots before sharing them.

## Set up a development environment

The application currently targets Windows 10/11. Install the prerequisites listed in the [README](./README.md), then run these commands from the repository root:

```powershell
npm ci
npm run tauri -- dev
```

## Validate your changes

Run the checks that apply to your changes:

```powershell
# Frontend type-check and production build
npm run build

# Rust tests
cargo test --workspace

# Rust lint (matches CI)
cd src-tauri
cargo clippy --all-targets -- -D warnings
```

The CI workflow runs the frontend build, Rust tests, and Clippy on pushes and pull requests targeting `main`.

## Open a pull request

- Keep each pull request focused; separate unrelated changes.
- Explain the user-visible problem and the approach taken. Link related issues when applicable.
- Include relevant test results and screenshots for visual changes.
- Update the README or other documentation when behavior, setup, or configuration changes.
- Add or update tests when practical.
- Do not commit build output, local configuration, credentials, or private network data.

## Code and commit conventions

- Follow the existing patterns, naming, and formatting in the codebase.
- Prefer small, readable changes and report errors explicitly.
- Use clear commit messages that describe the change.

## License

By submitting a contribution, you agree that it may be distributed under the terms of this repository's [MIT License](./LICENSE). No separate contributor license agreement is currently required.

---

# 参与贡献

感谢你愿意为 ZeroTier Manager 做出贡献。我们欢迎 Bug 报告、功能建议、文档改进和代码贡献。

## 开始之前

- 提交新 Issue 前，请先搜索是否已有相同或相关问题。
- 对于影响范围较大的改动，建议先开 Issue 讨论行为和范围。
- 安全漏洞请按 [安全政策](./SECURITY.md) 私下报告，不要公开漏洞细节。

## 报告 Bug 或提出功能建议

请提供足以理解和处理问题的信息：

- Bug：应用版本、Windows 版本、复现步骤、预期行为、实际行为，以及必要且已脱敏的日志。
- 功能建议：希望解决的问题、预期的用户体验和相关限制。

请勿包含密码、API Token、节点或网络 ID、私有 IP 地址及其他敏感信息。分享日志或截图前，请先脱敏。

## 配置开发环境

应用当前面向 Windows 10/11。请按 [README](./README.md) 安装所需环境，然后在仓库根目录运行：

```powershell
npm ci
npm run tauri -- dev
```

## 验证改动

根据改动范围运行适用的检查：

```powershell
# 前端类型检查和生产构建
npm run build

# Rust 测试
cargo test --workspace

# Rust lint（与 CI 一致）
cd src-tauri
cargo clippy --all-targets -- -D warnings
```

CI 会在针对 `main` 的推送和 Pull Request 上运行前端构建、Rust 测试和 Clippy。

## 提交 Pull Request

- 保持每个 Pull Request 聚焦；无关改动请分开提交。
- 说明用户遇到的问题及采用的解决方案；适用时关联相关 Issue。
- 附上相关测试结果；界面改动请提供截图。
- 如果改动影响行为、安装或配置方式，请同步更新文档。
- 在可行时添加或更新测试。
- 不要提交构建产物、本地配置、凭据或私有网络信息。

## 代码与提交约定

- 遵循代码库中已有的模式、命名和格式。
- 优先编写小而清晰的改动，并明确报告错误。
- 提交信息应简洁说明改动内容。

## 许可证

提交贡献即表示你同意该贡献依照本仓库的 [MIT License](./LICENSE) 发布。目前无需签署单独的贡献者许可协议。
