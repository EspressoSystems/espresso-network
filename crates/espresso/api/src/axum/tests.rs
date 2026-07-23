use super::*;

fn rewritten_uri(uri: &str) -> String {
    let req = Request::builder()
        .uri(uri)
        .body(axum::body::Body::empty())
        .unwrap();
    rewrite_legacy_uri(req).uri().to_string()
}

#[test]
fn rewrite_legacy_uri_prefixes_unversioned_paths() {
    assert_eq!(
        rewritten_uri("/status/block-height"),
        "/v1/status/block-height"
    );
}

#[test]
fn rewrite_legacy_uri_rewrites_v0_to_v1() {
    assert_eq!(
        rewritten_uri("/v0/status/block-height"),
        "/v1/status/block-height"
    );
    assert_eq!(rewritten_uri("/v0"), "/v1");
}

#[test]
fn rewrite_legacy_uri_rewrites_v0_availability_paths() {
    assert_eq!(
        rewritten_uri("/v0/availability/block/1/namespace/2"),
        "/v1/availability/block/1/namespace/2"
    );
    assert_eq!(
        rewritten_uri("/v0/availability/leaf/1"),
        "/v1/availability/leaf/1"
    );
    assert_eq!(
        rewritten_uri("/v0/availability/vid/common/1"),
        "/v1/availability/vid/common/1"
    );
    assert_eq!(
        rewritten_uri("/v0/availability/stream/leaves/0"),
        "/v1/availability/stream/leaves/0"
    );
    assert_eq!(
        rewritten_uri("/availability/block/1/namespace/2"),
        "/v1/availability/block/1/namespace/2"
    );
    assert_eq!(
        rewritten_uri("/availability/leaf/1"),
        "/v1/availability/leaf/1"
    );
}

#[test]
fn rewrite_legacy_uri_leaves_v1_unchanged() {
    assert_eq!(
        rewritten_uri("/v1/node/block-height"),
        "/v1/node/block-height"
    );
}

#[test]
fn rewrite_legacy_uri_leaves_v2_unchanged() {
    assert_eq!(
        rewritten_uri("/v2/rewards/balance/0xabc"),
        "/v2/rewards/balance/0xabc"
    );
}

#[test]
fn rewrite_legacy_uri_respects_version_prefix_boundaries() {
    assert_eq!(rewritten_uri("/v1"), "/v1");
    assert_eq!(rewritten_uri("/v2"), "/v2");
    assert_eq!(rewritten_uri("/v1x"), "/v1/v1x");
    assert_eq!(rewritten_uri("/v2-foo/bar"), "/v1/v2-foo/bar");
    assert_eq!(rewritten_uri("/v0x/leaf"), "/v1/v0x/leaf");
}

#[test]
fn rewrite_legacy_uri_leaves_reserved_paths_unchanged() {
    assert_eq!(rewritten_uri("/"), "/");
    assert_eq!(rewritten_uri("/healthcheck"), "/healthcheck");
    assert_eq!(rewritten_uri("/version"), "/version");
}

#[test]
fn rewrite_legacy_uri_preserves_query_string() {
    assert_eq!(
        rewritten_uri("/availability/leaf/1?foo=bar"),
        "/v1/availability/leaf/1?foo=bar"
    );
}

/// Implements every v1 API trait with `unimplemented!()` bodies, purely so `create_router_v1`
/// can be instantiated in tests that only exercise the static docs routes (root redirect,
/// swagger UI, OpenAPI spec) and never call into a handler.
#[derive(Clone)]
struct MockState;

#[async_trait::async_trait]
impl v1::RewardApi for MockState {
    type RewardClaimInput = ();
    type RewardBalance = ();
    type RewardAccountQueryData = ();
    type RewardAmounts = ();
    type RewardMerkleTreeData = ();
    type RewardAccountQueryDataV1 = ();
    type RewardStatePathV1 = ();
    type RewardStatePathV2 = ();

    async fn get_reward_state_height(&self) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn get_reward_state_v2_height(&self) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn get_reward_account_proof_v1(
        &self,
        _height: u64,
        _address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV1> {
        unimplemented!()
    }
    async fn get_reward_claim_input(
        &self,
        _block_height: u64,
        _address: String,
    ) -> anyhow::Result<Self::RewardClaimInput> {
        unimplemented!()
    }
    async fn get_reward_balance(
        &self,
        _height: u64,
        _address: String,
    ) -> anyhow::Result<Self::RewardBalance> {
        unimplemented!()
    }
    async fn get_latest_reward_balance(
        &self,
        _address: String,
    ) -> anyhow::Result<Self::RewardBalance> {
        unimplemented!()
    }
    async fn get_reward_account_proof(
        &self,
        _height: u64,
        _address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        unimplemented!()
    }
    async fn get_latest_reward_account_proof(
        &self,
        _address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        unimplemented!()
    }
    async fn get_reward_amounts(
        &self,
        _height: u64,
        _offset: u64,
        _limit: u64,
    ) -> anyhow::Result<Self::RewardAmounts> {
        unimplemented!()
    }
    async fn get_reward_merkle_tree_v2(
        &self,
        _height: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeData> {
        unimplemented!()
    }
    async fn get_reward_state_path_v1(
        &self,
        _snapshot: v1::merklized_state::Snapshot,
        _key: String,
    ) -> anyhow::Result<Self::RewardStatePathV1> {
        unimplemented!()
    }
    async fn get_reward_state_path_v2(
        &self,
        _snapshot: v1::merklized_state::Snapshot,
        _key: String,
    ) -> anyhow::Result<Self::RewardStatePathV2> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::AvailabilityApi for MockState {
    type NamespaceProofQueryData = ();
    type IncorrectEncodingProof = ();
    type StateCertQueryDataV1 = ();
    type StateCertQueryDataV2 = ();

    async fn get_namespace_proof(
        &self,
        _block_id: v1::BlockId,
        _namespace: u32,
    ) -> anyhow::Result<Self::NamespaceProofQueryData> {
        unimplemented!()
    }
    async fn get_namespace_proof_range(
        &self,
        _from: u64,
        _until: u64,
        _namespace: u32,
    ) -> anyhow::Result<Vec<Self::NamespaceProofQueryData>> {
        unimplemented!()
    }
    async fn stream_namespace_proofs(
        &self,
        _from: usize,
        _namespace: u32,
    ) -> anyhow::Result<BoxStream<'static, Self::NamespaceProofQueryData>> {
        unimplemented!()
    }
    async fn get_incorrect_encoding_proof(
        &self,
        _block_id: v1::BlockId,
        _namespace: u32,
    ) -> anyhow::Result<Self::IncorrectEncodingProof> {
        unimplemented!()
    }
    async fn get_state_cert(&self, _epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1> {
        unimplemented!()
    }
    async fn get_state_cert_v2(&self, _epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV2> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::HotShotAvailabilityApi for MockState {
    type Leaf = ();
    type Block = ();
    type Header = ();
    type Payload = ();
    type VidCommon = ();
    type Transaction = ();
    type TransactionWithProof = ();
    type BlockSummary = ();
    type Limits = ();
    type Cert2 = ();

    async fn get_leaf(&self, _id: v1::LeafId) -> anyhow::Result<Self::Leaf> {
        unimplemented!()
    }
    async fn get_leaf_range(&self, _from: usize, _until: usize) -> anyhow::Result<Vec<Self::Leaf>> {
        unimplemented!()
    }
    async fn get_header(&self, _id: v1::BlockId) -> anyhow::Result<Self::Header> {
        unimplemented!()
    }
    async fn get_header_range(
        &self,
        _from: usize,
        _until: usize,
    ) -> anyhow::Result<Vec<Self::Header>> {
        unimplemented!()
    }
    async fn get_block(&self, _id: v1::BlockId) -> anyhow::Result<Self::Block> {
        unimplemented!()
    }
    async fn get_block_range(
        &self,
        _from: usize,
        _until: usize,
    ) -> anyhow::Result<Vec<Self::Block>> {
        unimplemented!()
    }
    async fn get_payload(&self, _id: v1::PayloadId) -> anyhow::Result<Self::Payload> {
        unimplemented!()
    }
    async fn get_payload_range(
        &self,
        _from: usize,
        _until: usize,
    ) -> anyhow::Result<Vec<Self::Payload>> {
        unimplemented!()
    }
    async fn get_vid_common(&self, _id: v1::BlockId) -> anyhow::Result<Self::VidCommon> {
        unimplemented!()
    }
    async fn get_vid_common_range(
        &self,
        _from: usize,
        _until: usize,
    ) -> anyhow::Result<Vec<Self::VidCommon>> {
        unimplemented!()
    }
    async fn get_transaction_by_position(
        &self,
        _height: u64,
        _index: u64,
    ) -> anyhow::Result<Self::Transaction> {
        unimplemented!()
    }
    async fn get_transaction_by_hash(&self, _hash: String) -> anyhow::Result<Self::Transaction> {
        unimplemented!()
    }
    async fn get_transaction_proof_by_position(
        &self,
        _height: u64,
        _index: u64,
    ) -> anyhow::Result<Self::TransactionWithProof> {
        unimplemented!()
    }
    async fn get_transaction_proof_by_hash(
        &self,
        _hash: String,
    ) -> anyhow::Result<Self::TransactionWithProof> {
        unimplemented!()
    }
    async fn get_block_summary(&self, _height: usize) -> anyhow::Result<Self::BlockSummary> {
        unimplemented!()
    }
    async fn get_block_summary_range(
        &self,
        _from: usize,
        _until: usize,
    ) -> anyhow::Result<Vec<Self::BlockSummary>> {
        unimplemented!()
    }
    async fn get_limits(&self) -> anyhow::Result<Self::Limits> {
        unimplemented!()
    }
    async fn get_cert2(&self, _height: u64) -> anyhow::Result<Option<Self::Cert2>> {
        unimplemented!()
    }
    async fn stream_leaves(&self, _from: usize) -> anyhow::Result<BoxStream<'static, Self::Leaf>> {
        unimplemented!()
    }
    async fn stream_headers(
        &self,
        _from: usize,
    ) -> anyhow::Result<BoxStream<'static, Self::Header>> {
        unimplemented!()
    }
    async fn stream_blocks(&self, _from: usize) -> anyhow::Result<BoxStream<'static, Self::Block>> {
        unimplemented!()
    }
    async fn stream_payloads(
        &self,
        _from: usize,
    ) -> anyhow::Result<BoxStream<'static, Self::Payload>> {
        unimplemented!()
    }
    async fn stream_vid_common(
        &self,
        _from: usize,
    ) -> anyhow::Result<BoxStream<'static, Self::VidCommon>> {
        unimplemented!()
    }
    async fn stream_transactions(
        &self,
        _from: usize,
        _namespace: Option<u32>,
    ) -> anyhow::Result<BoxStream<'static, Self::Transaction>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::BlockStateApi for MockState {
    type MerkleProof = ();

    async fn get_block_state_path(
        &self,
        _snapshot: v1::merklized_state::Snapshot,
        _key: String,
    ) -> anyhow::Result<Self::MerkleProof> {
        unimplemented!()
    }
    async fn get_block_state_height(&self) -> anyhow::Result<u64> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::FeeStateApi for MockState {
    type MerkleProof = ();
    type FeeAmount = ();

    async fn get_fee_state_path(
        &self,
        _snapshot: v1::merklized_state::Snapshot,
        _key: String,
    ) -> anyhow::Result<Self::MerkleProof> {
        unimplemented!()
    }
    async fn get_fee_state_height(&self) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn get_fee_balance_latest(
        &self,
        _address: String,
    ) -> anyhow::Result<Option<Self::FeeAmount>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::StatusApi for MockState {
    async fn block_height(&self) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn success_rate(&self) -> anyhow::Result<f64> {
        unimplemented!()
    }
    async fn time_since_last_decide(&self) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn metrics(&self) -> anyhow::Result<String> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::ConfigApi for MockState {
    type HotShotConfig = ();
    type RuntimeConfig = ();

    async fn hotshot_config(&self) -> anyhow::Result<Self::HotShotConfig> {
        unimplemented!()
    }
    async fn env(&self) -> anyhow::Result<Vec<String>> {
        unimplemented!()
    }
    async fn runtime_config(&self) -> anyhow::Result<Self::RuntimeConfig> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::NodeApi for MockState {
    type VidShare = ();
    type SyncStatus = ();
    type HeaderWindow = ();
    type Limits = ();
    type StakeTable = ();
    type StakeTableCurrent = ();
    type Validators = ();
    type AllValidators = ();
    type Participation = ();
    type BlockReward = ();
    type Block = ();
    type Leaf = ();

    async fn block_height(&self) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn count_transactions(
        &self,
        _from: Option<u64>,
        _to: Option<u64>,
        _namespace: Option<u64>,
    ) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn payload_size(
        &self,
        _from: Option<u64>,
        _to: Option<u64>,
        _namespace: Option<u64>,
    ) -> anyhow::Result<u64> {
        unimplemented!()
    }
    async fn get_vid_share(&self, _id: v1::VidShareId) -> anyhow::Result<Self::VidShare> {
        unimplemented!()
    }
    async fn sync_status(&self) -> anyhow::Result<Self::SyncStatus> {
        unimplemented!()
    }
    async fn get_header_window(
        &self,
        _start: v1::HeaderWindowStart,
        _end: u64,
    ) -> anyhow::Result<Self::HeaderWindow> {
        unimplemented!()
    }
    async fn limits(&self) -> anyhow::Result<Self::Limits> {
        unimplemented!()
    }
    async fn stake_table(&self, _epoch: u64) -> anyhow::Result<Self::StakeTable> {
        unimplemented!()
    }
    async fn stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent> {
        unimplemented!()
    }
    async fn da_stake_table(&self, _epoch: u64) -> anyhow::Result<Self::StakeTable> {
        unimplemented!()
    }
    async fn da_stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent> {
        unimplemented!()
    }
    async fn get_validators(&self, _epoch: u64) -> anyhow::Result<Self::Validators> {
        unimplemented!()
    }
    async fn get_all_validators(
        &self,
        _epoch: u64,
        _offset: u64,
        _limit: u64,
    ) -> anyhow::Result<Self::AllValidators> {
        unimplemented!()
    }
    async fn current_proposal_participation(&self) -> anyhow::Result<Self::Participation> {
        unimplemented!()
    }
    async fn proposal_participation(&self, _epoch: u64) -> anyhow::Result<Self::Participation> {
        unimplemented!()
    }
    async fn current_vote_participation(&self) -> anyhow::Result<Self::Participation> {
        unimplemented!()
    }
    async fn vote_participation(&self, _epoch: u64) -> anyhow::Result<Self::Participation> {
        unimplemented!()
    }
    async fn get_block_reward(&self, _epoch: Option<u64>) -> anyhow::Result<Self::BlockReward> {
        unimplemented!()
    }
    async fn get_oldest_block(&self) -> anyhow::Result<Option<Self::Block>> {
        unimplemented!()
    }
    async fn get_oldest_leaf(&self) -> anyhow::Result<Option<Self::Leaf>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::CatchupApi for MockState {
    type FeeAccount = ();
    type RewardAccountV1 = ();
    type RewardAccountV2 = ();
    type AccountQueryData = ();
    type FeeMerkleTree = ();
    type BlocksFrontier = ();
    type ChainConfig = ();
    type LeafChain = ();
    type Cert2 = ();
    type RewardAccountQueryDataV1 = ();
    type RewardMerkleTreeV1 = ();
    type RewardAccountQueryDataV2 = ();
    type RewardMerkleTreeV2Data = ();
    type StateCert = ();

    async fn get_account(
        &self,
        _height: u64,
        _view: u64,
        _address: String,
    ) -> anyhow::Result<Self::AccountQueryData> {
        unimplemented!()
    }
    async fn get_accounts(
        &self,
        _height: u64,
        _view: u64,
        _accounts: Vec<Self::FeeAccount>,
    ) -> anyhow::Result<Self::FeeMerkleTree> {
        unimplemented!()
    }
    async fn get_blocks_frontier(
        &self,
        _height: u64,
        _view: u64,
    ) -> anyhow::Result<Self::BlocksFrontier> {
        unimplemented!()
    }
    async fn get_chain_config(&self, _commitment: String) -> anyhow::Result<Self::ChainConfig> {
        unimplemented!()
    }
    async fn get_leaf_chain(&self, _height: u64) -> anyhow::Result<Self::LeafChain> {
        unimplemented!()
    }
    async fn get_cert2(&self, _height: u64) -> anyhow::Result<Self::Cert2> {
        unimplemented!()
    }
    async fn get_reward_account_v1(
        &self,
        _height: u64,
        _view: u64,
        _address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV1> {
        unimplemented!()
    }
    async fn get_reward_accounts_v1(
        &self,
        _height: u64,
        _view: u64,
        _accounts: Vec<Self::RewardAccountV1>,
    ) -> anyhow::Result<Self::RewardMerkleTreeV1> {
        unimplemented!()
    }
    async fn get_reward_account_v2(
        &self,
        _height: u64,
        _view: u64,
        _address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV2> {
        unimplemented!()
    }
    async fn get_reward_merkle_tree_v2(
        &self,
        _height: u64,
        _view: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeV2Data> {
        unimplemented!()
    }
    async fn get_state_cert(&self, _epoch: u64) -> anyhow::Result<Self::StateCert> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::SubmitApi for MockState {
    type Transaction = ();
    type TxHash = ();

    async fn submit(&self, _tx: Self::Transaction) -> anyhow::Result<Self::TxHash> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::StateSignatureApi for MockState {
    type Signature = ();

    async fn get_state_signature(&self, _height: u64) -> anyhow::Result<Self::Signature> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::HotShotEventsApi for MockState {
    type Event = ();
    type StartupInfo = ();

    async fn startup_info(&self) -> anyhow::Result<Self::StartupInfo> {
        unimplemented!()
    }
    async fn events(&self) -> anyhow::Result<BoxStream<'static, Self::Event>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::LightClientApi for MockState {
    type LeafProof = ();
    type HeaderProof = ();
    type StakeTableEvents = ();
    type PayloadProof = ();
    type NamespaceProof = ();

    async fn get_leaf_proof(
        &self,
        _query: v1::LeafQuery,
        _finalized: Option<u64>,
    ) -> anyhow::Result<Self::LeafProof> {
        unimplemented!()
    }
    async fn get_header_proof(
        &self,
        _root: u64,
        _requested: v1::HeaderQuery,
    ) -> anyhow::Result<Self::HeaderProof> {
        unimplemented!()
    }
    async fn get_light_client_stake_table(
        &self,
        _epoch: u64,
    ) -> anyhow::Result<Self::StakeTableEvents> {
        unimplemented!()
    }
    async fn get_payload_proof(&self, _height: u64) -> anyhow::Result<Self::PayloadProof> {
        unimplemented!()
    }
    async fn get_payload_proof_range(
        &self,
        _start: u64,
        _end: u64,
    ) -> anyhow::Result<Vec<Self::PayloadProof>> {
        unimplemented!()
    }
    async fn get_lc_namespace_proof(
        &self,
        _height: u64,
        _namespace: u64,
    ) -> anyhow::Result<Self::NamespaceProof> {
        unimplemented!()
    }
    async fn get_lc_namespace_proof_range(
        &self,
        _start: u64,
        _end: u64,
        _namespace: u64,
    ) -> anyhow::Result<Vec<Self::NamespaceProof>> {
        unimplemented!()
    }
    async fn get_lc_namespaces_proof_range(
        &self,
        _start: u64,
        _end: u64,
        _namespaces: String,
    ) -> anyhow::Result<Vec<std::collections::HashMap<u64, Self::NamespaceProof>>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::ExplorerApi for MockState {
    type BlockDetail = ();
    type BlockSummaries = ();
    type TransactionDetail = ();
    type TransactionSummaries = ();
    type ExplorerSummary = ();
    type SearchResult = ();

    async fn get_block_detail(&self, _ident: v1::BlockIdent) -> anyhow::Result<Self::BlockDetail> {
        unimplemented!()
    }
    async fn get_block_summaries(
        &self,
        _target: v1::BlockIdent,
        _limit: u64,
    ) -> anyhow::Result<Self::BlockSummaries> {
        unimplemented!()
    }
    async fn get_transaction_detail(
        &self,
        _ident: v1::TxIdent,
    ) -> anyhow::Result<Self::TransactionDetail> {
        unimplemented!()
    }
    async fn get_transaction_summaries(
        &self,
        _target: v1::TxIdent,
        _limit: u64,
        _filter: v1::TxSummaryFilter,
    ) -> anyhow::Result<Self::TransactionSummaries> {
        unimplemented!()
    }
    async fn get_explorer_summary(&self) -> anyhow::Result<Self::ExplorerSummary> {
        unimplemented!()
    }
    async fn get_search_result(&self, _query: String) -> anyhow::Result<Self::SearchResult> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::TokenApi for MockState {
    async fn total_minted_supply(&self) -> anyhow::Result<String> {
        unimplemented!()
    }
    async fn circulating_supply(&self) -> anyhow::Result<String> {
        unimplemented!()
    }
    async fn circulating_supply_ethereum(&self) -> anyhow::Result<String> {
        unimplemented!()
    }
    async fn total_issued_supply(&self) -> anyhow::Result<String> {
        unimplemented!()
    }
    async fn total_reward_distributed(&self) -> anyhow::Result<String> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl v1::DatabaseApi for MockState {
    type TableSizes = ();
    type MigrationStatus = ();

    async fn get_table_sizes(&self) -> anyhow::Result<Self::TableSizes> {
        unimplemented!()
    }
    async fn get_migration_status(&self) -> anyhow::Result<Self::MigrationStatus> {
        unimplemented!()
    }
}

async fn body_string(resp: Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read response body");
    String::from_utf8(bytes.to_vec()).expect("response body is utf8")
}

#[tokio::test]
async fn root_redirects_to_v1() {
    let router = with_top_level_routes(Router::new());
    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        resp.headers().get(axum::http::header::LOCATION).unwrap(),
        "/v1"
    );
}

#[tokio::test]
async fn v1_swagger_ui_serves_html() {
    let router = create_router_v1(MockState);
    let req = Request::builder()
        .uri(routes::v1::VERSION_PREFIX)
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(content_type.contains("text/html"));
    let body = body_string(resp).await;
    assert!(body.contains(&routes::v1::openapi_spec_url()));
}

#[tokio::test]
async fn v1_openapi_spec_contains_known_route() {
    let router = create_router_v1(MockState);
    let req = Request::builder()
        .uri(routes::v1::openapi_spec_url())
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let spec: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    let expected = "/v1/status/block-height";
    assert!(
        spec["paths"]
            .as_object()
            .expect("spec has paths")
            .contains_key(expected),
        "expected {expected} in spec paths: {body}"
    );
}

#[tokio::test]
async fn max_connections_limits_in_flight_requests() {
    let router = Router::new().route(
        "/slow",
        get(|| async {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            "ok"
        }),
    );
    let router = crate::apply_connection_limit(router, 2);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    async fn get_slow(addr: std::net::SocketAddr) -> (tokio::net::TcpStream, String) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        sock.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut buf = [0u8; 32];
        let n = sock.read(&mut buf).await.unwrap();
        (sock, String::from_utf8_lossy(&buf[..n]).to_string())
    }

    use tokio::io::AsyncWriteExt;
    let mut s1 = tokio::net::TcpStream::connect(addr).await.unwrap();
    s1.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    let mut s2 = tokio::net::TcpStream::connect(addr).await.unwrap();
    s2.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    // Both requests in flight (each sleeps 2s); the third must be shed.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let (_s3, status) = get_slow(addr).await;
    assert!(
        status.contains("429"),
        "third request must be limited: {status}"
    );
}

/// Regression test: the docs routes must exist in the app a serve mode actually builds, not
/// only in `create_router_v1` (which the serve modes don't call). Assembles a router the way
/// `serve_axum_status` does, wrapped in the same top-level routes and legacy-URI rewrite
/// layers as `serve_router`, and checks the docs are reachable and the spec reflects only
/// the mounted modules.
#[tokio::test]
async fn serve_mode_assembly_serves_v1_docs() {
    let api_router = router_status(MockState).merge(router_state_signature(MockState));
    let router = with_top_level_routes(finish_v1_docs(api_router));
    let app = tower::Layer::layer(
        &tower::util::MapRequestLayer::new(rewrite_legacy_uri),
        router,
    );

    let get = |uri: String| {
        let app = app.clone();
        async move {
            let req = Request::builder()
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap();
            tower::ServiceExt::oneshot(app, req).await.unwrap()
        }
    };

    let resp = get("/".to_string()).await;
    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        resp.headers().get(axum::http::header::LOCATION).unwrap(),
        routes::v1::VERSION_PREFIX
    );

    let resp = get(routes::v1::VERSION_PREFIX.to_string()).await;
    assert_eq!(resp.status(), StatusCode::OK, "/v1 must serve the docs UI");

    let resp = get(routes::v1::openapi_spec_url()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let spec: serde_json::Value =
        serde_json::from_str(&body_string(resp).await).expect("valid JSON");
    let paths = spec["paths"].as_object().expect("spec has paths");
    assert!(paths.contains_key("/v1/status/block-height"));
    assert!(
        !paths.contains_key("/v1/availability/leaf/{height}"),
        "spec must only document the modules this mode mounts"
    );

    // Every `{name}` template segment must be declared as a path parameter, or Swagger's
    // try-it-out cannot fill the URL.
    let params = &paths["/v1/state-signature/block/{height}"]["get"]["parameters"];
    assert_eq!(
        params[0]["name"], "height",
        "template parameters must be declared: {params}"
    );
    assert_eq!(params[0]["in"], "path");
    assert_eq!(params[0]["required"], true);
    assert_eq!(params[0]["schema"]["type"], "integer");
}

/// Multi-segment templates declare one parameter per `{name}`, in template order.
#[tokio::test]
async fn v1_spec_declares_all_template_parameters() {
    let router = create_router_v1(MockState);
    let req = Request::builder()
        .uri(routes::v1::openapi_spec_url())
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
    let spec: serde_json::Value =
        serde_json::from_str(&body_string(resp).await).expect("valid JSON");
    let paths = spec["paths"].as_object().expect("spec has paths");
    for (path, item) in paths {
        let names: Vec<&str> = path
            .split('/')
            .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
            .collect();
        for op in item.as_object().unwrap().values() {
            let declared: Vec<&str> = op["parameters"]
                .as_array()
                .map(|ps| {
                    ps.iter()
                        .filter(|p| p["in"] == "path")
                        .map(|p| p["name"].as_str().unwrap())
                        .collect()
                })
                .unwrap_or_default();
            assert_eq!(
                declared, names,
                "path {path} must declare its template params"
            );
        }
    }

    // Numeric segments are typed integer, hash/key-like segments string.
    let key_path = &paths["/v1/reward-state/{height}/{key}"]["get"]["parameters"];
    assert_eq!(key_path[0]["name"], "height");
    assert_eq!(key_path[0]["schema"]["type"], "integer");
    assert_eq!(key_path[1]["name"], "key");
    assert_eq!(key_path[1]["schema"]["type"], "string");

    // Operations are grouped by module tag.
    assert_eq!(
        paths["/v1/availability/leaf/{height}"]["get"]["tags"][0],
        "availability"
    );
    assert!(
        spec["tags"]
            .as_array()
            .expect("spec has tags")
            .iter()
            .any(|t| t["name"] == "status")
    );
}
