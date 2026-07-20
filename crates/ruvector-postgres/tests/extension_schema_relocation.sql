-- Regression gate for the architecture bootstrap extension namespace.
-- Run by docker/run-integration-tests.sh against a freshly packaged image.
\set ON_ERROR_STOP on

DROP DATABASE IF EXISTS ruvector_extension_schema_test WITH (FORCE);
CREATE DATABASE ruvector_extension_schema_test;
\connect ruvector_extension_schema_test

CREATE SCHEMA extensions;
CREATE EXTENSION pgcrypto WITH SCHEMA extensions;
CREATE EXTENSION btree_gin WITH SCHEMA extensions;
CREATE EXTENSION ruvector WITH SCHEMA extensions;

DO $$
DECLARE
    installed_count integer;
    wrong_schema text;
BEGIN
    SELECT count(*), string_agg(e.extname, ', ' ORDER BY e.extname)
      INTO installed_count, wrong_schema
      FROM pg_extension AS e
      JOIN pg_namespace AS n ON n.oid = e.extnamespace
     WHERE e.extname IN ('pgcrypto', 'btree_gin', 'ruvector')
       AND n.nspname <> 'extensions';

    IF installed_count <> 0 THEN
        RAISE EXCEPTION
            'extensions installed outside schema extensions: %',
            wrong_schema;
    END IF;

    SELECT count(*)
      INTO installed_count
      FROM pg_extension AS e
      JOIN pg_namespace AS n ON n.oid = e.extnamespace
     WHERE e.extname IN ('pgcrypto', 'btree_gin', 'ruvector')
       AND n.nspname = 'extensions';

    IF installed_count <> 3 THEN
        RAISE EXCEPTION
            'expected pgcrypto, btree_gin, and ruvector in schema extensions; found %',
            installed_count;
    END IF;
END;
$$;

\connect postgres
DROP DATABASE ruvector_extension_schema_test WITH (FORCE);
