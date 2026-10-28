# Security reporting

Security fixes target the latest published 0.2.x release. Older releases are not maintained as separate security branches. See [compatibility](docs/compatibility.md) for supported Pi versions and the [changelog](CHANGELOG.md) for released fixes.

For a sensitive report, use **Report a vulnerability** on this repository's GitHub Security tab if private reporting is available. If that option is unavailable, open an issue requesting a private contact channel without disclosing the vulnerability details. This policy does not promise that private reporting is enabled or set a response deadline.

Do not include credentials, private prompts/history, agent configuration or raw diagnostics in public reports. After a private channel is established, provide affected versions, a minimal synthetic reproduction and the impact. Keep model requests, external hosts and resource cleanup within the authorization of the affected system's owner.

Local subprocess execution inherits its environment and is not an isolation boundary. Transport byte budgets are not RSS limits. The host owns executable trust, credentials, environment policy and remote resource cleanup; see [connections](docs/connections.md) and [native sessions](docs/native-sessions.md).
