-- Adds pg_sleep triggers to query-service-pruned tables to simulate slow deletes.
-- Default delay is 0.05s per row. Override per table with GUC variables.
-- See scripts/pruning-debug/README.md for usage.

SET search_path TO hotshot;

CREATE OR REPLACE FUNCTION slow_delete() RETURNS trigger AS $$
BEGIN
  PERFORM pg_sleep(COALESCE(current_setting('slow_delete.' || TG_TABLE_NAME, true)::float, 0.05));
  RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS slow_delete_trigger ON block_merkle_tree;
CREATE TRIGGER slow_delete_trigger
  BEFORE DELETE ON block_merkle_tree
  FOR EACH ROW EXECUTE FUNCTION slow_delete();

DROP TRIGGER IF EXISTS slow_delete_trigger ON fee_merkle_tree;
CREATE TRIGGER slow_delete_trigger
  BEFORE DELETE ON fee_merkle_tree
  FOR EACH ROW EXECUTE FUNCTION slow_delete();

DROP TRIGGER IF EXISTS slow_delete_trigger ON header;
CREATE TRIGGER slow_delete_trigger
  BEFORE DELETE ON header
  FOR EACH ROW EXECUTE FUNCTION slow_delete();

DROP TRIGGER IF EXISTS slow_delete_trigger ON leaf2;
CREATE TRIGGER slow_delete_trigger
  BEFORE DELETE ON leaf2
  FOR EACH ROW EXECUTE FUNCTION slow_delete();

DROP TRIGGER IF EXISTS slow_delete_trigger ON payload;
CREATE TRIGGER slow_delete_trigger
  BEFORE DELETE ON payload
  FOR EACH ROW EXECUTE FUNCTION slow_delete();

DROP TRIGGER IF EXISTS slow_delete_trigger ON vid_common;
CREATE TRIGGER slow_delete_trigger
  BEFORE DELETE ON vid_common
  FOR EACH ROW EXECUTE FUNCTION slow_delete();

DROP TRIGGER IF EXISTS slow_delete_trigger ON transactions;
CREATE TRIGGER slow_delete_trigger
  BEFORE DELETE ON transactions
  FOR EACH ROW EXECUTE FUNCTION slow_delete();
