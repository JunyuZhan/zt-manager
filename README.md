<div align="center">
  <img src="public/zt-manager-icon.svg" alt="ZeroTier Manager" width="96">
  <h1>ZeroTier Manager</h1>
  <p>A Windows desktop app for managing ZeroTier nodes, Central networks, and local controllers.</p>
  <p>
    <a href="https://github.com/JunyuZhan/zt-manager/actions/workflows/ci.yml"><img src="https://github.com/JunyuZhan/zt-manager/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-0b8f87.svg" alt="MIT License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-3978c5.svg" alt="Windows 10 and 11">
    <img src="https://img.shields.io/badge/Tauri-v2-303846.svg" alt="Tauri 2">
  </p>
  <p><a href="#中文">简体中文</a> · <a href="#english">English</a></p>
</div>

## 中文

ZeroTier Manager 是一款面向 Windows 10/11 的 ZeroTier 管理桌面应用，使用 Tauri 2、React、TypeScript 和 Rust 构建。它可以管理本机 ZeroTier One、ZeroTier Central 网络和本地控制器。本项目由社区独立维护，与 ZeroTier, Inc. 无隶属或背书关系。

### 功能

- **本机节点：** 查看节点、服务和网络状态；启动、停止或重启服务；加入或离开网络；运行连通性诊断。
- **ZeroTier Central：** 登录 ZeroTier 账号或使用 API Token；管理网络、成员、授权状态和成员设置；支持 Legacy 与 New Central API。
- **本地控制器：** 通过本机 ZeroTier API 查看和编辑控制器网络、成员及网络配置。

### 下载

目前尚未发布正式安装包。发布后，安装包将在本仓库的 [Releases](https://github.com/JunyuZhan/zt-manager/releases) 页面提供。

### 从源码运行

#### 环境要求

- Windows 10 或 Windows 11
- Node.js 20.19+ 或 22.12+，以及 npm
- Rust stable 工具链和 MSVC target
- Microsoft C++ Build Tools
- WebView2 Runtime

#### 开发模式

在仓库根目录运行：

```powershell
npm ci
npm run tauri -- dev
```

#### 构建与测试

```powershell
# 前端类型检查和生产构建
npm run build

# Rust 测试
cargo test --workspace

# 构建桌面安装包
npm run tauri -- build
```

桌面安装包位于 `src-tauri/target/release/bundle/`。

### 配置

Central 登录、API Token、API 模式、组织 ID 和可选的自定义 API 地址可在应用的「设置」页面配置。API 默认地址如下：

| 服务 | 默认地址 |
| --- | --- |
| Legacy Central API | `https://api.zerotier.com/api/v1` |
| New Central API | `https://central.zerotier.com/api/v2` |
| 本机 ZeroTier API | `http://127.0.0.1:9993` |

也可使用以下环境变量提供默认值：

| 变量 | 说明 |
| --- | --- |
| `ZTM_CENTRAL_TOKEN` | Central API Token |
| `ZTM_CENTRAL_MODE` | Central API 模式：`auto`、`legacy` 或 `new` |
| `ZTM_CENTRAL_ORG_ID` | New Central 组织 ID |
| `ZTM_LEGACY_BASE` | 自定义 Legacy Central API 地址 |
| `ZTM_NEW_BASE` | 自定义 New Central API 地址 |

应用中已保存的非空设置优先于环境变量。本机节点和控制器功能需要安装并运行 ZeroTier One。访问受保护的本机 Token 或控制 Windows 服务时，Windows 可能会请求 UAC 授权。

### 隐私与安全

- 登录密码仅用于向 ZeroTier 登录服务验证，不会保存。登录会话和 API Token 使用 Windows DPAPI 加密后存储。
- 网络请求会发送到配置的 ZeroTier Central、ZeroTier 登录服务或本机 ZeroTier API。请仅配置你信任的自定义 API 地址。
- 不要提交账号密码、API Token、`.env` 文件、日志或包含私有网络信息的截图。
- 请通过 [安全政策](./SECURITY.md) 私下报告漏洞，不要在公开 Issue 中披露可利用细节。

### 参与贡献

Bug 报告、功能建议和代码贡献都欢迎。提交前请阅读[贡献指南](./CONTRIBUTING.md)。

### 许可证

本项目基于 [MIT License](./LICENSE) 发布。第三方依赖和资源仍受其各自许可证约束。

---

## English

ZeroTier Manager is a Windows 10/11 desktop app for managing a local ZeroTier One node, ZeroTier Central networks, and local controllers. It is built with Tauri 2, React, TypeScript, and Rust. The project is maintained independently by the community and is not affiliated with or endorsed by ZeroTier, Inc.

### Features

- **Local node:** Inspect node, service, and network status; start, stop, or restart the service; join or leave networks; and run connectivity diagnostics.
- **ZeroTier Central:** Sign in with a ZeroTier account or use an API token; manage networks, members, authorization, and member settings; supports Legacy and New Central APIs.
- **Local controller:** View and edit controller networks, members, and network configuration through the local ZeroTier API.

### Downloads

There is no published installer yet. Once a release is available, downloads will be published on the repository's [Releases](https://github.com/JunyuZhan/zt-manager/releases) page.

### Build from source

#### Requirements

- Windows 10 or Windows 11
- Node.js 20.19+ or 22.12+, with npm
- The stable Rust toolchain with the MSVC target
- Microsoft C++ Build Tools
- WebView2 Runtime

#### Run in development

From the repository root:

```powershell
npm ci
npm run tauri -- dev
```

#### Build and test

```powershell
# Type-check and build the frontend
npm run build

# Run Rust tests
cargo test --workspace

# Build desktop installers
npm run tauri -- build
```

Desktop bundles are written to `src-tauri/target/release/bundle/`.

### Configuration

Configure Central sign-in, an API token, API mode, organization ID, and optional custom API URLs in **Settings**. The default API endpoints are:

| Service | Default URL |
| --- | --- |
| Legacy Central API | `https://api.zerotier.com/api/v1` |
| New Central API | `https://central.zerotier.com/api/v2` |
| Local ZeroTier API | `http://127.0.0.1:9993` |

The following environment variables can provide defaults:

| Variable | Description |
| --- | --- |
| `ZTM_CENTRAL_TOKEN` | Central API token |
| `ZTM_CENTRAL_MODE` | Central API mode: `auto`, `legacy`, or `new` |
| `ZTM_CENTRAL_ORG_ID` | New Central organization ID |
| `ZTM_LEGACY_BASE` | Custom Legacy Central API URL |
| `ZTM_NEW_BASE` | Custom New Central API URL |

Non-empty settings saved in the app take precedence over environment variables. Local-node and controller features require ZeroTier One to be installed and running. Windows may request UAC approval to access the protected local token or control the service.

### Privacy and security

- Account passwords are used only to authenticate with ZeroTier's login service and are not saved. Login sessions and API tokens are encrypted at rest with Windows DPAPI.
- Requests are sent to the configured ZeroTier Central or login service, or to the local ZeroTier API. Use only custom API endpoints you trust.
- Do not commit passwords, API tokens, `.env` files, logs, or screenshots containing private network details.
- Report vulnerabilities privately according to the [security policy](./SECURITY.md); do not disclose exploitable details in a public issue.

### Contributing

Bug reports, feature suggestions, and code contributions are welcome. Please read the [contribution guide](./CONTRIBUTING.md) before submitting a change.

### License

This project is licensed under the [MIT License](./LICENSE). Third-party dependencies and assets remain subject to their respective licenses.
