# Security policy

## Reporting a vulnerability

Please report security issues privately through
[GitHub's private vulnerability reporting](https://github.com/Alssndr0/readme-style/security/advisories/new),
not in a public issue. You will get an answer within a week.

## Scope

readme-style reads the README and the project files next to it (manifests, workflows, license,
`.git/config`, logo names) and writes only the README it was asked to style, atomically. Styling makes
no network requests and runs no other programs. Only `readme-style suggest` uses the network: it sends
the README, the project metadata and (for diagrams) a two-level file listing to the model you
configured, over HTTPS unless you point it at a plain-HTTP local server. API keys are read from the
environment, never from configuration files, and never printed.

Reports about content readme-style writes into a README, such as HTML or links that could mislead a
reader, are in scope too.
