-- HotShotConfig was upgraded to include parameters for stake table capacity and DRB difficulty. Configs
-- which were persisted before this upgrade may be missing these parameters. This migration
-- initializes them with a default. We use the `||` operator to merge two JSON objects, one
-- containing default values for the new config parameters and one containing the existing config.
-- When keys are present in both, the rightmost operand (the existing config) will take precedence.
UPDATE network_config SET
    config = jsonb_set(config, '{config}', '{
        "stake_table_capacity": 200,
        "drb_difficulty": 0,
        "drb_upgrade_difficulty": 0

    }' || (config->'config'));
