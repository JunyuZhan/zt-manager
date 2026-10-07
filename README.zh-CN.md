<div align="center">
  <img src="public/zt-manager-icon.svg" alt="ZeroTier Manager" width="96">
  <h1>ZeroTier Manager</h1>
  <p>用于管理 ZeroTier 节点、Central 网络和本地控制器的 Windows 桌面应用。</p>
  <p>
    <a href="https://github.com/JunyuZhan/zt-manager/actions/workflows/ci.yml"><img src="https://github.com/JunyuZhan/zt-manager/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-0b8f87.svg" alt="MIT License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-3978c5.svg" alt="Windows 10 和 11">
    <img src="https://img.shields.io/badge/Tauri-v2-303846.svg" alt="Tauri 2">
  </p>
  <p><a href="README.md">English</a></p>
</div>

ZeroTier Manager 是一款面向 Windows 10/11 的 ZeroTier 管理桌面应用，使用 Tauri 2、React、TypeScript 和 Rust 构建。它可以管理本机 ZeroTier One、ZeroTier Central 网络和本地控制器。本项目独立维护，与 ZeroTier, Inc. 无隶属或背书关系。

## 功能

- **本机节点：** 查看节点、服务和网络状态；启动、停止或重启服务；加入或离开网络；运行连通性诊断。
- **ZeroTier Central：** 登录 ZeroTier 账号或使用 API Token；管理网络、成员、授权状态和成员设置；支持 Legacy 与 New Central API。
- **本地控制器：** 通过本机 ZeroTier API 查看和编辑控制器网络、成员及网络配置。

## 下载

目前尚未发布安装包。正式版本发布后，可从本仓库的 [Releases](https://github.com/JunyuZhan/zt-manager/releases) 页面下载。

## 从源码构建

### 环境要求

- Windows 10 或 Windows 11
- Node.js 20.19+ 或 22.12+，以及 npm
- Rust stable 工具链和 MSVC target
- Microsoft C++ Build Tools
- WebView2 Runtime

### 开发模式

在仓库根目录运行：

```powershell
npm ci
npm run tauri -- dev
```

### 构建与测试

```powershell
# 前端类型检查和生产构建
npm run build

# Rust 测试
cargo test --workspace

# 构建桌面安装包
npm run tauri -- build
```

桌面安装包位于 `src-tauri/target/release/bundle/`。

## 配置

Central 登录、API Token、API 模式、组织 ID 和可选的自定义 API 地址可在应用的「设置」页面配置。默认 API 地址如下：

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

## 隐私与安全

- 登录密码仅用于向 ZeroTier 登录服务验证，不会保存。登录会话和 API Token 使用 Windows DPAPI 加密后存储。
- 请求会发送到配置的 ZeroTier Central、登录服务或本机 ZeroTier API。自定义 API 地址只应填写可信服务。
- 不要提交密码、API Token、`.env` 文件、日志或包含私有网络信息的截图。
- 请按[安全政策](SECURITY.md)私下报告漏洞，不要在公开 Issue 中披露可利用细节。

## 参与贡献

欢迎提交 Bug 报告、功能建议和代码改进。提交前请阅读[贡献指南](CONTRIBUTING.zh-CN.md)。

## 许可证

本项目基于 [MIT License](LICENSE) 发布。第三方依赖和资源仍受其各自许可证约束。
