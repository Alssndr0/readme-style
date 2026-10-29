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

A repository you clone is not trusted with your files. readme-style follows symbolic links only
where they cannot point it at a file it should not read, print in a diff, send to a model or
rewrite: a README that is a link must lead to a Markdown file (by its extension, or a plain
`README`) in its own folder or below, and a folder link must stay inside the repository it sits in.
Nothing inside a `.git` folder is ever read through a link.

The browser playground runs the same engine in the page. Project files you open are read there and
never uploaded. The preview shows the README's HTML reduced to what GitHub keeps, in a sandboxed
frame whose content security policy runs no script but the preview's own.

Reports about content readme-style writes into a README, such as HTML or links that could mislead a
reader, are in scope too.
