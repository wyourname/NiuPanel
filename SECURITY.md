# Security policy

## Supported versions

Security fixes are applied to the latest released version and the current default branch. Older releases may be asked to upgrade before receiving a fix.

## Reporting a vulnerability

Please do not open a public issue for vulnerabilities that could expose credentials, bypass authorization, escape a plugin sandbox, execute arbitrary commands, or access another user's data.

Report security issues privately through GitHub Security Advisories for this repository. Include:

- affected version or commit;
- deployment and platform details;
- reproduction steps or a proof of concept;
- expected impact;
- any suggested mitigation.

Maintainers will acknowledge a complete report as soon as practical, coordinate remediation, and credit reporters who want public attribution.

## Deployment responsibilities

- Set a unique `SESSION_KEY`; never deploy the example value.
- Keep `data/`, `.env`, signing keys, API keys, bot tokens, and backups outside version control.
- Require signed plugins in production and review trusted signing keys.
- Restrict MCP hosts, CORS origins, terminal access, and exposed ports to trusted networks.
- Treat imported scripts and third-party plugins as untrusted code.
