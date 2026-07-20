# Architect plan

VERDICT: GO.

1. Remove all `actions/cache`, `Swatinem/rust-cache`, setup-action cache inputs, and `type=gha` directives.
2. Set local workflow `run:` steps to the profile Nushell.
3. Disable non-Nushell jobs until their invoked scripts are ported.
4. Add a fail-closed repository policy gate and do not re-enable Actions until it passes.
