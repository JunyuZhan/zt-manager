# Contributing

Bug reports, feature requests, documentation improvements, and code changes are welcome. For the Chinese version, see [CONTRIBUTING.zh-CN.md](CONTRIBUTING.zh-CN.md).

## Before opening an issue

- Search existing issues for the same problem or request.
- For a substantial change, open an issue first to discuss the proposed behavior and scope.
- Report security vulnerabilities privately as described in [SECURITY.md](SECURITY.md), not in a public issue.

## Bug reports and feature requests

For a bug report, include the app version, Windows version, steps to reproduce, expected and actual behavior, and relevant redacted logs.

For a feature request, describe the problem, the expected user experience, and any relevant constraints.

Do not post passwords, API tokens, node or network IDs, private IP addresses, or other sensitive information. Redact logs and screenshots before sharing them.

## Development setup

The app targets Windows 10/11. Install the prerequisites listed in [README.md](README.md), then run these commands from the repository root:

```powershell
npm ci
npm run tauri -- dev
```

## Checks

Run the checks relevant to your change:

```powershell
# Frontend type-check and production build
npm run build

# Rust tests
cargo test --workspace

# Rust lint
cd src-tauri
cargo clippy --all-targets -- -D warnings
```

CI runs the frontend build, Rust tests, and Clippy for pushes and pull requests targeting `main`.

## Pull requests

- Keep the change focused; submit unrelated work separately.
- Explain the problem and the approach. Link related issues when applicable.
- Include relevant test results and screenshots for visual changes.
- Update the documentation when behavior, setup, or configuration changes.
- Add or update tests when practical.
- Do not include build output, local configuration, credentials, or private network data.

Follow the existing code style and use clear commit messages. By submitting a contribution, you agree to its distribution under this repository's [MIT License](LICENSE). No separate contributor license agreement is currently required.
