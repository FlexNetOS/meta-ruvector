-- RuVector PostgreSQL Extension v0.3.1 Upgrade Script
-- Adds the canonical RVF SHAKE256-256 digest primitive.

\echo Use "ALTER EXTENSION ruvector UPDATE TO '0.3.1'" to load this file. \quit

CREATE FUNCTION ruvector_shake256_256(input bytea)
RETURNS bytea
AS 'MODULE_PATHNAME', 'ruvector_shake256_256_wrapper'
LANGUAGE C IMMUTABLE STRICT PARALLEL SAFE;

REVOKE EXECUTE ON FUNCTION ruvector_shake256_256(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ruvector_shake256_256(bytea)
  TO lifeos_migrator, lifeos_envctl, lifeos_runtime;
