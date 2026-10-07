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
  <p><a href="README.zh-CN.md">简体中文</a></p>
</div>

ZeroTier Manager is a Windows 10/11 desktop app for managing a local ZeroTier One node, ZeroTier Central networks, and local controllers. It is built with Tauri 2, React, TypeScript, and Rust. The project is maintained independently and is not affiliated with or endorsed by ZeroTier, Inc.

## Features

- **Local node:** Inspect node, service, and network status; start, stop, or restart the service; join or leave networks; and run connectivity diagnostics.
- **ZeroTier Central:** Sign in with a ZeroTier account or use an API token; manage networks, members, authorization, and member settings; supports Legacy and New Central APIs.
- **Local controller:** View and edit controller networks, members, and network configuration through the local ZeroTier API.

## Downloads

There is no published installer yet. Once a release is available, downloads will be published on the repository's [Releases](https://github.com/JunyuZhan/zt-manager/releases) page.

## Build from source

### Requirements

- Windows 10 or Windows 11
- Node.js 20.19+ or 22.12+, with npm
- The stable Rust toolchain with the MSVC target
- Microsoft C++ Build Tools
- WebView2 Runtime

### Run in development

From the repository root:

```powershell
npm ci
npm run tauri -- dev
```

### Build and test

```powershell
# Type-check and build the frontend
npm run build

# Run Rust tests
cargo test --workspace

# Build desktop installers
npm run tauri -- build
```

Desktop bundles are written to `src-tauri/target/release/bundle/`.

## Configuration

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

## Privacy and security

- Account passwords are used only to authenticate with ZeroTier's login service and are not saved. Login sessions and API tokens are encrypted at rest with Windows DPAPI.
- Requests are sent to the configured ZeroTier Central or login service, or to the local ZeroTier API. Use only custom API endpoints you trust.
- Do not commit passwords, API tokens, `.env` files, logs, or screenshots containing private network details.
- Report vulnerabilities privately according to the [security policy](SECURITY.md); do not disclose exploitable details in a public issue.

## Contributing

Bug reports, feature suggestions, and code contributions are welcome. Please read the [contribution guide](CONTRIBUTING.md) before submitting a change.

## License

This project is licensed under the [MIT License](LICENSE). Third-party dependencies and assets remain subject to their respective licenses.
