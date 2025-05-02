use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use alloy::primitives::Address;
use anyhow::{Context, Ok};
use espresso_types::{
    v0_99::ChainConfig, FeeAccount, FeeAmount, GenesisHeader, L1BlockInfo, L1Client, Timestamp,
    Upgrade,
};
use serde::{Deserialize, Serialize};
use vbs::version::Version;

/// Initial configuration of an Espresso stake table.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StakeTableConfig {
    pub capacity: usize,
}

/// An L1 block from which an Espresso chain should start syncing.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum L1Finalized {
    /// Complete block info.
    ///
    /// This allows a validator to specify the exact, existing L1 block to start syncing from. A
    /// validator that specifies a specific L1 block will not be able to reach consensus with a
    /// malicious validator that starts from a different L1 block.
    Block(L1BlockInfo),

    /// An L1 block number to sync from.
    ///
    /// This allows a validator to specify a future L1 block whose hash is not yet known, and start
    /// syncing only when a finalized block with the given number becomes available. The configured
    /// L1 client will be used to fetch the rest of the block info once available.
    Number { number: u64 },

    /// A time from which to start syncing L1 blocks.
    ///
    /// This allows a validator to specify a future time at which the network should start. The
    /// network will start syncing from the first L1 block with timestamp greater than or equal to
    /// this, once said block is finalized.
    Timestamp { timestamp: Timestamp },
}

/// Genesis of an Espresso chain.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Genesis {
    #[serde(with = "version_ser")]
    pub base_version: Version,
    #[serde(with = "version_ser")]
    pub upgrade_version: Version,
    pub epoch_height: Option<u64>,
    pub epoch_start_block: Option<u64>,
    pub chain_config: ChainConfig,
    pub stake_table: StakeTableConfig,
    #[serde(default)]
    pub accounts: HashMap<FeeAccount, FeeAmount>,
    pub l1_finalized: L1Finalized,
    pub header: GenesisHeader,
    #[serde(rename = "upgrade", with = "upgrade_ser")]
    #[serde(default)]
    pub upgrades: BTreeMap<Version, Upgrade>,
}

impl Genesis {
    pub fn max_base_fee(&self) -> FeeAmount {
        let mut base_fee = self.chain_config.base_fee;

        let upgrades: Vec<&Upgrade> = self.upgrades.values().collect();

        for upgrade in upgrades {
            let chain_config = upgrade.upgrade_type.chain_config();

            if let Some(cf) = chain_config {
                base_fee = std::cmp::max(cf.base_fee, base_fee);
            }
        }

        base_fee
    }
}

impl Genesis {
    pub async fn validate_fee_contract(&self, l1: &L1Client) -> anyhow::Result<()> {
        if let Some(fee_contract_address) = self.chain_config.fee_contract {
            tracing::info!("validating fee contract at {fee_contract_address:x}");

            if !l1
                .retry_on_all_providers(|| l1.is_proxy_contract(fee_contract_address))
                .await
                .context("checking if fee contract is a proxy")?
            {
                anyhow::bail!("Fee contract address {fee_contract_address:x} is not a proxy");
            }
        }

        // now iterate over each upgrade type and validate the fee contract if it exists
        for (version, upgrade) in &self.upgrades {
            let chain_config = &upgrade.upgrade_type.chain_config();

            if chain_config.is_none() {
                continue;
            }

            let chain_config = chain_config.unwrap();

            if let Some(fee_contract_address) = chain_config.fee_contract {
                if fee_contract_address == Address::default() {
                    anyhow::bail!("Fee contract cannot use the zero address");
                } else if !l1
                    .retry_on_all_providers(|| l1.is_proxy_contract(fee_contract_address))
                    .await
                    .context(format!(
                        "checking if fee contract is a proxy in upgrade {version}",
                    ))?
                {
                    anyhow::bail!("Fee contract's address is not a proxy");
                }
            } else {
                // The Fee Contract address has to be provided for an upgrade so return an error
                anyhow::bail!("Fee contract's address for the upgrade is missing");
            }
        }
        // TODO: it's optional for the fee contract to be included in a proxy in v1 so no need to panic but revisit this after v1 https://github.com/EspressoSystems/espresso-sequencer/pull/2000#discussion_r1765174702
        Ok(())
    }
}

mod version_ser {

    use serde::{de, Deserialize, Deserializer, Serializer};
    use vbs::version::Version;

    pub fn serialize<S>(ver: &Version, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&ver.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Version, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version_str = String::deserialize(deserializer)?;

        let version: Vec<_> = version_str.split('.').collect();

        let version = Version {
            major: version[0]
                .parse()
                .map_err(|_| de::Error::custom("invalid version format"))?,
            minor: version[1]
                .parse()
                .map_err(|_| de::Error::custom("invalid version format"))?,
        };

        Ok(version)
    }
}

mod upgrade_ser {

    use std::{collections::BTreeMap, fmt};

    use espresso_types::{
        v0_1::{TimeBasedUpgrade, UpgradeMode, ViewBasedUpgrade},
        Upgrade, UpgradeType,
    };
    use serde::{
        de::{self, SeqAccess, Visitor},
        ser::SerializeSeq,
        Deserialize, Deserializer, Serialize, Serializer,
    };
    use vbs::version::Version;

    pub fn serialize<S>(map: &BTreeMap<Version, Upgrade>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Fields {
            pub version: String,
            #[serde(flatten)]
            pub mode: UpgradeMode,
            #[serde(flatten)]
            pub upgrade_type: UpgradeType,
        }

        let mut seq = serializer.serialize_seq(Some(map.len()))?;
        for (version, upgrade) in map {
            seq.serialize_element(&Fields {
                version: version.to_string(),
                mode: upgrade.mode.clone(),
                upgrade_type: upgrade.upgrade_type.clone(),
            })?
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<Version, Upgrade>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VecToHashMap;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Fields {
            pub version: String,
            // If both `time_based` and `view_based` fields are provided
            // and we use an enum for deserialization, then one of the variant fields will be ignored.
            // We want to raise an error in such a case to avoid ambiguity
            #[serde(flatten)]
            pub time_based: Option<TimeBasedUpgrade>,
            #[serde(flatten)]
            pub view_based: Option<ViewBasedUpgrade>,
            #[serde(flatten)]
            pub upgrade_type: UpgradeType,
        }

        impl<'de> Visitor<'de> for VecToHashMap {
            type Value = BTreeMap<Version, Upgrade>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a vector of tuples (key-value pairs)")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<BTreeMap<Version, Upgrade>, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut map = BTreeMap::new();

                while let Some(fields) = seq.next_element::<Fields>()? {
                    // add try_from in Version
                    let version: Vec<_> = fields.version.split('.').collect();

                    let version = Version {
                        major: version[0]
                            .parse()
                            .map_err(|_| de::Error::custom("invalid version format"))?,
                        minor: version[1]
                            .parse()
                            .map_err(|_| de::Error::custom("invalid version format"))?,
                    };

                    match (fields.time_based, fields.view_based) {
                        (Some(_), Some(_)) => {
                            return Err(de::Error::custom(
                                "both view and time mode parameters are set",
                            ))
                        },
                        (None, None) => {
                            return Err(de::Error::custom(
                                "no view or time mode parameters provided",
                            ))
                        },
                        (None, Some(v)) => {
                            if v.start_proposing_view > v.stop_proposing_view {
                                return Err(de::Error::custom(
                                    "stop_proposing_view is less than start_proposing_view",
                                ));
                            }

                            map.insert(
                                version,
                                Upgrade {
                                    mode: UpgradeMode::View(v),
                                    upgrade_type: fields.upgrade_type,
                                },
                            );
                        },
                        (Some(t), None) => {
                            if t.start_proposing_time.unix_timestamp()
                                > t.stop_proposing_time.unix_timestamp()
                            {
                                return Err(de::Error::custom(
                                    "stop_proposing_time is less than start_proposing_time",
                                ));
                            }

                            map.insert(
                                version,
                                Upgrade {
                                    mode: UpgradeMode::Time(t),
                                    upgrade_type: fields.upgrade_type.clone(),
                                },
                            );
                        },
                    }
                }

                Ok(map)
            }
        }

        deserializer.deserialize_seq(VecToHashMap)
    }
}

impl Genesis {
    pub fn to_file(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(path, toml.as_bytes())?;
        Ok(())
    }

    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).context(format!("genesis file {}", path.display()))?;
        let text = std::str::from_utf8(&bytes).context("genesis file must be UTF-8")?;

        toml::from_str(text).context("malformed genesis file")
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use alloy::{
        node_bindings::Anvil,
        primitives::{B256, U256},
        providers::{layers::AnvilProvider, ProviderBuilder},
    };
    use espresso_types::{
        L1BlockInfo, TimeBasedUpgrade, Timestamp, UpgradeMode, UpgradeType, ViewBasedUpgrade,
    };
    use sequencer_utils::{
        deployer::{self, Contracts},
        ser::FromStringOrInteger,
        test_utils::setup_test,
    };
    use toml::toml;

    use super::*;

    #[test]
    fn test_genesis_from_toml_with_optional_fields() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [accounts]
            "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f" = 100000
            "0x0000000000000000000000000000000000000000" = 42

            [l1_finalized]
            number = 64
            timestamp = "0x123def"
            hash = "0x80f5dd11f2bdda2814cb1ad94ef30a47de02cf28ad68c89e104c00c4e51bb7a5"
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));
        assert_eq!(genesis.stake_table, StakeTableConfig { capacity: 10 });
        assert_eq!(
            genesis.chain_config,
            ChainConfig {
                chain_id: 12345.into(),
                max_block_size: 30000.into(),
                base_fee: 1.into(),
                fee_recipient: FeeAccount::default(),
                fee_contract: Some(Address::default()),
                bid_recipient: None,
                stake_table_contract: None
            }
        );
        assert_eq!(
            genesis.header,
            GenesisHeader {
                timestamp: Timestamp::from_integer(123456).unwrap(),
            }
        );
        assert_eq!(
            genesis.accounts,
            [
                (
                    FeeAccount::from(Address::from([
                        0x23, 0x61, 0x8e, 0x81, 0xe3, 0xf5, 0xcd, 0xf7, 0xf5, 0x4c, 0x3d, 0x65,
                        0xf7, 0xfb, 0xc0, 0xab, 0xf5, 0xb2, 0x1e, 0x8f
                    ])),
                    100000.into()
                ),
                (FeeAccount::default(), 42.into())
            ]
            .into_iter()
            .collect::<HashMap<_, _>>()
        );
        assert_eq!(
            genesis.l1_finalized,
            L1Finalized::Block(L1BlockInfo {
                number: 64,
                timestamp: U256::from(0x123def),
                // Can't do B256 here directly because it's the wrong endianness
                hash: B256::from([
                    0x80, 0xf5, 0xdd, 0x11, 0xf2, 0xbd, 0xda, 0x28, 0x14, 0xcb, 0x1a, 0xd9, 0x4e,
                    0xf3, 0x0a, 0x47, 0xde, 0x02, 0xcf, 0x28, 0xad, 0x68, 0xc8, 0x9e, 0x10, 0x4c,
                    0x00, 0xc4, 0xe5, 0x1b, 0xb7, 0xa5
                ])
            })
        );
    }

    #[test]
    fn test_genesis_from_toml_without_optional_fields() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [l1_finalized]
            number = 0
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));
        assert_eq!(genesis.stake_table, StakeTableConfig { capacity: 10 });
        assert_eq!(
            genesis.chain_config,
            ChainConfig {
                chain_id: 12345.into(),
                max_block_size: 30000.into(),
                base_fee: 1.into(),
                fee_recipient: FeeAccount::default(),
                bid_recipient: None,
                fee_contract: None,
                stake_table_contract: None,
            }
        );
        assert_eq!(
            genesis.header,
            GenesisHeader {
                timestamp: Timestamp::from_integer(123456).unwrap(),
            }
        );
        assert_eq!(genesis.accounts, HashMap::default());
        assert_eq!(genesis.l1_finalized, L1Finalized::Number { number: 0 });
    }

    #[test]
    fn test_genesis_l1_finalized_number_only() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [l1_finalized]
            number = 42
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));
        assert_eq!(genesis.l1_finalized, L1Finalized::Number { number: 42 });
    }

    #[test]
    fn test_genesis_l1_finalized_timestamp_only() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [l1_finalized]
            timestamp = "2024-01-02T00:00:00Z"
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));
        assert_eq!(
            genesis.l1_finalized,
            L1Finalized::Timestamp {
                timestamp: Timestamp::from_string("2024-01-02T00:00:00Z".to_string()).unwrap()
            }
        );
    }

    // tests for fee contract not being a proxy are removed, since we now only have one function in `deployer.rs` that ensures
    // deploying of the fee contract behind proxy, and this function is being unit tested there.
    // Here, we primarily focus on testing the config and validation logic, not deployment logic.

    #[tokio::test(flavor = "multi_thread")]
    async fn test_genesis_fee_contract_is_a_proxy() -> anyhow::Result<()> {
        setup_test();

        let anvil = Arc::new(Anvil::new().spawn());
        let wallet = anvil.wallet().unwrap();
        let admin = wallet.default_signer().address();
        let inner_provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil.endpoint_url());
        let provider = AnvilProvider::new(inner_provider, Arc::clone(&anvil));
        let mut contracts = Contracts::new();

        let proxy_addr =
            deployer::deploy_fee_contract_proxy(&provider, &mut contracts, admin).await?;

        let toml = format!(
            r#"
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "{:?}"

            [header]
            timestamp = 123456

            [l1_finalized]
            number = 42
        "#,
            proxy_addr,
        )
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));

        // Call the validation logic for the fee_contract address
        let result = genesis
            .validate_fee_contract(&L1Client::anvil(&anvil).unwrap())
            .await;

        assert!(
            result.is_ok(),
            "Expected Fee Contract to be a proxy, but it was not"
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_genesis_fee_contract_is_a_proxy_with_upgrades() -> anyhow::Result<()> {
        setup_test();

        let anvil = Arc::new(Anvil::new().spawn());
        let wallet = anvil.wallet().unwrap();
        let admin = wallet.default_signer().address();
        let inner_provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil.endpoint_url());
        let provider = AnvilProvider::new(inner_provider, Arc::clone(&anvil));
        let mut contracts = Contracts::new();

        let proxy_addr =
            deployer::deploy_fee_contract_proxy(&provider, &mut contracts, admin).await?;

        let toml = format!(
            r#"
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [l1_finalized]
            number = 42

            [[upgrade]]
            version = "0.2"
            start_proposing_view = 5
            stop_proposing_view = 15

            [upgrade.fee]

            [upgrade.fee.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "{:?}"

            [[upgrade]]
            version = "0.3"
            start_proposing_view = 5
            stop_proposing_view = 15

            [upgrade.marketplace]
            [upgrade.marketplace.chain_config]
            chain_id = 999999999
            max_block_size = 3000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            bid_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "{:?}"
        "#,
            proxy_addr, proxy_addr,
        )
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));

        // Call the validation logic for the fee_contract address
        let result = genesis
            .validate_fee_contract(&L1Client::anvil(&anvil).unwrap())
            .await;

        assert!(
            result.is_ok(),
            "Expected Fee Contract to be a proxy, but it was not"
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_genesis_missing_fee_contract_with_upgrades() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [l1_finalized]
            number = 42

            [[upgrade]]
            version = "0.2"
            start_proposing_view = 5
            stop_proposing_view = 15

            [upgrade.fee]

            [upgrade.fee.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"

            [[upgrade]]
            version = "0.3"
            start_proposing_view = 5
            stop_proposing_view = 15

            [upgrade.marketplace]
            [upgrade.marketplace.chain_config]
            chain_id = 999999999
            max_block_size = 3000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            bid_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0xa15bb66138824a1c7167f5e85b957d04dd34e468" //not a proxy
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));
        let rpc_url = "https://ethereum-sepolia.publicnode.com";

        // validate the fee_contract address
        let result = genesis
            .validate_fee_contract(&L1Client::new(vec![rpc_url.parse().unwrap()]).unwrap())
            .await;

        // check if the result from the validation is an error
        if let Err(e) = result {
            // assert that the error message contains "Fee contract's address is not a proxy"
            assert!(e
                .to_string()
                .contains("Fee contract's address for the upgrade is missing"));
        } else {
            panic!("Expected the fee contract to be missing, but the validation succeeded");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_genesis_upgrade_fee_contract_address_is_zero() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [l1_finalized]
            number = 42

            [[upgrade]]
            version = "0.2"
            start_proposing_view = 5
            stop_proposing_view = 15

            [upgrade.fee]
            [upgrade.fee.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));
        let rpc_url = "https://ethereum-sepolia.publicnode.com";

        // validate the fee_contract address
        let result = genesis
            .validate_fee_contract(&L1Client::new(vec![rpc_url.parse().unwrap()]).unwrap())
            .await;

        // check if the result from the validation is an error
        if let Err(e) = result {
            // assert that the error message contains "Fee contract's address is not a proxy"
            assert!(e
                .to_string()
                .contains("Fee contract cannot use the zero address"));
        } else {
            panic!("Expected the fee contract to complain about the zero address but the validation succeeded");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_genesis_fee_contract_l1_failover() -> anyhow::Result<()> {
        setup_test();

        let anvil = Arc::new(Anvil::new().spawn());
        let wallet = anvil.wallet().unwrap();
        let admin = wallet.default_signer().address();
        let inner_provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil.endpoint_url());
        let provider = AnvilProvider::new(inner_provider, Arc::clone(&anvil));
        let mut contracts = Contracts::new();

        let proxy_addr =
            deployer::deploy_fee_contract_proxy(&provider, &mut contracts, admin).await?;

        let toml = format!(
            r#"
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "{:?}"

            [header]
            timestamp = 123456

            [l1_finalized]
            number = 42
        "#,
            proxy_addr
        )
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));
        genesis
            .validate_fee_contract(
                &L1Client::new(vec![
                    "http://notareall1provider".parse().unwrap(),
                    anvil.endpoint().parse().unwrap(),
                ])
                .unwrap(),
            )
            .await
            .unwrap();

        Ok(())
    }

    #[test]
    fn test_genesis_from_toml_units() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = "30mb"
            base_fee = "1 gwei"
            fee_recipient = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = "2024-05-16T11:20:28-04:00"

            [l1_finalized]
            number = 0
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));
        assert_eq!(genesis.stake_table, StakeTableConfig { capacity: 10 });
        assert_eq!(*genesis.chain_config.max_block_size, 30000000);
        assert_eq!(genesis.chain_config.base_fee, 1_000_000_000.into());
        assert_eq!(
            genesis.header,
            GenesisHeader {
                timestamp: Timestamp::from_integer(1715872828).unwrap(),
            }
        )
    }

    #[test]
    fn test_genesis_toml_fee_upgrade_view_mode() {
        // without optional fields
        // with view settings
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [accounts]
            "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f" = 100000
            "0x0000000000000000000000000000000000000000" = 42

            [l1_finalized]
            number = 64
            timestamp = "0x123def"
            hash = "0x80f5dd11f2bdda2814cb1ad94ef30a47de02cf28ad68c89e104c00c4e51bb7a5"

            [[upgrade]]
            version = "0.2"
            start_proposing_view = 1
            stop_proposing_view = 15

            [upgrade.fee]

            [upgrade.fee.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));

        let (version, genesis_upgrade) = genesis.upgrades.last_key_value().unwrap();
        println!("{:?}", genesis_upgrade);

        assert_eq!(*version, Version { major: 0, minor: 2 });

        let upgrade = Upgrade {
            mode: UpgradeMode::View(ViewBasedUpgrade {
                start_voting_view: None,
                stop_voting_view: None,
                start_proposing_view: 1,
                stop_proposing_view: 15,
            }),
            upgrade_type: UpgradeType::Fee {
                chain_config: genesis.chain_config,
            },
        };

        assert_eq!(*genesis_upgrade, upgrade);
    }

    #[test]
    fn test_genesis_toml_fee_upgrade_time_mode() {
        // without optional fields
        // with time settings
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [accounts]
            "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f" = 100000
            "0x0000000000000000000000000000000000000000" = 42

            [l1_finalized]
            number = 64
            timestamp = "0x123def"
            hash = "0x80f5dd11f2bdda2814cb1ad94ef30a47de02cf28ad68c89e104c00c4e51bb7a5"

            [[upgrade]]
            version = "0.2"
            start_proposing_time = "2024-01-01T00:00:00Z"
            stop_proposing_time = "2024-01-02T00:00:00Z"

            [upgrade.fee]

            [upgrade.fee.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
        }
        .to_string();

        let genesis: Genesis = toml::from_str(&toml).unwrap_or_else(|err| panic!("{err:#}"));

        let (version, genesis_upgrade) = genesis.upgrades.last_key_value().unwrap();

        assert_eq!(*version, Version { major: 0, minor: 2 });

        let upgrade = Upgrade {
            mode: UpgradeMode::Time(TimeBasedUpgrade {
                start_voting_time: None,
                stop_voting_time: None,
                start_proposing_time: Timestamp::from_string("2024-01-01T00:00:00Z".to_string())
                    .unwrap(),
                stop_proposing_time: Timestamp::from_string("2024-01-02T00:00:00Z".to_string())
                    .unwrap(),
            }),
            upgrade_type: UpgradeType::Fee {
                chain_config: genesis.chain_config,
            },
        };

        assert_eq!(*genesis_upgrade, upgrade);
    }

    #[test]
    fn test_genesis_toml_fee_upgrade_view_and_time_mode() {
        // set both time and view parameters
        // this should err
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [accounts]
            "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f" = 100000
            "0x0000000000000000000000000000000000000000" = 42

            [l1_finalized]
            number = 64
            timestamp = "0x123def"
            hash = "0x80f5dd11f2bdda2814cb1ad94ef30a47de02cf28ad68c89e104c00c4e51bb7a5"

            [[upgrade]]
            version = "0.2"
            start_proposing_view = 1
            stop_proposing_view = 10
            start_proposing_time = 1
            stop_proposing_time = 10

            [upgrade.fee]

            [upgrade.fee.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
        }
        .to_string();

        toml::from_str::<Genesis>(&toml).unwrap_err();
    }

    #[test]
    fn test_marketplace_upgrade_toml() {
        let toml = toml! {
            base_version = "0.3"
            upgrade_version = "0.99"

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [accounts]
            "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f" = 100000
            "0x0000000000000000000000000000000000000000" = 42

            [l1_finalized]
            number = 64
            timestamp = "0x123def"
            hash = "0x80f5dd11f2bdda2814cb1ad94ef30a47de02cf28ad68c89e104c00c4e51bb7a5"

            [[upgrade]]
            version = "0.3"
            start_proposing_view = 1
            stop_proposing_view = 10

            [upgrade.marketplace]
            [upgrade.marketplace.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            bid_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"

        }
        .to_string();

        toml::from_str::<Genesis>(&toml).unwrap();
    }

    #[test]
    fn test_fee_and_epoch_upgrade_toml() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"
            epoch_height = 20
            epoch_start_block = 1

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [accounts]
            "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f" = 100000
            "0x0000000000000000000000000000000000000000" = 42

            [l1_finalized]
            number = 64
            timestamp = "0x123def"
            hash = "0x80f5dd11f2bdda2814cb1ad94ef30a47de02cf28ad68c89e104c00c4e51bb7a5"

            [[upgrade]]
            version = "0.3"
            start_proposing_view = 1
            stop_proposing_view = 10

            [upgrade.epoch]
            [upgrade.epoch.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
            stake_table_contract = "0x0000000000000000000000000000000000000000"

            [[upgrade]]
            version = "0.2"
            start_proposing_view = 1
            stop_proposing_view = 15

            [upgrade.fee]

            [upgrade.fee.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
        }
        .to_string();

        toml::from_str::<Genesis>(&toml).unwrap();
    }

    #[test]
    fn test_fee_and_epoch_and_marketplace_upgrade_toml() {
        let toml = toml! {
            base_version = "0.1"
            upgrade_version = "0.2"
            epoch_height = 20
            epoch_start_block = 1

            [stake_table]
            capacity = 10

            [chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"

            [header]
            timestamp = 123456

            [accounts]
            "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f" = 100000
            "0x0000000000000000000000000000000000000000" = 42

            [l1_finalized]
            number = 64
            timestamp = "0x123def"
            hash = "0x80f5dd11f2bdda2814cb1ad94ef30a47de02cf28ad68c89e104c00c4e51bb7a5"

            [[upgrade]]
            version = "0.99"
            start_proposing_view = 1
            stop_proposing_view = 10

            [upgrade.marketplace]
            [upgrade.marketplace.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            bid_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
            stake_table_contract = "0x0000000000000000000000000000000000000000"

            [[upgrade]]
            version = "0.3"
            start_proposing_view = 1
            stop_proposing_view = 10

            [upgrade.epoch]
            [upgrade.epoch.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            bid_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
            stake_table_contract = "0x0000000000000000000000000000000000000000"

            [[upgrade]]
            version = "0.2"
            start_proposing_view = 1
            stop_proposing_view = 15

            [upgrade.fee]

            [upgrade.fee.chain_config]
            chain_id = 12345
            max_block_size = 30000
            base_fee = 1
            fee_recipient = "0x0000000000000000000000000000000000000000"
            fee_contract = "0x0000000000000000000000000000000000000000"
        }
        .to_string();

        toml::from_str::<Genesis>(&toml).unwrap();
    }
}
