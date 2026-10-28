# Security policy

jgrapht-rs is a library without network or file-system access of its own, but the importers in
`jgrapht-io` parse untrusted text and a malformed document must never cause a panic, a hang or
unbounded memory use.

## Reporting a vulnerability

Please report suspected vulnerabilities privately through GitHub:
**Security → Report a vulnerability** on the repository page. Do not open a public issue for them.
Include a minimal input or program that reproduces the problem and the version or commit you used.

You can expect an acknowledgement within a few days. Fixes are released as soon as they are ready
and credited to the reporter unless the reporter prefers otherwise.

## Supported versions

Until the first stable release only the latest commit on `master` is supported.
