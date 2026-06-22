---
name: source-command-hooks-setup
description: 'Setting Up Hooks (claude-flow + ICM + RTK + grit + vox)'
---

# /hooks/setup

Source: `.claude/commands/hooks/setup.md`

# Setting Up Hooks (claude-flow + ICM + RTK + grit + vox)

This repo already runs a full hook stack through `.claude/helpers/hook-handler.cjs`
(all 10 events: `PreToolUse`, `PostToolUse`, `UserPromptSubmit`, `SessionStart`,
`SessionEnd`, `Stop`, `PreCompact`, `SubagentStart`, `SubagentStop`, `Notification`).

This command documents how to (re)wire that stack **and** the four integrated tools —
**ICM** (memory), **RTK** (token-killer command rewriter), **grit** (symbol locks /
multi-agent coordination), and **vox** (spoken feedback). All commands below are the
tools' real hook surfaces — verify with `<tool> hook --help` / `<tool> --help`.

> ⚠️ `npx claude-flow init --hooks` **overwrites** `.claude/settings.json`. This repo's
> config is hand-tuned — back it up first (`cp .claude/settings.json{,.bak}`) or merge
> the blocks below by hand instead of re-initializing.

---

## Quick Start (one command per tool)

Each tool writes its own hook entries into the Claude Code settings:

```bash
# claude-flow (this repo's core handler)
npx claude-flow init --hooks            # ⚠️ rewrites settings.json — back up first

# ICM — Infinite Context Memory (persistent cross-session memory)
icm init --mode hook                    # hooks only; use --mode standard for cli+skill+hook

# RTK — Rust Token Killer (auto-rewrites verbose commands to compact equivalents)
rtk init                                # installs the PreToolUse rewrite hook

# grit — symbol locks + worktree coordination for multi-agent work
grit init                               # initialize grit in the repo (then wire hooks below)

# vox — spoken feedback (TTS)
vox init                                # Claude Code + Desktop integration (MCP + optional hooks)
```

After any change, verify the wiring:

```bash
icm doctor          # checks ICM hook binary paths in Claude Code settings
rtk hook-audit      # hook rewrite metrics (set RTK_HOOK_AUDIT=1 first)
grit status         # current lock state
vox config show     # active backend / voice
```

---

## ICM hooks — persistent memory

`icm hook` reads the Claude Code hook JSON from stdin and emits a hook response. Six
handlers map 1:1 to events:

| Event | Command | Purpose |
|-------|---------|---------|
| `SessionStart` | `icm hook start` | inject a wake-up pack of critical facts |
| `UserPromptSubmit` | `icm hook prompt` | inject recalled context at prompt start |
| `PreToolUse` | `icm hook pre` | auto-allow `icm` CLI commands (no permission prompt) |
| `PostToolUse` | `icm hook post` | auto-extract context every N tool calls |
| `PreCompact` | `icm hook compact` | extract memories from transcript before compression |
| `SessionEnd` | `icm hook end` | extract memories before the session closes |

```json
{
  "hooks": {
    "SessionStart":     [{ "hooks": [{ "type": "command", "command": "icm hook start",  "timeout": 8000 }] }],
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "icm hook prompt", "timeout": 8000 }] }],
    "PreToolUse":       [{ "hooks": [{ "type": "command", "command": "icm hook pre",    "timeout": 5000 }] }],
    "PostToolUse":      [{ "hooks": [{ "type": "command", "command": "icm hook post",   "timeout": 8000 }] }],
    "PreCompact":       [{ "hooks": [{ "type": "command", "command": "icm hook compact","timeout": 10000 }] }],
    "SessionEnd":       [{ "hooks": [{ "type": "command", "command": "icm hook end",    "timeout": 10000 }] }]
  }
}
```

Telemetry: `icm hook-log` (recent rows), `icm hook-stats` (latency percentiles, error rate).

---

## RTK hooks — command rewriting

RTK rewrites verbose commands to compact equivalents (60–90% token savings) at the
`PreToolUse` boundary. `rtk hook claude` reads the hook JSON from stdin and returns the
rewritten command. `rtk rewrite` is the single source of truth the hook engine uses.

| Event | Matcher | Command | Purpose |
|-------|---------|---------|---------|
| `PreToolUse` | `Bash` | `rtk hook claude` | rewrite `git status` → `rtk git status`, etc. (0-token overhead, transparent) |

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [{ "type": "command", "command": "rtk hook claude", "timeout": 5000 }]
      }
    ]
  }
}
```

- Dry-run a rewrite: `rtk hook check` (or `echo '{"tool_input":{"command":"git log"}}' | rtk hook check`)
- Audit metrics: `RTK_HOOK_AUDIT=1 rtk hook-audit`
- Other LLM CLIs: `rtk hook {cursor,gemini,copilot}`

---

## grit hooks — symbol locks & multi-agent coordination

grit has **no native `grit hook` subcommand** — wire its primitives into events. Use this
when multiple agents edit the same repo concurrently (claim symbols → heartbeat → release).

| Event | Command | Purpose |
|-------|---------|---------|
| `SessionStart` | `grit status` | surface current lock state on entry |
| `PostToolUse` (Write\|Edit\|MultiEdit) | `grit heartbeat` | refresh this agent's lock TTL while editing |
| `SubagentStop` | `grit release --all` | release locks a finished subagent held |
| `SessionEnd` | `grit gc` | garbage-collect expired locks |

```json
{
  "hooks": {
    "SessionStart": [{ "hooks": [{ "type": "command", "command": "grit status 2>/dev/null || true", "timeout": 4000 }] }],
    "PostToolUse": [
      {
        "matcher": "Write|Edit|MultiEdit",
        "hooks": [{ "type": "command", "command": "grit heartbeat 2>/dev/null || true", "timeout": 3000, "async": true }]
      }
    ],
    "SubagentStop": [{ "hooks": [{ "type": "command", "command": "grit release --all 2>/dev/null || true", "timeout": 4000 }] }],
    "SessionEnd":   [{ "hooks": [{ "type": "command", "command": "grit gc 2>/dev/null || true", "timeout": 5000 }] }]
  }
}
```

> All grit hook commands are guarded with `|| true` so a repo without an active grit
> session (no claims) never blocks a tool call. Explicit handoff uses `grit done`
> (merge worktree + release all locks) — keep that an agent action, not a hook.

---

## vox hooks — spoken feedback

vox's primary path is `vox init` (MCP integration via `vox serve`) + agents calling
`vox -b say "summary"` after significant tasks (see `CLAUDE.md`). It has no native hook
subcommand, but you can wire spoken **alerts** to events. vox reads the text as an
argument, so extract the payload with `jq`.

| Event | Command | Purpose |
|-------|---------|---------|
| `Notification` | speak the notification message | hear permission/idle prompts |
| `Stop` | speak a short completion cue | audible "turn done" |

```json
{
  "hooks": {
    "Notification": [
      {
        "hooks": [{
          "type": "command",
          "command": "jq -r '.message // empty' 2>/dev/null | { read -r m; [ -n \"$m\" ] && vox -b say \"$m\"; } || true",
          "timeout": 4000,
          "async": true
        }]
      }
    ],
    "Stop": [
      {
        "hooks": [{ "type": "command", "command": "vox -b say 'Turn complete' >/dev/null 2>&1 || true", "timeout": 4000, "async": true }]
      }
    ]
  }
}
```

- `-b` runs in the background (non-blocking). Current backend/voice: `vox config show`
  (this machine: `piper` / `en_US-lessac-medium`).
- Keep spoken hooks short — TTS for trivial ops is noise. Prefer agents calling
  `vox say` deliberately for real summaries over a chatty `Stop` hook.

---

## Combined example — all tools coexisting

Multiple hooks under one event run in sequence. This shows claude-flow's `hook-handler.cjs`
(this repo's core) running **alongside** ICM, RTK, grit, and vox. Keep RTK first on
`PreToolUse/Bash` so downstream hooks see the rewritten command.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "rtk hook claude", "timeout": 5000 },
          { "type": "command", "command": "icm hook pre", "timeout": 5000 },
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" pre-bash", "timeout": 5000 }
        ]
      },
      {
        "matcher": "Write|Edit|MultiEdit",
        "hooks": [{ "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" pre-edit", "timeout": 5000 }]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Write|Edit|MultiEdit",
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" post-edit", "timeout": 10000 },
          { "type": "command", "command": "grit heartbeat 2>/dev/null || true", "timeout": 3000, "async": true }
        ]
      },
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" post-bash", "timeout": 5000 },
          { "type": "command", "command": "icm hook post", "timeout": 8000 }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" route", "timeout": 10000 },
          { "type": "command", "command": "icm hook prompt", "timeout": 8000 }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" session-restore", "timeout": 15000 },
          { "type": "command", "command": "icm hook start", "timeout": 8000 },
          { "type": "command", "command": "grit status 2>/dev/null || true", "timeout": 4000 }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" session-end", "timeout": 10000 },
          { "type": "command", "command": "icm hook end", "timeout": 10000 },
          { "type": "command", "command": "grit gc 2>/dev/null || true", "timeout": 5000 }
        ]
      }
    ],
    "PreCompact": [
      {
        "matcher": "manual",
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" compact-manual" },
          { "type": "command", "command": "icm hook compact", "timeout": 10000 }
        ]
      },
      {
        "matcher": "auto",
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" compact-auto" },
          { "type": "command", "command": "icm hook compact", "timeout": 10000 }
        ]
      }
    ],
    "SubagentStop": [
      {
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" post-task", "timeout": 5000 },
          { "type": "command", "command": "grit release --all 2>/dev/null || true", "timeout": 4000 }
        ]
      }
    ],
    "Notification": [
      {
        "hooks": [
          { "type": "command", "command": "node \"${CLAUDE_PROJECT_DIR:-.}/.claude/helpers/hook-handler.cjs\" notify", "timeout": 3000 },
          { "type": "command", "command": "jq -r '.message // empty' 2>/dev/null | { read -r m; [ -n \"$m\" ] && vox -b say \"$m\"; } || true", "timeout": 4000, "async": true }
        ]
      }
    ]
  }
}
```

---

## Hook response format

Hooks may return JSON to influence the tool call:
- `continue` — proceed (true/false)
- `reason` — explanation for the decision
- `metadata` — additional context

```json
{ "continue": false, "reason": "Protected file - manual review required", "metadata": { "file": ".env.production" } }
```

## Verify & debug

```bash
# Validate settings JSON after editing
python3 -c "import json; json.load(open('.claude/settings.json')); print('valid')"

# Per-tool health
icm doctor && icm hook-stats        # ICM wiring + latency
rtk hook check                       # RTK rewrite dry-run
grit status                          # grit lock state
vox config show                      # vox backend/voice

# Live smoke-test a handler (reads stdin JSON)
echo '{}' | icm hook start
echo '{"tool_input":{"command":"git status"}}' | rtk hook claude
```

## Performance tips
- Keep hooks lightweight (< 100ms where possible); use `"async": true` for non-blocking ones (vox, grit heartbeat, brain inject).
- Guard external-tool hooks with `|| true` so a missing session never blocks a tool call.
- Order matters under one event: put RTK's rewrite first on `PreToolUse/Bash`.
- Don't speak (vox) on trivial ops — reserve it for real summaries.
