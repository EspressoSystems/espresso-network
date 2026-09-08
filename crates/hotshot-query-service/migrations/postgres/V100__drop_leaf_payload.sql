-- A previous version of the software erroneously stored leaves in the database with the full
-- payload. This is unnecesssary, since we store payloads in their own separate table, and hurts
-- performance. The updated software no longer does this for new leaves. This migration removes the
-- redundant payloads for old leaves.
UPDATE leaf SET leaf = jsonb_set(leaf, '{block_payload}', 'null');
