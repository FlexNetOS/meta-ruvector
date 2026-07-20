#!/usr/bin/env nu

const active_policy_workflow = ".github/workflows/automation_policy.yml"

def yaml_files [] {
  glob ".github/**/*.{yml,yaml}" | sort
}

def policy_files [] {
  glob ".github/**/*"
  | where {|file| ($file | path type) == "file" }
  | sort
}

def matching_lines [files: list<path>, pattern: string, rule: string] {
  $files
  | each {|file|
      open $file --raw
      | lines
      | enumerate
      | where {|row|
          let text = $row.item | str trim
          (not ($text | str starts-with "#")) and ($row.item =~ $pattern)
        }
      | each {|row|
          {
            file: ($file | path relative-to $env.PWD)
            line: ($row.index + 1)
            rule: $rule
            text: ($row.item | str trim)
          }
        }
    }
  | flatten
}

let files = yaml_files
let scanned_files = policy_files
let remote_cache_pattern = "(?i)(actions/cache|Swatinem/rust-cache|magic-nix-cache|cachix|type=gha|sccache|ccache)"
let cache_key_pattern = "(?i)^\\s*(cache|cache-dependency-path|cache-from|cache-to)\\s*:"
let rust_wrapper_pattern = "(?i)^\\s*(RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER)\\s*:"
let forbidden_shell_pattern = "(?i)(shell\\s*:\\s*(bash|sh|zsh|pwsh|powershell|cmd)|(^|\\s)(bash|sh|zsh|pwsh|powershell|cmd)(\\s|$)|\\.(bash|sh|zsh)\\b)"

mut violations = []
$violations = $violations | append (matching_lines $scanned_files $remote_cache_pattern "remote or non-Kache cache")
$violations = $violations | append (matching_lines $scanned_files $cache_key_pattern "workflow cache input")
$violations = $violations | append (
  matching_lines $files $rust_wrapper_pattern "non-Kache Rust compiler wrapper"
  | where {|row| not ($row.text | str downcase | str contains "kache") }
)

let active_workflows = glob ".github/workflows/*.{yml,yaml}" | each {|file| $file | path relative-to $env.PWD }
let workflows_without_nushell = $active_workflows | where {|file|
  let workflow = open $file --raw
  not ($workflow | str contains "shell: nu {0}")
}
$violations = $violations | append ($workflows_without_nushell | each {|file| {
  file: $file
  line: 1
  rule: "workflow has not completed the Nushell port"
  text: "defaults.run.shell must be nu {0} before workflow run steps are active"
} })

if ($active_policy_workflow | path exists) {
  let active_text = open $active_policy_workflow --raw
  if not ($active_text | str contains "shell: nu {0}") {
    $violations = $violations | append {
      file: $active_policy_workflow
      line: 1
      rule: "active policy workflow has no Nushell default"
      text: "defaults.run.shell must be nu {0}"
    }
  }
  let active_workflow_paths = $active_workflows | each {|file| $file | path expand }
  $violations = $violations | append (matching_lines $active_workflow_paths $forbidden_shell_pattern "automatic shell is not Nushell")
} else {
  $violations = $violations | append {
    file: $active_policy_workflow
    line: 1
    rule: "active policy workflow is missing"
    text: "repository policy is fail-closed"
  }
}

if ($violations | is-not-empty) {
  print "Kache-only / Nushell-only automation policy violations:"
  $violations | sort-by file line | table --expand | print
  exit 1
}

print $"automation policy: PASS — (($scanned_files | length)) automation files contain no non-Kache cache directives; (($active_workflows | length)) active workflows use Nushell defaults"
