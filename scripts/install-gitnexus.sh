#!/usr/bin/env bash
# Install GitNexus (https://github.com/abhigyanpatwari/GitNexus) for this
# repository — indexes the codebase into a local LadybugDB graph database
# and optionally registers the MCP server for any agent runtime that's on
# PATH (Claude Code, Codex, Cursor, OpenCode, …).
#
# GitNexus is the Phase 4 piece of the cross-repo self-learning roadmap:
# the brain (this repo, ruvector) needs structural awareness of its own
# 150+ crate workspace before it can reason about cross-repo refactors
# with weftos. The graph lives entirely on disk — no external service.
#
# Idempotent: safe to re-run. The CLI itself is staleness-aware (checks
# git HEAD against the indexed snapshot) and only re-walks changed files
# unless --force is passed.
#
# License note: GitNexus ships under PolyForm Noncommercial. This script
# only invokes the upstream CLI; no GitNexus code is vendored into this
# repo. If you have a commercial license arrangement with akonlabs.com,
# nothing here changes — it just makes the CLI available to your agents.

set -euo pipefail

GITNEXUS_VERSION="${GITNEXUS_VERSION:-latest}"
GITNEXUS_MODE="${GITNEXUS_MODE:-full}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# A peer checkout normally lives at <meta-root>/src/<repo>. Discover that
# control-plane root instead of baking a user-specific pre-Meta path into the
# generated launcher. Standalone clones fall back to XDG data ownership.
meta_root_candidate="$(cd "$REPO_ROOT/../.." 2>/dev/null && pwd || true)"
if [[ -n "${META_ROOT:-}" ]]; then
  GITNEXUS_META_ROOT="$META_ROOT"
elif [[ -f "$meta_root_candidate/.meta.yaml" ]]; then
  GITNEXUS_META_ROOT="$meta_root_candidate"
else
  GITNEXUS_META_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/meta"
fi

# Persistent CLI host for the bun path (native lbugjs.node is built once here and
# reused). Overridable so CI / alternate machines can relocate it.
GITNEXUS_HOME="${GITNEXUS_HOME:-$GITNEXUS_META_ROOT/var/lib/gitnexus-cli}"
GITNEXUS_LAUNCHER="${GITNEXUS_LAUNCHER:-$HOME/.local/bin/gitnexus}"

# All helpers route to stderr so callers can capture script output without
# mixing in informational chatter (matches scripts/attractor.sh convention).
log()  { printf '\033[1;34m[gitnexus]\033[0m %s\n' "$*" >&2; }
warn() { printf '\033[1;33m[gitnexus]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[1;31m[gitnexus]\033[0m %s\n' "$*" >&2; exit 1; }

rewrite_claude_mcp_config() {
  local config_path="${1:-$HOME/.claude.json}"
  [[ -f "$config_path" ]] || return 0
  GITNEXUS_LAUNCHER="$GITNEXUS_LAUNCHER" node - "$config_path" <<'NODE'
const fs = require("fs");
const configPath = process.argv[2];
const launcher = process.env.GITNEXUS_LAUNCHER;
if (!launcher) {
  throw new Error("GITNEXUS_LAUNCHER was not exported to the Claude MCP rewrite");
}
const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
if (config.mcpServers && config.mcpServers.gitnexus) {
  config.mcpServers.gitnexus = { command: launcher, args: ["mcp"] };
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + "\n");
}
NODE
}

# Tests source this file to exercise the config rewrite without running package
# installation, indexing, or editor setup.
if [[ "${GITNEXUS_LIBRARY_ONLY:-0}" == "1" ]]; then
  if [[ "${BASH_SOURCE[0]}" != "$0" ]]; then
    return 0
  fi
  exit 0
fi

case "$GITNEXUS_MODE" in
  full|launcher) ;;
  *) fail "GITNEXUS_MODE must be 'full' or 'launcher' (got '$GITNEXUS_MODE')" ;;
esac

# ── Preflight ────────────────────────────────────────────────────────
# Node 20+ is the hard requirement (GitNexus package.json engines field);
# older Node fails with a cryptic syntax error inside the download. On the
# nix-owned toolchain `node` comes from the foundation toolbin (bash -lc).
command -v node >/dev/null 2>&1 || fail "node not on PATH. GitNexus needs Node.js 20+ (nix foundation toolbin — run under 'bash -lc')."
node_major="$(node -v 2>/dev/null | sed -E 's/^v([0-9]+).*/\1/' || echo 0)"
if [[ "$node_major" -lt 20 ]]; then
  fail "Node.js >= 20 required (detected v${node_major})."
fi

# ── Resolve a WORKING gitnexus invocation ────────────────────────────
# Mirrors gitnexus's own hooks/resolve-analyze-cmd cascade, but adds a bun
# path for the FlexNetOS nix toolchain, where `npx` is NOT on PATH and a bare
# `bunx` SKIPS package install-scripts — so LadybugDB's native lbugjs.node
# never builds and every command dies with "native binary missing".
#
#   A. `gitnexus` already on PATH  → use it (launcher from a prior run, etc.)
#   B. `npx` present               → npx -y gitnexus@VER   (upstream default)
#   C. `bun` present               → build a persistent install at
#      GITNEXUS_HOME with trustedDependencies + `bun pm trust --all` (runs the
#      15 native postinstalls), drop a launcher on PATH, and invoke via node.
#
# GN is an array so callers splat it: "${GN[@]}" analyze …
declare -a GN=()
if gitnexus_path="$(command -v gitnexus 2>/dev/null)" &&
  "$gitnexus_path" --version >/dev/null 2>&1; then
  GN=("$gitnexus_path")
  log "using healthy gitnexus already on PATH ($gitnexus_path)"
elif [[ -n "${gitnexus_path:-}" ]]; then
  warn "ignoring broken gitnexus frontdoor at $gitnexus_path; regenerating it from the owning installer"
fi

if [[ ${#GN[@]} -eq 0 ]] && command -v npx >/dev/null 2>&1; then
  GN=(npx -y "gitnexus@${GITNEXUS_VERSION}")
  log "using npx → gitnexus@${GITNEXUS_VERSION}"
elif [[ ${#GN[@]} -eq 0 ]] && command -v bun >/dev/null 2>&1; then
  log "npx absent; bootstrapping a bun-built gitnexus at ${GITNEXUS_HOME}"
  mkdir -p "$GITNEXUS_HOME"
  # A package.json with trustedDependencies is not sufficient on bun 1.3 (it
  # still blocks transitive native builds), so we install then `trust --all`.
  if [[ ! -f "$GITNEXUS_HOME/package.json" ]]; then
    printf '{\n  "name": "gitnexus-cli-host",\n  "private": true,\n  "trustedDependencies": ["@ladybugdb/core", "gitnexus", "tree-sitter", "node-tree-sitter"]\n}\n' > "$GITNEXUS_HOME/package.json"
  fi
  (
    cd "$GITNEXUS_HOME"
    bun add "gitnexus@${GITNEXUS_VERSION}" >&2
    # Bun exits non-zero when no blocked scripts remain. That is a healthy,
    # idempotent state as long as the installed CLI and native module validate
    # below, so do not turn a no-op trust pass into an installer failure.
    bun pm trust --all >&2 || log "bun reports no pending trust scripts; validating the existing native install"
  )
  gn_entry="$GITNEXUS_HOME/node_modules/gitnexus/dist/cli/index.js"
  node_abs="$(command -v node)"
  [[ -f "$gn_entry" ]] || fail "bun install did not produce $gn_entry"
  # Drop a self-contained launcher on PATH so the CLI, the MCP config, and
  # gitnexus's own search-augment hooks (cascade step A) all resolve without npx.
  mkdir -p "$(dirname "$GITNEXUS_LAUNCHER")"
  cat > "$GITNEXUS_LAUNCHER" <<EOF
#!/usr/bin/env bash
# GitNexus launcher — nix-owned node + persistent bun-built install (native
# lbugjs.node). Auto-generated by scripts/install-gitnexus.sh; regenerate there.
exec "$node_abs" "$gn_entry" "\$@"
EOF
  chmod +x "$GITNEXUS_LAUNCHER"
  GN=("$GITNEXUS_LAUNCHER")
  case ":$PATH:" in
    *":$(dirname "$GITNEXUS_LAUNCHER"):"*) : ;;
    *) warn "launcher at $GITNEXUS_LAUNCHER is not on PATH — add $(dirname "$GITNEXUS_LAUNCHER") to PATH for the hooks to find it" ;;
  esac
  log "gitnexus (bun) ready: $("${GN[@]}" --version 2>/dev/null)"
elif [[ ${#GN[@]} -eq 0 ]]; then
  fail "need one of: gitnexus on PATH, npx, or bun. On the nix foundation toolchain, run under 'bash -lc' so bun resolves."
fi

cd "$REPO_ROOT"

if [[ "$GITNEXUS_MODE" == "launcher" ]]; then
  log "launcher-only repair complete: ${GN[*]}"
  "${GN[@]}" --version
  exit 0
fi

# ── Index ────────────────────────────────────────────────────────────
# `analyze` is idempotent: it checks git HEAD against the indexed
# snapshot and only walks changed files. `--skip-agents-md` is critical:
# without it, GitNexus rewrites CLAUDE.md / AGENTS.md and would clobber
# the hand-curated rules at the top of CLAUDE.md (workspace exclusion
# rules, lint policy, etc.). We keep our context files authoritative.
log "indexing repo with gitnexus@${GITNEXUS_VERSION} (writes to .gitnexus/)"
log "  — using --skip-agents-md to preserve hand-curated CLAUDE.md"
if [[ "${GITNEXUS_FORCE:-0}" == "1" ]]; then
  log "  — GITNEXUS_FORCE=1 set; forcing full re-index"
  "${GN[@]}" analyze --skip-agents-md --force
else
  "${GN[@]}" analyze --skip-agents-md
fi

# ── MCP registration ────────────────────────────────────────────────
# `gitnexus setup` writes per-editor MCP configs (~/.cursor/mcp.json,
# ~/.config/opencode/config.json, etc.) and is editor-aware — it only
# touches the configs of editors that are actually installed. Safe to
# re-run.
#
# We invoke setup unconditionally (not gated on a specific CLI being
# present) because it auto-detects and skips missing editors.
log "registering MCP server for any installed agent runtime"
"${GN[@]}" setup || warn "gitnexus setup returned non-zero — MCP may need manual config; see https://github.com/abhigyanpatwari/GitNexus#mcp-setup"

# When gitnexus was resolved via the bun launcher, `gitnexus setup` writes MCP
# configs that invoke bare `npx gitnexus mcp` — broken on this toolchain. Rewrite
# the command it emitted to point at our launcher so MCP actually starts.
if [[ "${GN[0]}" == "$GITNEXUS_LAUNCHER" ]]; then
  log "rewriting setup's npx MCP command → launcher ($GITNEXUS_LAUNCHER)"
  # ~/.claude.json (JSON, user scope)
  if [[ -f "$HOME/.claude.json" ]]; then
    rewrite_claude_mcp_config || warn "could not rewrite ~/.claude.json gitnexus command"
  fi
  # ~/.codex/config.toml ([mcp_servers.gitnexus])
  if [[ -f "$HOME/.codex/config.toml" ]] && command -v python3 >/dev/null 2>&1; then
    GITNEXUS_LAUNCHER="$GITNEXUS_LAUNCHER" python3 - <<'PY' || warn "could not rewrite ~/.codex/config.toml gitnexus command"
import os,re
f=os.path.expanduser("~/.codex/config.toml"); s=open(f).read()
L=os.environ["GITNEXUS_LAUNCHER"]
def repl(m):
    b=re.sub(r'command\s*=\s*"[^"]*"', f'command = "{L}"', m.group(2))
    b=re.sub(r'args\s*=\s*\[[^\]]*\]', 'args = ["mcp"]', b)
    return m.group(1)+b
s=re.compile(r'(\[mcp_servers\.gitnexus\]\n)(.*?)(?=\n\[|\Z)', re.S).sub(repl, s)
open(f,"w").write(s)
PY
  fi
fi

# GitNexus 1.x always installs 6 helper SKILLs at .claude/skills/gitnexus/
# (verified in dist/cli/ai-context.js). They're auto-generated, repo-local,
# and useful at runtime — but not something we want tracked. The
# corresponding .gitignore entry lives in the repo's top-level .gitignore.

# ── Done ─────────────────────────────────────────────────────────────
log "GitNexus install complete."
log ""
log "  Index location:  $REPO_ROOT/.gitnexus/  (gitignored)"
log "  Registry:        ~/.gitnexus/registry.json"
log "  MCP server:      ${GN[*]} mcp"
log ""
log "Quick smoke-test (run from repo root):"
log "  ${GN[*]} status"
log "  ${GN[*]} list"
log ""
log "From within an MCP-aware agent: ask it to call the gitnexus 'context'"
log "or 'impact' tool with a symbol name to confirm it's wired up."
