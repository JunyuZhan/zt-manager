<div align="center">
  <img src="public/zt-manager-icon.svg" alt="ZeroTier Manager logo" width="112">
  <h1>ZeroTier Manager</h1>
  <p><strong>Manage your ZeroTier networks from a native Windows desktop app.</strong></p>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-24C8DB.svg" alt="MIT License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-FFC131.svg" alt="Windows 10 and 11">
    <img src="https://img.shields.io/badge/Tauri-2-24C8DB.svg" alt="Tauri 2">
  </p>
  <p><a href="#中文">中文</a> · <a href="#english">English</a></p>
</div>

> [!IMPORTANT]
> ZeroTier Manager is an independent, community-maintained project. It is not affiliated with, endorsed by, or an official product of ZeroTier, Inc.

## 中文

ZeroTier Manager 是一款面向 **Windows 10/11** 的桌面管理工具，使用 Tauri 2、React、TypeScript 和 Rust 构建。你可以在一个界面中管理本机 ZeroTier 节点、ZeroTier Central 网络以及本地控制器。

### 功能一览

| 本机节点 | ZeroTier Central | 本地控制器 |
| --- | --- | --- |
| 查看服务状态、节点信息和已加入网络 | 管理网络及成员、授权状态和成员配置 | 浏览和编辑控制器网络、成员及网络配置 |
| 启动、停止、重启服务；加入或离开网络 | 支持账号登录和 API Token 配置 | 通过本机 ZeroTier API 操作 |
| 运行连通性诊断 | 兼容 Legacy 和 New Central API | — |

### 快速开始

#### 使用应用

发布版本后，可从项目的 **Releases** 下载 Windows 安装包并启动。首次使用 Central 功能时，在「设置」中登录 ZeroTier 账号，或配置 Central API Token。

目前也可以按下方说明从源码构建。

#### 从源码构建

需要 Windows 10/11、Node.js 20.19+（或 22.12+）与 npm、Rust stable（MSVC 工具链）、Microsoft C++ Build Tools 和 WebView2 Runtime。

在项目根目录运行：

```powershell
npm ci
npm run tauri -- dev
```

构建正式版本：

```powershell
npm run tauri -- build
```

Tauri 安装包生成在 `src-tauri/target/release/bundle/`。仅检查前端类型并构建静态资源：

```powershell
npm run build
```

运行 Rust 测试：

```powershell
cargo test --workspace
```

### 配置

在应用「设置」页面登录 ZeroTier 账号，或配置 API Token、Central 模式、组织 ID 和可选的自定义 API 地址。自定义地址留空时使用默认值。

| 服务 | 默认地址 |
| --- | --- |
| Legacy Central API | `https://api.zerotier.com/api/v1` |
| New Central API | `https://central.zerotier.com/api/v2` |
| 本机 ZeroTier API | `http://127.0.0.1:9993` |

也可通过环境变量提供默认配置；应用中已保存的非空设置优先：

| 环境变量 | 用途 |
| --- | --- |
| `ZTM_CENTRAL_TOKEN` | Central API Token |
| `ZTM_CENTRAL_MODE` | `auto`、`legacy` 或 `new` |
| `ZTM_CENTRAL_ORG_ID` | New Central 组织 ID |
| `ZTM_LEGACY_BASE` | 自定义 Legacy Central API 地址 |
| `ZTM_NEW_BASE` | 自定义 New Central API 地址 |

本机节点功能需要安装并运行 ZeroTier One。读取受保护的本地 Token 或控制 Windows 服务时，系统可能会弹出 UAC 权限确认。

### 隐私与安全

- 账号密码仅用于向 ZeroTier 登录服务验证，不会保存；登录会话和 API Token 使用 Windows DPAPI 加密后存储。
- 网络请求会发送到配置的 ZeroTier Central、ZeroTier 登录服务或本机 ZeroTier API。使用自定义 API 地址前，请确认你信任该服务。
- 本项目不需要提交个人凭据即可构建或运行。请勿提交真实 Token、密码、`.env` 文件、日志或包含私有网络信息的截图。
- 发现安全问题时，请遵循 [安全问题报告说明](./SECURITY.md)，不要在公开 Issue 中披露可利用细节。

### 参与贡献

欢迎提交 Bug 报告、功能建议和代码改进。开始之前请阅读[贡献指南](./CONTRIBUTING.md)。

### 许可证

本项目按 [MIT License](./LICENSE) 发布。第三方依赖及资源仍受其各自许可证约束。

---

## English

ZeroTier Manager is a native desktop app for **Windows 10/11**, built with Tauri 2, React, TypeScript, and Rust. Manage a local ZeroTier node, ZeroTier Central networks, and controller networks in one place.

### Features

| Local node | ZeroTier Central | Local controller |
| --- | --- | --- |
| Inspect service, node, and joined-network status | Manage networks, members, authorization, and member settings | Browse and edit controller networks and members |
| Start, stop, or restart the service; join or leave networks | Sign in with a ZeroTier account or configure an API token | Operate through the local ZeroTier API |
| Run connectivity diagnostics | Supports Legacy and New Central APIs | — |

### Get started

Once a release is published, download its Windows installer from the project's **Releases** page. To build from source, install Windows 10/11, Node.js 20.19+ (or 22.12+) with npm, stable Rust with the MSVC toolchain, Microsoft C++ Build Tools, and the WebView2 Runtime. From the project root, run:

```powershell
npm ci
npm run tauri -- dev
```

Build an installer with `npm run tauri -- build`. Bundles are written to `src-tauri/target/release/bundle/`. Run `npm run build` for the frontend build or `cargo test --workspace` for Rust tests.

### Configuration

Sign in or configure a Central API token, mode, organization ID, and optional custom API URLs from **Settings**. Defaults are `https://api.zerotier.com/api/v1` (Legacy), `https://central.zerotier.com/api/v2` (New Central), and `http://127.0.0.1:9993` (local node).

Environment defaults are `ZTM_CENTRAL_TOKEN`, `ZTM_CENTRAL_MODE` (`auto`, `legacy`, or `new`), `ZTM_CENTRAL_ORG_ID`, `ZTM_LEGACY_BASE`, and `ZTM_NEW_BASE`. Saved non-empty settings take precedence. Local-node features require ZeroTier One; Windows may request UAC approval to access its token or control its service.

### Privacy and security

Account passwords are used only to authenticate with ZeroTier's login services and are not saved. API tokens and login sessions are protected at rest with Windows DPAPI. Requests go to the configured Central or login endpoint, or to the local ZeroTier API. Only use custom endpoints you trust. Do not commit credentials, `.env` files, logs, or private network details. See [SECURITY.md](./SECURITY.md) to report a vulnerability privately.

Contributions are welcome; see [CONTRIBUTING.md](./CONTRIBUTING.md). This project is distributed under the [MIT License](./LICENSE); third-party dependencies and assets remain under their respective licenses.
