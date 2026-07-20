# Disabled workflows

These workflows are retained as migration source but are not executable by GitHub Actions.
They were disabled because their automatic `run` steps have not yet been ported to Nushell.

A workflow may return to `.github/workflows/` only after it uses native Nushell for every
automatic command and passes `nu ci/gates/automation_policy.nu`. Remote or non-Kache cache
directives are forbidden even in this disabled source tree.
