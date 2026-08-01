# Security policy

Report vulnerabilities privately through GitHub Security Advisories for
`dinglebear-ai/rgotify`.

High-impact findings include:

- exposure of Gotify application tokens, OAuth keys, or bearer credentials;
- an authentication or scope-check bypass on the MCP HTTP surface;
- a destructive Gotify action that bypasses the explicit confirmation gate;
- command, URL, header, or request-body injection through MCP or CLI input;
- divergence that lets the CLI or MCP shim bypass `GotifyService` policy;
- release, installer, npm, plugin, or registry publication of unverified bytes.

Do not open a public issue containing live credentials or a working
credential-exfiltration, authorization-bypass, or publication-bypass proof.
