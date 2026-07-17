-- HotShotConfig was upgraded to include parameters for proposing and voting on upgrades. Configs
-- which were persisted before this upgrade may be missing these parameters. This migration
-- initializes them with a default. We use the `||` operator to merge two JSON objects, one
-- containing default values for the new config parameters and one containing the existing config.
-- When keys are present in both, the rightmost operand (the existing config) will take precedence.
--
-- For the upgrade settings, we use JS MAX_SAFE_INTEGER for the start parameters so that nodes will
-- never do an upgrade, unless explicitly configured otherwise.
UPDATE network_config SET
    config = jsonb_set(config, '{config}', '{
        "start_proposing_view": 9007199254740991,
        "stop_proposing_view": 0,
        "start_voting_view": 9007199254740991,
        "stop_voting_view": 0,
        "start_proposing_time": 9007199254740991,
        "stop_proposing_time": 0,
        "start_voting_time": 9007199254740991,
        "stop_voting_time": 0
    }' || (config->'config'));
