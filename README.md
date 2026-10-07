# ZeroTier Manager

**ZeroTier Manager** is a Windows desktop application for managing a local ZeroTier One installation, ZeroTier Central networks, and controller networks. It is built with Tauri 2, React, TypeScript, and Rust.

ZeroTier Manager is an independent, community-maintained project and is not affiliated with or endorsed by ZeroTier, Inc.

## Features

- View the local ZeroTier service, node status, and joined networks.
- Start, stop, or restart the local ZeroTier service; join or leave a network.
- Manage networks and members through Legacy Central or New Central.
- Inspect and edit networks hosted by a local controller.
- Run network diagnostics and configure Central API access.

## Platform and prerequisites

The current implementation targets **Windows 10/11**. It uses Windows DPAPI and Windows service management APIs; other platforms are not supported yet.

To build and run from source, install:

- Node.js 20.19+ (or 22.12+) and npm.
- Rust stable with the MSVC toolchain.
- Microsoft C++ Build Tools and the WebView2 Runtime.
- ZeroTier One, if you want to manage a local node or controller.

## Build and run

```powershell
npm ci
npm run tauri -- dev
```

Create a production build:

```powershell
npm run tauri -- build
```

Frontend-only type checking and production bundling:

```powershell
npm run build
```

Run the Rust workspace tests:

```powershell
cargo test --workspace
```

Tauri bundles are written below `src-tauri/target/release/bundle/`.

## Configuration

Open **Settings** in the app to enter a ZeroTier Central API token, select a Central API mode, and optionally set an organization ID or custom API base URLs.

| Setting | Default |
| --- | --- |
| Legacy Central API | `https://api.zerotier.com/api/v1` |
| New Central API | `https://central.zerotier.com/api/v2` |
| Local ZeroTier API | `http://127.0.0.1:9993` |

The following environment variables can provide defaults when the corresponding saved setting is empty:

| Variable | Purpose |
| --- | --- |
| `ZTM_CENTRAL_TOKEN` | Central API token |
| `ZTM_CENTRAL_MODE` | `auto`, `legacy`, or `new` |
| `ZTM_CENTRAL_ORG_ID` | New Central organization ID |
| `ZTM_LEGACY_BASE` | Custom Legacy Central API base URL |
| `ZTM_NEW_BASE` | Custom New Central API base URL |

The local node token is read from the standard ZeroTier One installation. Reading or changing protected service files may prompt for Windows elevation.

## Privacy and credentials

Central API credentials are stored in the per-user application configuration directory and protected with Windows DPAPI. The local ZeroTier token cache is also DPAPI-protected. Network requests go to the configured ZeroTier Central endpoint or the local ZeroTier API; do not use custom endpoints unless you trust them.

Do not commit real API tokens, `.env` files, local configuration, or diagnostic data containing private network details. `.env` files and local development artifacts are excluded by `.gitignore`.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development and contribution guidelines.

## License

This project is licensed under the [MIT License](./LICENSE). Third-party dependencies and assets remain subject to their respective licenses.

---

## English summary

A Windows desktop manager for local ZeroTier One services and networks, ZeroTier Central, and local controllers. Install the prerequisites above, then run `npm ci` and `npm run tauri -- dev`. Contributions are welcome under the MIT license. The application is independent and is not affiliated with ZeroTier, Inc.
