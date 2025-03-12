searchState.loadedDescShard("espresso_types", 0, "Returns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nR value\nS Value\nV value\nPossible builder validation failures\nType to describe DA and Stake memberships\nPossible charge fee failures\nInformation about the genesis state which feeds into the …\nEach variant represents a specific minor version header.\nRepresents the immutable state of a node.\nPossible proposal validation failures\nAuction Results provider holding the Url of the solver in …\nThis enum is not used in code but functions as an index of …\nState to be validated by replicas.\nNumber of blocks in an epoch\nExponential backoff base delay.\nFrontier of <code>BlockMerkleTree</code>\nbuilder to use\nCommit over fee_amount, payload_commitment and metadata\nThe maximum amount of time a leader can wait to get a …\nThe address for the Push CDN’s “marshal”, A.K.A. …\nConfiguration <code>Header</code> proposals will be validated against.\ncombined network config\nthe commit this run is based on\nGet all eligible leaders of the committee for the current …\nGet all members of the committee for the current view\nthe hotshot config\nAddress of Stake Table Contract\nCurrent version of the sequencer.\nGet all members of the committee for the current view\nGet the DA stake table entry for a public key\nGet the stake table for the current view\nGet the voting success threshold for the committee\nGet the total number of DA nodes in the committee\ntime to wait until we request data associated with a …\nDisable retries and just fail after one failed attempt.\nThe underlying event\nExponential backoff exponent.\nGet the voting failure threshold for the committee\nFrontier of <code>FeeMerkleTree</code>\nFetch the auction results from the solver.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nConstruct the state with the given block header.\nConstruct a genesis validated state.\nGet the results of the auction for this Header. Only used …\nCheck if a node has stake in the committee\nCheck if a node has stake in the committee\nwhether DA membership is determined by index. if true, the …\nContains the epoch after which initial_drb_result will not …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nExponential backoff jitter as a ratio of the backoff …\nname of the key type (for debugging)\nL1 provider\nthe libp2p config\nIndex the vector of public keys with the current view …\npassword to have the orchestrator start the network, …\nExponential max delay.\nBuild a header with the parent validate state, …\nThe DRB result for the next epoch.\ntimeout before starting the next view\nglobal index of node (for testing purposes a uid)\nCommittee used when we’re in pre-epoch state\nnumber of bootstrap nodes\nPeers for catching up the stake table\nThe list of public keys that are allowed to connect to the …\nrandom builder config\nRandomized committees, filled when we receive the DrbResult\nRe-export types which have not changed across any minor …\nGet a partial snapshot of the given fee state, which …\nreward_balance at the moment is only implemented as a …\nnumber of views to run\nunique seed (for randomness? TODO)\nGet the stake table entry for a public key\nGet the stake table for the current view\nHolds Stake table and da stake\nGet the voting success threshold for the committee\nGet the total number of nodes in the committee\nThis module contains all the traits used for building the …\nsize of transactions\nnumber of transactions per view\nGet the voting upgrade threshold for the committee\nMap containing all planned and executed upgrades.\nValidate parent against known values (from state) and …\nPossible timeout or view sync certificate. If the …\nThe view number that this event originates from\ntimeout before starting next view sync round\nIndicates whether or not epochs were enabled.\nThis struct defines the public Hotshot configuration …\nThis struct defines the public Hotshot validator …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nEnum to represent the first field of different versions of …\nEach variant represents a specific minor version header.\nHeaders with versions greater than 0.1 are serialized as …\nRoot Commitment of Block Merkle Tree\nAccount (etheruem address) of builder\nA commitment to a ChainConfig or a full ChainConfig.\nFee paid by the block builder\nRoot Commitment of <code>FeeMerkleTree</code>\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe Espresso block header includes information about the …\nThe Espresso block header includes a reference to the …\nBid Recipient is not set on <code>ChainConfig</code>\nFailure cases of transaction execution\nInsufficient funds or MerkleTree error.\nTransaction submitted during incorrect Marketplace Phase\nTransaction Signature could not be verified.\nAuction Results provider holding the Url of the solver in …\nCould not resolve <code>ChainConfig</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nSerialization (and deserialization) of primitive unsigned …\nTypes related to a namespace table.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nTypes related to a namespace payload and its transaction …\nCrazy boilerplate code to make it so that [<code>NsPayloadOwned</code>] …\nImpl <code>serde</code> for type <code>$T</code> with methods named <code>$to_bytes</code>, …\nCan <code>n</code> fit into <code>byte_len</code> bytes?\nDeserialize <code>bytes</code> in little-endian form into a <code>$T</code>, padding …\nReturn the largest <code>$T</code> value that can fit into <code>byte_len</code> …\nSerialize <code>n</code> into <code>BYTE_LEN</code> bytes in little-endian form, …\nCan <code>n</code> fit into <code>byte_len</code> bytes?\nDeserialize <code>bytes</code> in little-endian form into a <code>$T</code>, padding …\nReturn the largest <code>$T</code> value that can fit into <code>byte_len</code> …\nSerialize <code>n</code> into <code>BYTE_LEN</code> bytes in little-endian form, …\nPossible charge fee failures\nGet a partial snapshot of the given fee state, which …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nRepresents the immutable state of a node.\nCurrent version of the sequencer.\nMap containing all planned and executed upgrades.\nHolds Stake table and da stake\nType to describe DA and Stake memberships\nNumber of blocks in an epoch\nAddress of Stake Table Contract\nKeys for DA members\nThe nodes eligible for leadership. NOTE: This is currently …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nDA entries indexed by public key, for efficient lookup.\nStake entries indexed by public key, for efficient lookup.\nContains the epoch after which initial_drb_result will not …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nL1 provider\nCommittee used when we’re in pre-epoch state\nPeers for catching up the stake table\nRandomized committees, filled when we receive the DrbResult\nKeys for nodes participating in the network\nHolds Stake table and da stake\nUpdates <code>Self.stake_table</code> with stake_table for …\nPossible builder validation failures\nBlock Proposal to be verified and applied.\nPossible proposal validation failures\nThis enum is not used in code but functions as an index of …\nState to be validated by replicas.\nType to hold cloned validated state and provide validation …\nUpdates state with <code>Header</code> proposal.\nUpdates the <code>ValidatedState</code> if a protocol upgrade has …\nFrontier of <code>BlockMerkleTree</code>\nConfiguration <code>Header</code> proposals will be validated against.\nCharge a fee to an account, transferring the funds to the …\nFrontier of <code>FeeMerkleTree</code>\nFind accounts that are not in memory.\nReturns the argument unchanged.\nReturns the argument unchanged.\nRetrieves the <code>ChainConfig</code>.\nInsert a fee deposit receipt\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCheck if the merkle tree is available\nPrefund an account with a given amount. Only for demo …\nTop level validation routine. Performs all validation …\nThe proposed [‘BlockMerkleTree’] must match the one in …\nValidate <code>BlockMerkleTree</code> by comparing proposed commitment …\nValidate that proposal block size does not exceed …\nValidate builder accounts by verifying signatures. All …\nValidate basic numerical soundness and builder accounts by …\nThe <code>ChainConfig</code> of proposal must be equal to the one …\nValidates proposals <code>ChainConfig</code> against expectation by …\nValidate that <code>FeeAmount</code> (or sum of fees for Marketplace …\nValidate <code>FeeMerkleTree</code> by comparing proposed commitment …\nValidate that proposal height is <code>parent_height + 1</code>.\nThe proposal Header::l1_finalized must be <code>Some</code> and …\nThe L1 head block number in the proposal must be …\nEnsure that L1 Head on proposal is not decreasing.\nProxy to <code>super::NsTable::validate()</code>.\nValidate timestamp is not decreasing relative to parent …\nThe timestamp must not drift too much from local system …\nThe timestamp must be non-decreasing relative to parent.\nWait for our view of the finalized L1 block number to …\nWait for our view of the L1 chain to catch up to the …\nWait for our view of the latest L1 block number to catch …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nData that can be deserialized from a subslice of namespace …\nTypes which can be deserialized from either integers or …\nSpecifies a subslice of namespace payload bytes to read.\nAppend decided leaves to persistent storage and emit a …\nFetch the given list of accounts, retrying on transient …\nReturns the argument unchanged.\nDeserialize <code>Self</code> from namespace payload bytes.\nUpdate storage based on an event from consensus.\nCalls <code>U::from(self)</code>.\nUse this storage as a state catchup backend, if supported.\nLoad the orchestrator config from storage.\nLoad the latest known consensus state.\nLoad the highest view saved with <code>save_voted_view</code>.\nLoad the proposals saved by consensus\nLoad undecided state saved by consensus before we shut …\nRange relative to this ns payload\nFetch and remember the blocks frontier, retrying on …\nSave the orchestrator config to storage.\nTry to fetch the given accounts state, failing without …\nTry to fetch and remember the blocks frontier, failing …\nInformation about the genesis state which feeds into the …\nExponential backoff base delay.\nDisable retries and just fail after one failed attempt.\nExponential backoff exponent.\nExponential backoff jitter as a ratio of the backoff …\nExponential max delay.\nGlobal variables for an Espresso blockchain.\nA proof of the balance of an account in the fee ledger.\n<code>FeeInfo</code> holds data related to builder fees.\nA header is like a [<code>Block</code>] with the body replaced by a …\nCartesian product of <code>NsIter</code>, <code>TxIter</code>.\nAn Ethereum provider and configuration to interact with …\nConfiguration for an L1 client.\nByte lengths for the different items that could appear in …\nByte lengths for the different items that could appear in …\nIndex for an entry in a ns table.\nReturn type for [<code>Payload::ns_iter</code>].\nRaw binary data for a single namespace’s payload.\nBuild an individual namespace payload one transaction at a …\nByte length of a namespace payload.\nIndex range for a namespace payload inside a block payload.\nProof of correctness for namespace payload bytes in a …\nRaw binary data for a namespace table.\nReturn type for <code>NsTable::validate</code>.\nNumber of entries in a namespace table.\nNumber of txs in a namespace.\nByte range for the part of a tx table that declares the …\nThe part of a tx table that declares the number of txs in …\nRaw payload data for an entire block.\nByte length of a block payload, which includes all …\nAn RPC client with multiple remote (HTTP) providers.\nUpgrade based on unix timestamp.\nRepresents an upgrade based on time (unix timestamp).\nIndex for an entry in a tx table.\nA transaction’s payload data.\nByte range for a transaction’s payload data.\nProof of correctness for transaction bytes in a block.\nEntries from a tx table in a namespace for use in a …\nByte range for entries from a tx table for use in a …\nRepresents a general upgrade with mode and type.\nRepresents the specific type of upgrade.\nRepresents the specific type of upgrade.\nUpgrade based on view.\nRepresents an upgrade based on view.\nReturn inner <code>Address</code>\nAdd an entry to the namespace table.\nAdd a transaction’s payload to this namespace\nAccess the underlying index range for this namespace …\nReturn byte slice representation of inner <code>Address</code> type\nThe minimum fee paid by the given builder account for a …\nMinimum fee in WEI per byte of payload\nConvert a <code>NsPayloadBytesRange</code> into a range that’s …\nReturn the byte length of this namespace.\nA commitment to a ChainConfig or a full ChainConfig.\nEspresso chain ID\nInstantiate an <code>L1Client</code> for a given list of provider <code>Url</code>s.\nThe transport currently being used by the client\nByte length of a single namespace table entry.\nReturn all transactions in this namespace. The namespace …\nReturn all transactions in the namespace whose payload is …\nReturn a transaction from this namespace. Set its …\nFee contract address on L1.\nAccount that receives sequencing fees.\nThe snapshot also includes information about the latest …\nSearch the namespace table for the ns_index belonging to …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet a <code>Vec&lt;FeeInfo&gt;</code> from <code>Vec&lt;BuilderFee&gt;</code>\nInstantiate an <code>NsTable</code> from a byte slice.\nNeed a sync version of <code>BlockPayload::from_transactions</code> in …\nExtract payload byte length from a <code>ADVZCommon</code> and …\nGet fee info for each <code>Deposit</code> occurring between <code>prev</code> and …\nGet <code>StakeTable</code> at block height.\nThe relevant snapshot of the L1 includes a reference to …\nByte length of a namespace table header.\nDoes the <code>index</code>th entry exist in the namespace table?\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nSerialize to bytes and consume self.\nSerialize to bytes and consume self.\nIs the payload byte length declared in a <code>ADVZCommon</code> equal …\nCheck if the given address is a proxy contract.\nIterator over all transactions in this namespace.\nIterator over all unique namespaces in the namespace table.\nPrivate helper\nMaximum number of L1 blocks to keep in cache at once.\nFail over to another provider if the current provider …\nNumber of L1 events to buffer before discarding.\nMaximum number of L1 blocks that can be scanned for events …\nFail over to another provider if the current provider …\nRequest rate when polling L1.\nAmount of time to wait after receiving a 429 response …\nDelay when retrying failed L1 queries.\nSeparate provider to use for subscription feeds.\nNumber of entries in the namespace table.\nMaximum size in bytes of a block\nThe mode of the upgrade (time-based or view-based).\nReturns the minimum of:\nTODO restrict visibility?\nCreate a new <code>SwitchingTransport</code> with the given options and …\nConstruct a new L1 client with the default options.\nReturns the <code>Transaction</code> indicated by <code>index</code>, along with a …\nReturns the payload bytes for the <code>index</code>th namespace, along …\nConvenience wrapper for <code>Self::read_ns_payload</code>.\nRead subslice range for the <code>index</code>th namespace from the …\nUseful for when we want to test size of transaction(s)\nA <code>RootProvider</code> from <code>alloy</code> which uses our custom …\nR value\nRead and parse bytes from the ns payload.\nRead the namespace id from the <code>index</code>th entry from the …\nLike <code>Self::read_ns_id</code> except <code>index</code> is not checked. Use …\nRead the namespace offset from the <code>index</code>th entry from the …\nRead the number of namespaces declared in the namespace …\nPrivate helper. (Could be pub if desired.)\nReceiver for events from the async update task.\nS Value\nChannel used by the async update task to send events to …\nShut down background tasks associated with this L1 client.\nGet a snapshot from the l1.\nStart the background tasks which keep the L1 client up to …\nthe earliest unix timestamp in which the node can propose …\nthe earliest view in which the node can propose an upgrade\nThe timestamp at which voting for the upgrade proposal …\nThe view at which voting for the upgrade proposal starts\nShared state updated by an asynchronous task which polls …\ntimestamp after which the node stops proposing an upgrade\nview after which the node stops proposing an upgrade\nThe timestamp at which voting for the upgrade proposal …\nThe view at which voting for the upgrade proposal stops\nMaximum time to wait for new heads before considering a …\nReturn array containing underlying bytes of inner <code>Address</code> …\nLike <code>QueryablePayload::transaction_with_proof</code> except …\nPrivate helper\nByte length of a single tx table entry.\nByte length of a tx table header.\nAsync task which updates the shared state.\nThe type of the upgrade.\nThe list of configured HTTP URLs to use for RPC requests\nV value\nAre the bytes of this <code>NsTable</code> uncorrupted?\nHelper for <code>NsTable::validate</code>, used in our custom <code>serde</code> …\nVerify a <code>TxProof</code> for <code>tx</code> against a payload commitment. …\nVerify a <code>NsProof</code> against a payload commitment. Returns <code>None</code>…\nWait until the highest L1 block number reaches at least …\nGet information about the given block.\nGet information about the first finalized block with …\nReturns when the transport has been switched\nUse the given metrics collector to publish metrics related …\nCartesian product of <code>NsIter</code>, <code>TxIter</code>.\nByte lengths for the different items that could appear in …\nByte lengths for the different items that could appear in …\nIndex for an entry in a ns table.\nReturn type for [<code>Payload::ns_iter</code>].\nRaw binary data for a single namespace’s payload.\nBuild an individual namespace payload one transaction at a …\nByte length of a namespace payload.\nIndex range for a namespace payload inside a block payload.\nProof of correctness for namespace payload bytes in a …\nRaw binary data for a namespace table.\nReturn type for <code>NsTable::validate</code>.\nNumber of entries in a namespace table.\nNumber of txs in a namespace.\nByte range for the part of a tx table that declares the …\nThe part of a tx table that declares the number of txs in …\nRaw payload data for an entire block.\nByte length of a block payload, which includes all …\nIndex for an entry in a tx table.\nA transaction’s payload data.\nByte range for a transaction’s payload data.\nProof of correctness for transaction bytes in a block.\nEntries from a tx table in a namespace for use in a …\nByte range for entries from a tx table for use in a …\nGlobal variables for an Espresso blockchain.\nMinimum fee in WEI per byte of payload\nEspresso chain ID\nFee contract address on L1.\nAccount that receives sequencing fees.\nMaximum size in bytes of a block\nA proof of the balance of an account in the fee ledger.\n<code>FeeInfo</code> holds data related to builder fees.\nA header is like a [<code>Block</code>] with the body replaced by a …\nA commitment to a ChainConfig or a full ChainConfig.\nUpgrade based on unix timestamp.\nRepresents an upgrade based on time (unix timestamp).\nRepresents a general upgrade with mode and type.\nRepresents the specific type of upgrade.\nRepresents the specific type of upgrade.\nUpgrade based on view.\nRepresents an upgrade based on view.\nGet the upgrade data from <code>UpgradeType</code>. As of this writing, …\nThe mode of the upgrade (time-based or view-based).\nthe earliest unix timestamp in which the node can propose …\nthe earliest view in which the node can propose an upgrade\nThe timestamp at which voting for the upgrade proposal …\nThe view at which voting for the upgrade proposal starts\ntimestamp after which the node stops proposing an upgrade\nview after which the node stops proposing an upgrade\nThe timestamp at which voting for the upgrade proposal …\nThe view at which voting for the upgrade proposal stops\nThe type of the upgrade.\nAn Ethereum provider and configuration to interact with …\nConfiguration for an L1 client.\nIn-memory view of the L1 state, updated asynchronously.\nThe state of the current provider being used by a …\nThe status of a single transport\nAn RPC client with multiple remote (HTTP) providers.\nThe transport currently being used by the client\nThe snapshot also includes information about the latest …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nThe relevant snapshot of the L1 includes a reference to …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nMaximum number of L1 blocks to keep in cache at once.\nFail over to another provider if the current provider …\nNumber of L1 events to buffer before discarding.\nMaximum number of L1 blocks that can be scanned for events …\nFail over to another provider if the current provider …\nRequest rate when polling L1.\nAmount of time to wait after receiving a 429 response …\nDelay when retrying failed L1 queries.\nSeparate provider to use for subscription feeds.\nLog a failure to call the inner transport. Returns whether …\nLog a successful call to the inner transport\nCreate a new <code>SingleTransport</code> with the given URL\nCreate a new <code>SingleTransportStatus</code> at the given URL index\nA <code>RootProvider</code> from <code>alloy</code> which uses our custom …\nReceiver for events from the async update task.\nChannel used by the async update task to send events to …\nWhether or not the transport should be switched to the …\nWhether or not this current transport is being shut down …\nShared state updated by an asynchronous task which polls …\nMaximum time to wait for new heads before considering a …\nAsync task which updates the shared state.\nThe list of configured HTTP URLs to use for RPC requests\nR value\nS Value\nV value\nGlobal variables for an Espresso blockchain.\nStake table holding all staking information (DA and non-DA …\nNewType to disambiguate DA Membership\nNewType to disambiguate StakeTable\nMinimum fee in WEI per byte of payload\nEspresso chain ID\nFee contract address on L1.\nAccount that receives sequencing fees.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCreate the consensus and DA stake tables from L1 events\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nMaximum size in bytes of a block\n<code>StakeTable </code>(proxy) contract address on L1.\nGlobal variables for an Espresso blockchain.\nMinimum fee in WEI per byte of payload\nEspresso chain ID\nFee contract address on L1.\nAccount that receives sequencing fees.\nMaximum size in bytes of a block\n<code>StakeTable </code>(proxy) contract address on L1.\nStake table holding all staking information (DA and non-DA …\nNewType to disambiguate DA Membership\nNewType to disambiguate StakeTable\nA transaction to bid for the sequencing rights of a …\nA transaction body holding data required for bid …\nGlobal variables for an Espresso blockchain.\nWrapper enum for Full Network Transactions. Each …\nA header is like a [<code>Block</code>] with the body replaced by a …\nMethods for use w/ Vec\nA commitment to a ChainConfig or a full ChainConfig.\nThe results of an Auction\nGet account submitting the bid\nget bid account\nAccount responsible for the signature\nGet amount of bid\nget bid amount\nMinimum fee in WEI per byte of payload\nThe bid amount designated in Wei.  This is different than …\nAccount that receives sequencing bids.\nReturn the body of the transaction\nA commitment to a ChainConfig or a full ChainConfig.\nEspresso chain ID\nCharge Bid. Only winning bids are charged in JIT.\nExecute <code>BidTx</code>.\nProxy for <code>execute</code> method of each transaction variant.\nFee contract address on L1.\nAccount that receives sequencing fees.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nget gas price\nFee to be sequenced in the network.  Different than the …\nEmpty results for the genesis view.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nMaximum size in bytes of a block\nThe set of namespace ids the sequencer is bidding for\nConstruct a <code>SolverAuctionResults</code>\nConstruct a new <code>BidTxBody</code>.\nGet the reserve bids of the auction\nA list of reserve sequencers being used\nSign Body and return a <code>BidTx</code>. This is the expected way to …\n<code>StakeTable </code>(proxy) contract address on L1.\nGet the cloned <code>url</code> field.\nGet the <code>url</code> field from the body.\nThe URL the HotShot leader will use to request a bundle …\nGet the urls to fetch bids from builders.\nCryptographic signature verification\nGet the view number for these auction results\nget the view number\nget the view number\nThe slot this bid is for\nview number the results are for\nGet the winning bids of the auction\nA list of the bid txs that won\nInstantiate a <code>BidTxBody</code> containing the values of <code>self</code> with …\nA transaction to bid for the sequencing rights of a …\nA transaction body holding data required for bid …\nWrapper enum for Full Network Transactions. Each …\nThe results of an Auction\nAccount responsible for the signature\nThe bid amount designated in Wei.  This is different than …\nFee to be sequenced in the network.  Different than the …\nThe set of namespace ids the sequencer is bidding for\nA list of reserve sequencers being used\nThe URL the HotShot leader will use to request a bundle …\nThe slot this bid is for\nview number the results are for\nA list of the bid txs that won\nGlobal variables for an Espresso blockchain.\nA commitment to a ChainConfig or a full ChainConfig.\nMinimum fee in WEI per byte of payload\nAccount that receives sequencing bids.\nEspresso chain ID\nFee contract address on L1.\nAccount that receives sequencing fees.\nMaximum size in bytes of a block\n<code>StakeTable </code>(proxy) contract address on L1.\nMethods for use w/ Vec\nA header is like a [<code>Block</code>] with the body replaced by a …\nA commitment to a ChainConfig or a full ChainConfig.")