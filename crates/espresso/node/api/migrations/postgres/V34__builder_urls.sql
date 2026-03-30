-- When multi-builder support was added, the configuration field `builder_url: Url` was replaced by
-- an array `builder_urls: Vec<Url>`. If the saved config has no `builder_urls` field, it is older
-- than this change. Populate `builder_urls` with a singleton array formed from the old value of
-- `builder_url`, and delete the no longer used `builder_url`.
UPDATE network_config
   SET config =
        jsonb_set(config, '{config,builder_urls}', jsonb_build_array(config->'config'->>'builder_url'))
        #- '{config,builder_url}'
 WHERE NOT (config->'config' ? 'builder_urls');
