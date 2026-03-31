-- HotShotConfig was upgraded to include an `epoch_height` parameter. This migration initializes it
-- with a default. We use the `||` operator to merge two JSON objects, one containing the default
-- value for the new config parameter and one containing the existing config. When keys are present
-- in both, the rightmost operand (the existing config) will take precedence.
UPDATE network_config SET
    config = jsonb_set(config, '{config}', '{
        "epoch_height": 0
    }' || (config->'config'));
