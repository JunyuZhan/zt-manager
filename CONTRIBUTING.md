# Contributing

Thanks for helping improve ZeroTier Manager.

## Development setup

Use Windows 10/11 and install the prerequisites listed in [README.md](./README.md). Then run:

```powershell
npm ci
npm run tauri -- dev
```

## Before opening a change

Run the checks relevant to your changes:

```powershell
npm run build
cargo test --workspace
```

Keep changes focused, describe user-visible behavior, and include or update tests when practical. Do not include credentials, `.env` files, personal configuration, build outputs, or private network details in commits.

## Reporting issues

Include the application version, Windows version, steps to reproduce, and relevant redacted error messages. Remove API tokens, node IDs, network IDs, IP addresses, and other private network information from logs and screenshots before sharing them.

By submitting a contribution, you agree that it may be distributed under this repository's MIT License.
