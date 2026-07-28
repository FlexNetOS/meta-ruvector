#!/usr/bin/env bash
set -euo pipefail

TASK_ID="CAP-INV011-001_RUVECTOR_SHAKE256_SQL"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd -- "$CRATE_DIR/../.." && pwd)"
META_ROOT="/home/flexnetos/meta"
LIFEOS_ROOT="$META_ROOT/src/lifeos"
YAZELIX_ROOT="$META_ROOT/src/yazelix"
PROFILE="/home/flexnetos/.nix-profile"
PROFILE_RESOLVED="$(readlink -f "$PROFILE")"
PROFILE_BIN="$PROFILE/bin"
EXTENSION_DIR="$PROFILE/share/postgresql/extension"
CONTROL_FILE="$EXTENSION_DIR/ruvector.control"
BASE_SQL="$EXTENSION_DIR/ruvector--0.3.1.sql"
UPGRADE_SQL="$EXTENSION_DIR/ruvector--0.3.0--0.3.1.sql"
SOURCE_MARKER="$EXTENSION_DIR/.envctl-ruvector-source"
PROFILE_LIBRARY="$PROFILE/lib/ruvector.so"
PSQL="$PROFILE_BIN/psql"
CARGO="$PROFILE_BIN/cargo"
JQ="$PROFILE_BIN/jq"
NIX="$PROFILE_BIN/nix"
NIX_STORE="$PROFILE_BIN/nix-store"
RG="$PROFILE_BIN/rg"
PG_SOCKET="$META_ROOT/var/run/postgresql"
PG_PORT="5432"
TEMP_ROOT="$META_ROOT/var/tmp"
ARTIFACT="$LIFEOS_ROOT/planning-spine-v0/envctl-db-nu-plugin-migration-automation-package/execution-framework/migration-artifacts/inv-011/ruvector-shake256-sql.json"
EXPECTED_EMPTY="46b9dd2b0ba88d13233b3feb743eeb243fcd52ea62b81b82b50c27646ed5762f"
EXPECTED_ABC="483366601360a8771c6863080cc4114d8db44530f8f1e1ee4f94ea37e78b5739"

log() {
  printf '[%s] %s\n' "$TASK_ID" "$*"
}

fail() {
  printf '[%s] ERROR: %s\n' "$TASK_ID" "$*" >&2
  exit 1
}

for executable in "$PSQL" "$CARGO" "$JQ" "$NIX" "$NIX_STORE" "$RG"; do
  [[ -x "$executable" ]] || fail "required profile executable is missing: $executable"
done
for payload in \
  "$CONTROL_FILE" \
  "$BASE_SQL" \
  "$UPGRADE_SQL" \
  "$SOURCE_MARKER" \
  "$PROFILE_LIBRARY"; do
  [[ -s "$payload" ]] || fail "required profile payload is missing: $payload"
done
[[ -S "$PG_SOCKET/.s.PGSQL.$PG_PORT" ]] \
  || fail "live PostgreSQL socket is missing: $PG_SOCKET/.s.PGSQL.$PG_PORT"

/usr/bin/mkdir -p -- "$TEMP_ROOT" "$(dirname -- "$ARTIFACT")"
/usr/bin/chmod 0700 "$TEMP_ROOT"
TEST_DIR="$(/usr/bin/mktemp -d "$TEMP_ROOT/ruvector-shake256-verify.XXXXXX")"

cleanup() {
  case "$TEST_DIR" in
    "$TEMP_ROOT"/ruvector-shake256-verify.*)
      /usr/bin/rm -rf -- "$TEST_DIR"
      ;;
    *)
      fail "refusing to remove unexpected temporary path: $TEST_DIR"
      ;;
  esac
}
trap cleanup EXIT

log "checking the source binding and focused Rust NIST vectors"
"$RG" -qF 'rvf_crypto::shake256_256(&input).to_vec()' "$CRATE_DIR/src/crypto.rs" \
  || fail "PostgreSQL wrapper does not call rvf_crypto::shake256_256"
"$RG" -qF 'mod crypto;' "$CRATE_DIR/src/lib.rs" \
  || fail "PostgreSQL extension does not export the crypto module"
"$RG" -qF 'rvf-crypto' "$CRATE_DIR/Cargo.toml" \
  || fail "PostgreSQL extension lacks the rvf-crypto dependency"

/usr/bin/mkdir -p -- "$TEST_DIR/tests"
/usr/bin/printf '%s\n' \
  '[package]' \
  'name = "ruvector-shake256-nist-vectors"' \
  'version = "0.0.0"' \
  'edition = "2021"' \
  '' \
  '[dependencies]' \
  "rvf-crypto = { path = \"$REPO_ROOT/crates/rvf/rvf-crypto\", default-features = false, features = [\"std\"] }" \
  '' \
  '[workspace]' \
  >"$TEST_DIR/Cargo.toml"
/usr/bin/printf '%s\n' \
  'use rvf_crypto::shake256_256;' \
  '' \
  '#[test]' \
  'fn nist_vectors() {' \
  '    assert_eq!(' \
  '        hex(&shake256_256(b"")),' \
  '        "46b9dd2b0ba88d13233b3feb743eeb243fcd52ea62b81b82b50c27646ed5762f"' \
  '    );' \
  '    assert_eq!(' \
  '        hex(&shake256_256(b"abc")),' \
  '        "483366601360a8771c6863080cc4114d8db44530f8f1e1ee4f94ea37e78b5739"' \
  '    );' \
  '}' \
  '' \
  'fn hex(bytes: &[u8]) -> String {' \
  '    bytes.iter().map(|byte| format!("{byte:02x}")).collect()' \
  '}' \
  >"$TEST_DIR/tests/nist.rs"
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR="$TEST_DIR/target" \
  "$CARGO" test --offline \
  --manifest-path "$TEST_DIR/Cargo.toml" \
  --test nist \
  -- --nocapture

log "proving the current source builds to the active Nix-managed extension"
BUILD_CLOSURE="$(
  "$NIX" build \
    --no-link \
    --print-out-paths \
    "path:$YAZELIX_ROOT#lifeos_foundation_yzx"
)"
[[ "$BUILD_CLOSURE" == /nix/store/*-lifeos-foundation-yzx ]] \
  || fail "unexpected Nix build result: $BUILD_CLOSURE"
BUILD_LIBRARY_REAL="$(readlink -f "$BUILD_CLOSURE/lib/ruvector.so")"
PROFILE_LIBRARY_REAL="$(readlink -f "$PROFILE_LIBRARY")"
[[ "$BUILD_LIBRARY_REAL" == "$PROFILE_LIBRARY_REAL" ]] \
  || fail "active profile library differs from the current Nix build"
EXTENSION_PACKAGE="${PROFILE_LIBRARY_REAL%/lib/ruvector.so}"
EXTENSION_DERIVER="$("$NIX_STORE" --query --deriver "$EXTENSION_PACKAGE")"
[[ "$EXTENSION_DERIVER" == /nix/store/*.drv ]] \
  || fail "extension package lacks a Nix derivation receipt"

"$RG" -q "^default_version = '0\\.3\\.1'$" "$CONTROL_FILE" \
  || fail "profile control file does not default to extension version 0.3.1"
"$RG" -q "^relocatable = true$" "$CONTROL_FILE" \
  || fail "profile control file is not relocatable"
if "$RG" -q 'JsonB\(serde_json' "$BASE_SQL"; then
  fail "profile base SQL contains an unevaluated Rust expression"
fi
[[ "$("$RG" -c '^CREATE FUNCTION ruvector_shake256_256\(input bytea\)$' "$BASE_SQL")" == "1" ]] \
  || fail "profile base SQL does not contain exactly one SHAKE256-256 function"
"$RG" -qF "AS 'MODULE_PATHNAME', 'ruvector_shake256_256_wrapper'" "$BASE_SQL" \
  || fail "profile base SQL lacks the native SHAKE256-256 wrapper symbol"
"$RG" -qF "AS 'MODULE_PATHNAME', 'ruvector_shake256_256_wrapper'" "$UPGRADE_SQL" \
  || fail "profile upgrade SQL lacks the native SHAKE256-256 wrapper symbol"
for sql_payload in "$BASE_SQL" "$UPGRADE_SQL"; do
  "$RG" -qF "REVOKE EXECUTE ON FUNCTION ruvector_shake256_256(bytea) FROM PUBLIC;" "$sql_payload" \
    || fail "$sql_payload does not revoke PUBLIC EXECUTE on SHAKE256-256"
  "$RG" -qF "TO lifeos_migrator, lifeos_envctl, lifeos_runtime;" "$sql_payload" \
    || fail "$sql_payload does not grant SHAKE256-256 only to the intended roles"
done

inspect_database() {
  local database="$1"
  local schema="$2"
  "$PSQL" -X -w -qAt -v ON_ERROR_STOP=1 \
    -h "$PG_SOCKET" -p "$PG_PORT" -d "$database" <<SQL
WITH function_row AS (
  SELECT procedure.*,
         namespace.nspname,
         owner.rolname AS owner_name,
         language.lanname
  FROM pg_proc AS procedure
  JOIN pg_namespace AS namespace ON namespace.oid = procedure.pronamespace
  JOIN pg_roles AS owner ON owner.oid = procedure.proowner
  JOIN pg_language AS language ON language.oid = procedure.prolang
  WHERE procedure.oid = '$schema.ruvector_shake256_256(bytea)'::regprocedure
),
extension_row AS (
  SELECT extension.oid,
         extension.extversion,
         namespace.nspname AS extension_schema
  FROM pg_extension AS extension
  JOIN pg_namespace AS namespace ON namespace.oid = extension.extnamespace
  WHERE extension.extname = 'ruvector'
),
execute_grantees AS (
  SELECT coalesce(
           json_agg(grantee ORDER BY grantee),
           '[]'::json
         ) AS values
  FROM (
    SELECT DISTINCT coalesce(role.rolname, 'PUBLIC') AS grantee
    FROM function_row AS function
    CROSS JOIN LATERAL aclexplode(
      coalesce(function.proacl, acldefault('f', function.proowner))
    ) AS acl
    LEFT JOIN pg_roles AS role ON role.oid = acl.grantee
    WHERE acl.privilege_type = 'EXECUTE'
  ) AS grants
)
SELECT json_build_object(
  'database', current_database(),
  'extversion', extension.extversion,
  'extension_schema', extension.extension_schema,
  'function', function.oid::regprocedure::text,
  'owner', function.owner_name,
  'language', function.lanname,
  'library', function.probin,
  'symbol', function.prosrc,
  'immutable', function.provolatile = 'i',
  'strict', function.proisstrict,
  'parallel_safe', function.proparallel = 's',
  'security_definer', function.prosecdef,
  'extension_member', EXISTS (
    SELECT 1
    FROM pg_depend AS dependency
    WHERE dependency.classid = 'pg_proc'::regclass
      AND dependency.objid = function.oid
      AND dependency.refclassid = 'pg_extension'::regclass
      AND dependency.refobjid = extension.oid
      AND dependency.deptype = 'e'
  ),
  'schema_usage', json_build_object(
    'lifeos_migrator',
      has_schema_privilege('lifeos_migrator', '$schema', 'USAGE'),
    'lifeos_envctl',
      has_schema_privilege('lifeos_envctl', '$schema', 'USAGE'),
    'lifeos_runtime',
      has_schema_privilege('lifeos_runtime', '$schema', 'USAGE')
  ),
  'execute_grantees', execute_grantees.values,
  'nist_empty', encode(
    $schema.ruvector_shake256_256(decode('', 'hex')),
    'hex'
  ),
  'nist_abc', encode(
    $schema.ruvector_shake256_256(convert_to('abc', 'UTF8')),
    'hex'
  ),
  'output_bytes', octet_length(
    $schema.ruvector_shake256_256(convert_to('abc', 'UTF8'))
  ),
  'strict_null', $schema.ruvector_shake256_256(NULL::bytea) IS NULL
)
FROM function_row AS function
CROSS JOIN extension_row AS extension
CROSS JOIN execute_grantees;
SQL
}

assert_database() {
  local report="$1"
  local database="$2"
  local schema="$3"
  "$JQ" -e \
    --arg database "$database" \
    --arg schema "$schema" \
    --arg empty "$EXPECTED_EMPTY" \
    --arg abc "$EXPECTED_ABC" \
    '(.database == $database)
     and (.extversion == "0.3.1")
     and (.extension_schema == $schema)
     and (.owner == "lifeos_migrator")
     and (.language == "c")
     and (.library == "$libdir/ruvector")
     and (.symbol == "ruvector_shake256_256_wrapper")
     and .immutable
     and .strict
     and .parallel_safe
     and (.security_definer | not)
     and .extension_member
     and .schema_usage.lifeos_migrator
     and .schema_usage.lifeos_envctl
     and .schema_usage.lifeos_runtime
     and (.execute_grantees == [
       "lifeos_envctl",
       "lifeos_migrator",
       "lifeos_runtime"
     ])
     and (.nist_empty == $empty)
     and (.nist_abc == $abc)
     and (.output_bytes == 32)
     and .strict_null' \
    <<<"$report" >/dev/null \
    || fail "$database/$schema native extension or ACL assertion failed"
}

assert_role_access() {
  local database="$1"
  local schema="$2"
  local role
  local output
  for role in lifeos_migrator lifeos_envctl lifeos_runtime; do
    output="$(
      "$PSQL" -X -w -qAt -v ON_ERROR_STOP=1 \
        -h "$PG_SOCKET" -p "$PG_PORT" -d "$database" \
        -c "SET ROLE $role; SELECT encode($schema.ruvector_shake256_256(convert_to('abc', 'UTF8')), 'hex'); RESET ROLE;"
    )"
    [[ "$output" == "$EXPECTED_ABC" ]] \
      || fail "$database role $role did not receive the exact SHAKE256-256 vector"
  done
}

log "asserting native extension identity and least privilege in both databases"
RUVECTOR_REPORT="$(inspect_database ruvector public)"
LIFEOS_REPORT="$(inspect_database lifeos extensions)"
assert_database "$RUVECTOR_REPORT" ruvector public
assert_database "$LIFEOS_REPORT" lifeos extensions
assert_role_access ruvector public
assert_role_access lifeos extensions

if "$PSQL" -X -w -qAt -v ON_ERROR_STOP=1 \
  -h "$PG_SOCKET" -p "$PG_PORT" -d ruvector \
  >"$TEST_DIR/unauthorized.stdout" 2>"$TEST_DIR/unauthorized.stderr" \
  -c "SET ROLE lifeos_cow_runtime_test_role; SELECT public.ruvector_shake256_256(convert_to('abc', 'UTF8'));"
then
  fail "unauthorized role unexpectedly executed ruvector_shake256_256"
fi

SOURCE_REVISION="$(git -C "$REPO_ROOT" rev-parse HEAD)"
if [[ -n "$(git -C "$REPO_ROOT" status --porcelain)" ]]; then
  SOURCE_DIRTY=true
else
  SOURCE_DIRTY=false
fi
SOURCE_TREE_DIGEST="$(
  /usr/bin/sha256sum \
    "$CRATE_DIR/Cargo.toml" \
    "$CRATE_DIR/Cargo.lock" \
    "$CRATE_DIR/ruvector.control" \
    "$CRATE_DIR/src/lib.rs" \
    "$CRATE_DIR/src/crypto.rs" \
    "$CRATE_DIR/src/bin/pgrx_embed.rs" \
    "$CRATE_DIR/sql/ruvector--0.3.0--0.3.1.sql" \
    "$SCRIPT_DIR/verify-ruvector-shake256-sql.sh" \
    | /usr/bin/sha256sum \
    | /usr/bin/cut -d' ' -f1
)"
LIBRARY_DIGEST="$(/usr/bin/sha256sum "$PROFILE_LIBRARY_REAL" | /usr/bin/cut -d' ' -f1)"
CONTROL_DIGEST="$(/usr/bin/sha256sum "$CONTROL_FILE" | /usr/bin/cut -d' ' -f1)"
BASE_SQL_DIGEST="$(/usr/bin/sha256sum "$BASE_SQL" | /usr/bin/cut -d' ' -f1)"
UPGRADE_SQL_DIGEST="$(/usr/bin/sha256sum "$UPGRADE_SQL" | /usr/bin/cut -d' ' -f1)"
SOURCE_MARKER_VALUE="$(/usr/bin/tr -d '\n' <"$SOURCE_MARKER")"
VERIFIED_AT="$(/usr/bin/date -u +%Y-%m-%dT%H:%M:%SZ)"
ARTIFACT_TMP="$ARTIFACT.$$.new"

"$JQ" -n \
  --arg schema "lifeos.migration-artifact.ruvector-shake256-sql.v2" \
  --arg task_id "$TASK_ID" \
  --arg verified_at "$VERIFIED_AT" \
  --arg source_revision "$SOURCE_REVISION" \
  --argjson source_dirty "$SOURCE_DIRTY" \
  --arg source_tree_digest "$SOURCE_TREE_DIGEST" \
  --arg profile_resolved "$PROFILE_RESOLVED" \
  --arg build_closure "$BUILD_CLOSURE" \
  --arg extension_package "$EXTENSION_PACKAGE" \
  --arg extension_deriver "$EXTENSION_DERIVER" \
  --arg source_marker "$SOURCE_MARKER_VALUE" \
  --arg library "$PROFILE_LIBRARY_REAL" \
  --arg library_sha256 "$LIBRARY_DIGEST" \
  --arg control_sha256 "$CONTROL_DIGEST" \
  --arg base_sql_sha256 "$BASE_SQL_DIGEST" \
  --arg upgrade_sql_sha256 "$UPGRADE_SQL_DIGEST" \
  --arg rustc "$("$PROFILE_BIN/rustc" --version)" \
  --arg cargo "$("$CARGO" --version)" \
  --arg postgresql "$("$PSQL" --version)" \
  --arg nix "$("$NIX" --version)" \
  --argjson ruvector "$RUVECTOR_REPORT" \
  --argjson lifeos "$LIFEOS_REPORT" \
  '{
    schema: $schema,
    task_id: $task_id,
    status: "passed",
    verified_at: $verified_at,
    source: {
      revision: $source_revision,
      dirty: $source_dirty,
      target_files_sha256: $source_tree_digest
    },
    nix: {
      profile_resolved: $profile_resolved,
      build_closure: $build_closure,
      extension_package: $extension_package,
      extension_deriver: $extension_deriver,
      source_marker: $source_marker,
      direct_store_write: false
    },
    installation: {
      library: $library,
      library_sha256: $library_sha256,
      control_sha256: $control_sha256,
      base_sql_sha256: $base_sql_sha256,
      upgrade_sql_sha256: $upgrade_sql_sha256
    },
    toolchain: {
      rustc: $rustc,
      cargo: $cargo,
      postgresql: $postgresql,
      nix: $nix
    },
    databases: {
      ruvector: $ruvector,
      lifeos: $lifeos
    },
    commands: [
      "cargo test --offline --manifest-path <temporary-rvf-crypto-harness>/Cargo.toml --test nist",
      "nix build --no-link --print-out-paths path:/home/flexnetos/meta/src/yazelix#lifeos_foundation_yzx",
      "psql native extension identity, NIST vectors, extension membership, and ACL assertions in ruvector and lifeos"
    ]
  }' >"$ARTIFACT_TMP"
/usr/bin/chmod 0644 "$ARTIFACT_TMP"
/usr/bin/mv -f -- "$ARTIFACT_TMP" "$ARTIFACT"

log "PASS library_sha256=$LIBRARY_DIGEST"
log "PASS ruvector=$(printf '%s' "$RUVECTOR_REPORT" | "$JQ" -r .function)"
log "PASS lifeos=$(printf '%s' "$LIFEOS_REPORT" | "$JQ" -r .function)"
log "PASS artifact=$ARTIFACT"
