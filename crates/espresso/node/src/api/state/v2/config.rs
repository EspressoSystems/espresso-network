//! `ConfigService`, and the conversions of this crate's config types.

use super::*;

#[tonic::async_trait]
impl<D> proto::config_service_server::ConfigService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: HotShotConfigDataSource + Send + Sync,
{
    async fn get_hotshot_config(
        &self,
        _request: tonic::Request<proto::GetHotshotConfigRequest>,
    ) -> Result<tonic::Response<proto::HotshotConfigResponse>, tonic::Status> {
        let config = <Self as v1::ConfigApi>::hotshot_config(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(config.into()))
    }

    async fn get_env(
        &self,
        _request: tonic::Request<proto::GetEnvRequest>,
    ) -> Result<tonic::Response<proto::EnvResponse>, tonic::Status> {
        let variables = <Self as v1::ConfigApi>::env(self)
            .await
            .map_err(to_status)?
            .into_iter()
            .map(|entry| {
                let (name, value) = entry
                    .split_once('=')
                    .expect("ConfigApi::env yields KEY=value entries");
                proto::EnvVar {
                    name: name.to_string(),
                    value: value.to_string(),
                }
            })
            .collect();
        Ok(tonic::Response::new(proto::EnvResponse { variables }))
    }

    async fn get_runtime_config(
        &self,
        _request: tonic::Request<proto::GetRuntimeConfigRequest>,
    ) -> Result<tonic::Response<proto::RuntimeConfigResponse>, tonic::Status> {
        let config = <Self as v1::ConfigApi>::runtime_config(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(config.into()))
    }
}

// These stay here rather than in the api crate's `render` for the same reason as the stake table
// below: the source types are this crate's, and the api crate cannot depend on this one.

impl From<crate::options::PublicNodeConfig> for proto::RuntimeConfigResponse {
    fn from(config: crate::options::PublicNodeConfig) -> Self {
        // Destructured without `..` so that a new field fails to compile here instead of becoming
        // a setting v2 silently never serves; the `_` bindings are the deliberate drops. The guard
        // stops at this level, as the storage and module structs below are read field by field.
        let crate::options::PublicNodeConfig {
            orchestrator_url,
            cdn_endpoint,
            cliquenet_bind_address,
            cliquenet_advertise_address,
            libp2p_bind_address,
            libp2p_advertise_address,
            libp2p_bootstrap_nodes,
            public_api_url,
            builder_urls,
            state_relay_server_url,
            state_peers,
            config_peers,
            is_da,
            genesis_file,
            genesis: _,
            identity,
            catchup_base_timeout: _,
            local_catchup_timeout: _,
            bootstrap_epoch_catchup_timeout: _,
            catchup_backoff: _,
            proposal_fetcher: _,
            libp2p: _,
            l1: _,
            l1_provider_count,
            l1_ws_provider_count,
            storage,
            modules,
        } = config;
        let crate::options::Identity {
            node_name,
            node_description,
            company_name,
            company_website,
            country_code,
            latitude,
            longitude,
            operating_system,
            node_type,
            network_type,
            icon_14x14_1x,
            icon_14x14_2x,
            icon_14x14_3x,
            icon_24x24_1x,
            icon_24x24_2x,
            icon_24x24_3x,
        } = identity;
        Self {
            is_da,
            identity: Some(proto::NodeIdentity {
                node_name,
                node_description,
                company_name,
                company_website: company_website.map(|url| url.to_string()),
                country_code,
                latitude,
                longitude,
                operating_system,
                node_type,
                network_type,
                icon_14x14_1x: icon_14x14_1x.map(|url| url.to_string()),
                icon_14x14_2x: icon_14x14_2x.map(|url| url.to_string()),
                icon_14x14_3x: icon_14x14_3x.map(|url| url.to_string()),
                icon_24x24_1x: icon_24x24_1x.map(|url| url.to_string()),
                icon_24x24_2x: icon_24x24_2x.map(|url| url.to_string()),
                icon_24x24_3x: icon_24x24_3x.map(|url| url.to_string()),
            }),
            storage: Some(storage.into()),
            genesis_file: genesis_file.to_string(),
            public_api_url: public_api_url.map(|url| url.to_string()),
            builder_urls: builder_urls.iter().map(ToString::to_string).collect(),
            state_relay_server_url: state_relay_server_url.to_string(),
            state_peers: state_peers.iter().map(ToString::to_string).collect(),
            config_peers: config_peers
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect(),
            orchestrator_url: orchestrator_url.to_string(),
            cdn_endpoint,
            // `unbracketed_string` rather than `to_string`: NetAddr's Display brackets an IPv6
            // literal and its serde impl does not, so v1 serves the unbracketed form.
            cliquenet_bind_address: cliquenet_bind_address.unbracketed_string(),
            cliquenet_advertise_address: cliquenet_advertise_address
                .map(|addr| addr.unbracketed_string()),
            libp2p_bind_address,
            libp2p_advertise_address,
            libp2p_bootstrap_nodes: libp2p_bootstrap_nodes
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect(),
            l1_provider_count: l1_provider_count as u64,
            l1_ws_provider_count: l1_ws_provider_count as u64,
            modules: Some(modules.into()),
        }
    }
}

impl From<crate::options::StorageConfig> for proto::NodeStorage {
    fn from(storage: crate::options::StorageConfig) -> Self {
        Self {
            backend: match storage.backend {
                crate::options::StorageBackend::Sql => proto::StorageBackend::Sql,
                crate::options::StorageBackend::Fs => proto::StorageBackend::Fs,
                crate::options::StorageBackend::FsDefault => proto::StorageBackend::FsDefault,
            }
            .into(),
            fs: storage.fs.map(|fs| proto::FsStorage {
                path: fs.path.display().to_string(),
                consensus_view_retention: fs.consensus_view_retention,
            }),
            sql: storage.sql.map(Into::into),
        }
    }
}

impl From<crate::options::SqlStorageConfig> for proto::SqlStorage {
    fn from(sql: crate::options::SqlStorageConfig) -> Self {
        let millis = |duration: Duration| duration.as_millis() as u64;
        Self {
            prune: sql.prune,
            archive: sql.archive,
            lightweight: sql.lightweight,
            disable_proactive_fetching: sql.disable_proactive_fetching,
            fetch_rate_limit: sql.fetch_rate_limit.map(|limit| limit as u64),
            active_fetch_delay_ms: sql.active_fetch_delay.map(millis),
            chunk_fetch_delay_ms: sql.chunk_fetch_delay.map(millis),
            sync_status_chunk_size: sql.sync_status_chunk_size.map(|size| size as u64),
            sync_status_ttl_ms: sql.sync_status_ttl.map(millis),
            proactive_scan_chunk_size: sql.proactive_scan_chunk_size.map(|size| size as u64),
            proactive_scan_interval_ms: sql.proactive_scan_interval.map(millis),
            idle_connection_timeout_ms: millis(sql.idle_connection_timeout),
            connection_timeout_ms: millis(sql.connection_timeout),
            slow_statement_threshold_ms: millis(sql.slow_statement_threshold),
            statement_timeout_ms: millis(sql.statement_timeout),
            min_connections: sql.min_connections,
            max_connections: sql.max_connections,
            query_min_connections: sql.query_min_connections,
            query_max_connections: sql.query_max_connections,
            pruning: Some(proto::PruningConfig {
                pruning_threshold: sql.pruning.pruning_threshold,
                minimum_retention_ms: sql.pruning.minimum_retention.map(millis),
                target_retention_ms: sql.pruning.target_retention.map(millis),
                batch_size: sql.pruning.batch_size,
                max_usage: sql.pruning.max_usage.map(u32::from),
                interval_ms: sql.pruning.interval.map(millis),
                pages: sql.pruning.pages,
            }),
            consensus_pruning: Some(proto::ConsensusPruningConfig {
                target_retention: sql.consensus_pruning.target_retention,
                minimum_retention: sql.consensus_pruning.minimum_retention,
                target_usage: sql.consensus_pruning.target_usage,
            }),
        }
    }
}

impl From<crate::options::ApiModulesConfig> for proto::ApiModules {
    fn from(modules: crate::options::ApiModulesConfig) -> Self {
        Self {
            http: modules.http.map(|http| proto::HttpModule {
                port: http.port.into(),
                max_connections: http.max_connections.map(|max| max as u64),
                tonic_port: http.tonic_port.map(u32::from),
            }),
            query: modules.query.map(|query| proto::QueryModule {
                peers: query.peers.iter().map(ToString::to_string).collect(),
                light_client: Some(proto::LightClientModuleOptions {
                    num_stake_tables_in_memory: query.light_client.num_stake_tables_in_memory
                        as u64,
                }),
                light_client_db: Some(proto::LightClientDbOptions {
                    num_connections: query.light_client_db.num_connections,
                    num_leaves: query.light_client_db.num_leaves,
                    num_stake_tables: query.light_client_db.num_stake_tables,
                    lc_path: query
                        .light_client_db
                        .lc_path
                        .map(|path| path.display().to_string()),
                }),
            }),
            submit: modules.submit,
            status: modules.status,
            catchup: modules.catchup,
            config: modules.config,
            hotshot_events: modules.hotshot_events,
            explorer: modules.explorer,
            light_client: modules.light_client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A node under test registers no runtime config, so `test_v2_api_agrees_with_v1` only ever
    /// reaches the 404. This covers the mapping itself: every field the proto promises comes from
    /// the matching `PublicNodeConfig` field, with identity values distinct enough that a mapping
    /// crossing two of them fails.
    #[tokio::test]
    async fn runtime_config_mirrors_public_node_config() {
        use proto::config_service_server::ConfigService as _;

        use crate::options::{
            Identity, PublicNodeConfig,
            tests::{parse_options_with, test_genesis},
        };

        struct UnusedDataSource;

        impl HotShotConfigDataSource for UnusedDataSource {
            async fn get_config(&self) -> espresso_types::config::PublicNetworkConfig {
                unreachable!("the runtime config is served from the state, not the data source")
            }
        }

        let opt = parse_options_with(&[
            "--config-peers",
            "https://peer1.test,https://peer2.test",
            "--cliquenet-bind-address",
            "[2001:db8::1]:9999",
            "--",
            "http",
            "--port",
            "24000",
            "--",
            "query",
            "--peers",
            "https://query1.test,https://query2.test",
            "--light-client-db-num-connections",
            "7",
            "--light-client-db-num-leaves",
            "11",
            "--light-client-db-num-stake-tables",
            "13",
            "--",
            "config",
        ]);
        let mut cfg = PublicNodeConfig::new(&opt, &opt.modules(), &test_genesis());
        cfg.identity = Identity {
            node_name: Some("node-name".into()),
            node_description: Some("node-description".into()),
            company_name: Some("company-name".into()),
            company_website: Some("https://company.test/".parse().unwrap()),
            country_code: Some("DE".into()),
            latitude: Some(1.5),
            longitude: Some(-2.5),
            operating_system: Some("operating-system".into()),
            node_type: Some("node-type".into()),
            network_type: Some("network-type".into()),
            icon_14x14_1x: Some("https://icons.test/14/1".parse().unwrap()),
            icon_14x14_2x: Some("https://icons.test/14/2".parse().unwrap()),
            icon_14x14_3x: Some("https://icons.test/14/3".parse().unwrap()),
            icon_24x24_1x: Some("https://icons.test/24/1".parse().unwrap()),
            icon_24x24_2x: Some("https://icons.test/24/2".parse().unwrap()),
            icon_24x24_3x: Some("https://icons.test/24/3".parse().unwrap()),
        };

        let state = NodeApiStateImpl::new(std::sync::Arc::new(UnusedDataSource))
            .with_public_node_config(Some(cfg.clone()));
        let runtime = state
            .get_runtime_config(tonic::Request::new(proto::GetRuntimeConfigRequest {}))
            .await
            .unwrap()
            .into_inner();

        fn strings<T: ToString>(values: &[T]) -> Vec<String> {
            values.iter().map(ToString::to_string).collect()
        }

        assert_eq!(
            runtime,
            proto::RuntimeConfigResponse {
                is_da: cfg.is_da,
                identity: Some(proto::NodeIdentity {
                    node_name: Some("node-name".into()),
                    node_description: Some("node-description".into()),
                    company_name: Some("company-name".into()),
                    company_website: Some("https://company.test/".into()),
                    country_code: Some("DE".into()),
                    latitude: Some(1.5),
                    longitude: Some(-2.5),
                    operating_system: Some("operating-system".into()),
                    node_type: Some("node-type".into()),
                    network_type: Some("network-type".into()),
                    icon_14x14_1x: Some("https://icons.test/14/1".into()),
                    icon_14x14_2x: Some("https://icons.test/14/2".into()),
                    icon_14x14_3x: Some("https://icons.test/14/3".into()),
                    icon_24x24_1x: Some("https://icons.test/24/1".into()),
                    icon_24x24_2x: Some("https://icons.test/24/2".into()),
                    icon_24x24_3x: Some("https://icons.test/24/3".into()),
                }),
                storage: Some(proto::NodeStorage {
                    backend: proto::StorageBackend::FsDefault as i32,
                    // Not pinned to `None`: the default backend parses an empty argv, which
                    // still reads ESPRESSO_NODE_STORAGE_PATH.
                    fs: cfg.storage.fs.as_ref().map(|fs| proto::FsStorage {
                        path: fs.path.display().to_string(),
                        consensus_view_retention: fs.consensus_view_retention,
                    }),
                    sql: None,
                }),
                genesis_file: cfg.genesis_file.to_string(),
                public_api_url: cfg.public_api_url.as_ref().map(ToString::to_string),
                builder_urls: strings(&cfg.builder_urls),
                state_relay_server_url: cfg.state_relay_server_url.to_string(),
                state_peers: strings(&cfg.state_peers),
                config_peers: strings(cfg.config_peers.as_deref().unwrap()),
                orchestrator_url: cfg.orchestrator_url.to_string(),
                cdn_endpoint: cfg.cdn_endpoint.clone(),
                cliquenet_bind_address: cfg.cliquenet_bind_address.unbracketed_string(),
                cliquenet_advertise_address: cfg
                    .cliquenet_advertise_address
                    .as_ref()
                    .map(|addr| addr.unbracketed_string()),
                libp2p_bind_address: cfg.libp2p_bind_address.clone(),
                libp2p_advertise_address: cfg.libp2p_advertise_address.clone(),
                libp2p_bootstrap_nodes: cfg
                    .libp2p_bootstrap_nodes
                    .as_deref()
                    .map(strings)
                    .unwrap_or_default(),
                l1_provider_count: cfg.l1_provider_count as u64,
                l1_ws_provider_count: cfg.l1_ws_provider_count as u64,
                modules: Some(proto::ApiModules {
                    http: Some(proto::HttpModule {
                        port: 24000,
                        max_connections: None,
                        tonic_port: None,
                    }),
                    query: Some(proto::QueryModule {
                        peers: vec![
                            "https://query1.test/".to_string(),
                            "https://query2.test/".to_string(),
                        ],
                        light_client: Some(proto::LightClientModuleOptions {
                            num_stake_tables_in_memory: cfg
                                .modules
                                .query
                                .as_ref()
                                .unwrap()
                                .light_client
                                .num_stake_tables_in_memory
                                as u64,
                        }),
                        // Three same-typed fields whose defaults are 5/100/100, so the flags above
                        // give each a distinct value: a crossed pair would pass otherwise.
                        light_client_db: Some(proto::LightClientDbOptions {
                            num_connections: 7,
                            num_leaves: 11,
                            num_stake_tables: 13,
                            lc_path: None,
                        }),
                    }),
                    submit: false,
                    status: false,
                    catchup: false,
                    config: true,
                    hotshot_events: false,
                    explorer: false,
                    light_client: false,
                }),
            }
        );
        // IPv6 because that is the only case where NetAddr's Display and its serde impl
        // disagree, and v1 goes through serde.
        assert_eq!(runtime.config_peers.len(), 2);
        assert_eq!(runtime.cliquenet_bind_address, "2001:db8::1:9999");
    }

    /// A TestNetwork leaves all of these at their defaults, so the live parity test compares them
    /// `None` to `None` and empty to empty. Exercised here with values instead.
    #[test]
    fn hotshot_config_renders_the_fields_a_test_network_leaves_empty() {
        use espresso_types::config::PublicNetworkConfig;
        use hotshot_types::{
            PeerConfig, VersionedDaCommittee,
            network::{BuilderType, CombinedNetworkConfig, Libp2pConfig, NetworkConfig},
        };

        let peer_id = libp2p::PeerId::random();
        let multiaddr: libp2p::Multiaddr = "/ip4/10.0.0.1/tcp/1769".parse().unwrap();
        let committee_member = PeerConfig::<SeqTypes>::test_default();

        let mut network_config = NetworkConfig::<SeqTypes> {
            commit_sha: "deadbeef".to_string(),
            cdn_marshal_address: Some("marshal.test:8083".to_string()),
            builder: BuilderType::Random,
            libp2p_config: Some(Libp2pConfig {
                bootstrap_nodes: vec![(peer_id, multiaddr.clone())],
            }),
            combined_network_config: Some(CombinedNetworkConfig {
                delay_duration: Duration::from_millis(1500),
            }),
            ..Default::default()
        };
        network_config.config.da_committees = vec![VersionedDaCommittee {
            start_version: vbs::version::Version { major: 0, minor: 6 },
            start_epoch: 10,
            committee: vec![committee_member.clone()],
        }];

        let served: proto::HotshotConfigResponse = PublicNetworkConfig::from(network_config).into();

        assert_eq!(served.commit_sha, "deadbeef");
        assert_eq!(
            served.cdn_marshal_address.as_deref(),
            Some("marshal.test:8083")
        );
        assert_eq!(served.builder, proto::BuilderType::Random as i32);
        assert_eq!(
            served.libp2p_config,
            Some(proto::Libp2pNetworkConfig {
                bootstrap_nodes: vec![proto::Libp2pBootstrapNode {
                    peer_id: peer_id.to_string(),
                    multiaddr: multiaddr.to_string(),
                }],
            })
        );
        assert_eq!(
            served.combined_network_config,
            Some(proto::CombinedNetworkConfig {
                delay_duration_ms: 1500,
            })
        );
        // The version renders as v1's `version_ser` writes it, not as `Debug`.
        let da_committee = &served.da_committees[0];
        assert_eq!(served.da_committees.len(), 1);
        assert_eq!(da_committee.start_version, "0.6");
        assert_eq!(da_committee.start_epoch, 10);
        assert_eq!(
            da_committee.committee,
            vec![proto::PeerConfig::from(committee_member)]
        );
    }

    // Postgres only, as in `options.rs`: storage-sql under embedded-db needs a `--path` that is
    // irrelevant here.
    #[cfg(not(feature = "embedded-db"))]
    #[tokio::test]
    async fn sql_storage_settings_are_served_in_full() {
        use proto::config_service_server::ConfigService as _;

        use crate::options::{
            PublicNodeConfig,
            tests::{parse_options_with, test_genesis},
        };

        struct UnusedDataSource;

        impl HotShotConfigDataSource for UnusedDataSource {
            async fn get_config(&self) -> espresso_types::config::PublicNetworkConfig {
                unreachable!("the runtime config is served from the state, not the data source")
            }
        }

        let opt = parse_options_with(&[
            "--cliquenet-bind-address",
            "127.0.0.1:9999",
            "--",
            "storage-sql",
            "--prune",
            "--pruning-threshold",
            "1000000000000",
        ]);
        let cfg = PublicNodeConfig::new(&opt, &opt.modules(), &test_genesis());
        let sql = cfg.storage.sql.clone().expect("storage-sql was configured");

        let state = NodeApiStateImpl::new(std::sync::Arc::new(UnusedDataSource))
            .with_public_node_config(Some(cfg));
        let storage = state
            .get_runtime_config(tonic::Request::new(proto::GetRuntimeConfigRequest {}))
            .await
            .unwrap()
            .into_inner()
            .storage
            .expect("the runtime config always reports a backend");

        assert_eq!(storage.backend, proto::StorageBackend::Sql as i32);
        assert_eq!(storage.fs, None);
        // Compared against the source, not against `sql.clone().into()`, which would assert the
        // mapping against itself. v1 serves the durations as `{secs, nanos}`.
        assert_eq!(
            storage.sql,
            Some(proto::SqlStorage {
                prune: true,
                archive: sql.archive,
                lightweight: sql.lightweight,
                disable_proactive_fetching: sql.disable_proactive_fetching,
                fetch_rate_limit: sql.fetch_rate_limit.map(|limit| limit as u64),
                active_fetch_delay_ms: sql.active_fetch_delay.map(|delay| delay.as_millis() as u64),
                chunk_fetch_delay_ms: sql.chunk_fetch_delay.map(|delay| delay.as_millis() as u64),
                sync_status_chunk_size: sql.sync_status_chunk_size.map(|size| size as u64),
                sync_status_ttl_ms: sql.sync_status_ttl.map(|ttl| ttl.as_millis() as u64),
                proactive_scan_chunk_size: sql.proactive_scan_chunk_size.map(|size| size as u64),
                proactive_scan_interval_ms: sql
                    .proactive_scan_interval
                    .map(|interval| interval.as_millis() as u64),
                idle_connection_timeout_ms: sql.idle_connection_timeout.as_millis() as u64,
                connection_timeout_ms: sql.connection_timeout.as_millis() as u64,
                slow_statement_threshold_ms: sql.slow_statement_threshold.as_millis() as u64,
                statement_timeout_ms: sql.statement_timeout.as_millis() as u64,
                min_connections: sql.min_connections,
                max_connections: sql.max_connections,
                query_min_connections: sql.query_min_connections,
                query_max_connections: sql.query_max_connections,
                pruning: Some(proto::PruningConfig {
                    pruning_threshold: Some(1000000000000),
                    minimum_retention_ms: sql
                        .pruning
                        .minimum_retention
                        .map(|retention| retention.as_millis() as u64),
                    target_retention_ms: sql
                        .pruning
                        .target_retention
                        .map(|retention| retention.as_millis() as u64),
                    batch_size: sql.pruning.batch_size,
                    max_usage: sql.pruning.max_usage.map(u32::from),
                    interval_ms: sql
                        .pruning
                        .interval
                        .map(|interval| interval.as_millis() as u64),
                    pages: sql.pruning.pages,
                }),
                consensus_pruning: Some(proto::ConsensusPruningConfig {
                    target_retention: sql.consensus_pruning.target_retention,
                    minimum_retention: sql.consensus_pruning.minimum_retention,
                    target_usage: sql.consensus_pruning.target_usage,
                }),
            })
        );
        // The defaults these come from are non-zero, so the comparisons above are not vacuous.
        let served = storage.sql.unwrap();
        assert!(served.statement_timeout_ms > 0);
        assert!(served.consensus_pruning.unwrap().target_retention > 0);
    }
}
