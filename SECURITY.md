# Security Policy

中文版：[SECURITY.zh-CN.md](SECURITY.zh-CN.md)。

## Supported versions

Security fixes are currently considered for the latest version of the default branch. Older releases are not covered by a formal support policy.

## Report a vulnerability

Please do not report exploitable vulnerabilities in a public issue or pull request.

Use [GitHub's private vulnerability reporting form](https://github.com/JunyuZhan/zt-manager/security/advisories/new) to contact the maintainers privately. If private reporting is unavailable, contact the repository owner through a private contact method on their GitHub profile. Do not post vulnerability details in a public issue or pull request.

When reporting a vulnerability, include as much of the following as you can safely provide:

- The affected version, commit, or component.
- The security impact and conditions required to reproduce it.
- Clear reproduction steps or a proof of concept, if one can be shared privately.
- Any suggested mitigation or remediation.

Do not include real passwords, API tokens, node or network IDs, private IP addresses, or other users' data. If sensitive information is necessary to explain the issue, share it only through the private reporting channel.

## Security considerations

The app sends requests to ZeroTier Central, ZeroTier's login service, and the local ZeroTier API. A custom API URL changes where requests and credentials are sent; use one only if you trust its operator. Never commit credentials or include them in public logs and screenshots.
