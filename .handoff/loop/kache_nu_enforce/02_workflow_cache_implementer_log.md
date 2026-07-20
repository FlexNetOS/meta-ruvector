# Workflow-cache implementer log

## Outcome

- Removed every remote/non-Kache cache directive from the retained workflow YAML and release documentation.
- Disabled every legacy non-Nushell workflow by moving it outside the active workflow directory.
- Added `.github/workflows/automation_policy.yml` as the sole active workflow.
- Added `ci/gates/automation_policy.nu`; it rejects non-Kache cache actions/inputs/wrappers and any unported active workflow.
- No Rust symbol was edited. Nothing was pushed or activated on the host.

## Exact workflow moves

- `.github/workflows/agentic-synth-ci.yml` → `.github/workflows_disabled/agentic-synth-ci.yml`
- `.github/workflows/artifact-policy.yml` → `.github/workflows_disabled/artifact-policy.yml`
- `.github/workflows/benchmarks.yml` → `.github/workflows_disabled/benchmarks.yml`
- `.github/workflows/build-attention.yml` → `.github/workflows_disabled/build-attention.yml`
- `.github/workflows/build-diskann.yml` → `.github/workflows_disabled/build-diskann.yml`
- `.github/workflows/build-gnn.yml` → `.github/workflows_disabled/build-gnn.yml`
- `.github/workflows/build-graph-node.yml` → `.github/workflows_disabled/build-graph-node.yml`
- `.github/workflows/build-graph-transformer.yml` → `.github/workflows_disabled/build-graph-transformer.yml`
- `.github/workflows/build-native.yml` → `.github/workflows_disabled/build-native.yml`
- `.github/workflows/build-router.yml` → `.github/workflows_disabled/build-router.yml`
- `.github/workflows/build-rvf-node.yml` → `.github/workflows_disabled/build-rvf-node.yml`
- `.github/workflows/build-tiny-dancer.yml` → `.github/workflows_disabled/build-tiny-dancer.yml`
- `.github/workflows/build-verified.yml` → `.github/workflows_disabled/build-verified.yml`
- `.github/workflows/ci.yml` → `.github/workflows_disabled/ci.yml`
- `.github/workflows/clippy-fmt.yml` → `.github/workflows_disabled/clippy-fmt.yml`
- `.github/workflows/copilot-setup-steps.yml` → `.github/workflows_disabled/copilot-setup-steps.yml`
- `.github/workflows/devcontainer-ghcr.yml` → `.github/workflows_disabled/devcontainer-ghcr.yml`
- `.github/workflows/docker-publish.yml` → `.github/workflows_disabled/docker-publish.yml`
- `.github/workflows/edge-net-models.yml` → `.github/workflows_disabled/edge-net-models.yml`
- `.github/workflows/emergent-time-ci.yml` → `.github/workflows_disabled/emergent-time-ci.yml`
- `.github/workflows/hailo-backend-audit.yml` → `.github/workflows_disabled/hailo-backend-audit.yml`
- `.github/workflows/hailo-release-artifacts.yml` → `.github/workflows_disabled/hailo-release-artifacts.yml`
- `.github/workflows/hooks-ci.yml` → `.github/workflows_disabled/hooks-ci.yml`
- `.github/workflows/mirror-rulake.yml` → `.github/workflows_disabled/mirror-rulake.yml`
- `.github/workflows/postgres-extension-ci.yml` → `.github/workflows_disabled/postgres-extension-ci.yml`
- `.github/workflows/publish-all.yml` → `.github/workflows_disabled/publish-all.yml`
- `.github/workflows/publish-rvagent-wasm.yml` → `.github/workflows_disabled/publish-rvagent-wasm.yml`
- `.github/workflows/regression-guard.yml` → `.github/workflows_disabled/regression-guard.yml`
- `.github/workflows/release-rvf-cli.yml` → `.github/workflows_disabled/release-rvf-cli.yml`
- `.github/workflows/release.yml` → `.github/workflows_disabled/release.yml`
- `.github/workflows/ruvector-npm-ci.yml` → `.github/workflows_disabled/ruvector-npm-ci.yml`
- `.github/workflows/ruvector-postgres-ci.yml` → `.github/workflows_disabled/ruvector-postgres-ci.yml`
- `.github/workflows/ruvector-publish.yml` → `.github/workflows_disabled/ruvector-publish.yml`
- `.github/workflows/ruvllm-benchmarks.yml` → `.github/workflows_disabled/ruvllm-benchmarks.yml`
- `.github/workflows/ruvllm-build.yml` → `.github/workflows_disabled/ruvllm-build.yml`
- `.github/workflows/ruvllm-esp32-firmware.yml` → `.github/workflows_disabled/ruvllm-esp32-firmware.yml`
- `.github/workflows/ruvllm-native.yml` → `.github/workflows_disabled/ruvllm-native.yml`
- `.github/workflows/ruvltra-tests.yml` → `.github/workflows_disabled/ruvltra-tests.yml`
- `.github/workflows/self-learning.yml` → `.github/workflows_disabled/self-learning.yml`
- `.github/workflows/sona-drift.yml` → `.github/workflows_disabled/sona-drift.yml`
- `.github/workflows/sona-napi.yml` → `.github/workflows_disabled/sona-napi.yml`
- `.github/workflows/supply-chain.yml` → `.github/workflows_disabled/supply-chain.yml`
- `.github/workflows/sync-rvf-examples.yml` → `.github/workflows_disabled/sync-rvf-examples.yml`
- `.github/workflows/thermorust-ci.yml` → `.github/workflows_disabled/thermorust-ci.yml`
- `.github/workflows/ui-ci.yml` → `.github/workflows_disabled/ui-ci.yml`
- `.github/workflows/validate-lockfile.yml` → `.github/workflows_disabled/validate-lockfile.yml`
- `.github/workflows/wasm-dedup-check.yml` → `.github/workflows_disabled/wasm-dedup-check.yml`

Additional files:

- `.github/workflows/RELEASE-FLOW.md`
- `.github/workflows/RELEASE.md`
- `.github/workflows_disabled/README.md`
- `.github/workflows/automation_policy.yml`
- `ci/gates/automation_policy.nu`

## Verification

- Policy gate: PASS.
- Negative probe containing a cache input: correctly rejected with exit 1.
- Active workflow actionlint: PASS.
- Disabled YAML syntax: PASS (47 files).
- Banned directive scan under `.github`: zero matches.
- `git diff --check`: PASS.

## Blocker

The disabled workflows must stay inactive until each automatic command is ported to native Nushell. A full actionlint of the disabled legacy source still reports its pre-existing non-Nushell shellcheck findings, obsolete action warnings, and unknown future runner labels; the only active workflow actionlints cleanly.
