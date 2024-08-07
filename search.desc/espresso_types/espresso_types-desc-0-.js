searchState.loadedDescShard("espresso_types", 0, "Returns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nR value\nS Value\nV value\nPossible builder validation failures\nPossible charge fee failures\nInformation about the genesis state which feeds into the …\nEach variant represents a specific minor version header.\nRepresents the immutable state of a node.\nPossible proposal validation failures\nThis enum is not used in code but functions as an index of …\nExponential backoff base delay.\nFrontier of Block Merkle Tree\nbuilder to use\nThe maximum amount of time a leader can wait to get a …\nThe address for the Push CDN’s “marshal”, A.K.A. …\ncombined network config\nthe commit this run is based on\nthe hotshot config\nCurrent version of the sequencer.\ntime to wait until we request data associated with a …\nThe underlying event\nExponential backoff exponent.\nFee Merkle Tree\nReturns the argument unchanged.\nwhether DA membership is determined by index. if true, the …\nCalls <code>U::from(self)</code>.\nExponential backoff jitter as a ratio of the backoff …\nname of the key type (for debugging)\nthe libp2p config\npassword to have the orchestrator start the network, …\nExponential max delay.\ntimeout before starting the next view\nglobal index of node (for testing purposes a uid)\nnumber of bootstrap nodes\nrandom builder config\nRe-export types which have not changed across any minor …\nnumber of views to run\nunique seed (for randomness? TODO)\ndelay before beginning consensus\nThis module contains all the traits used for building the …\nsize of transactions\nnumber of transactions per view\nMap containing all planned and executed upgrades.\nThe view number that this event originates from\ntimeout before starting next view sync round\nEnum to represent the first field of different versions of …\nEach variant represents a specific minor version header.\nHeaders with versions greater than 0.1 are serialized as …\nRoot Commitment of Block Merkle Tree\nCommit over fee_amount, payload_commitment and metadata\nAccount (etheruem address) of builder\nA commitment to a ChainConfig or a full ChainConfig.\nFee paid by the block builder\nRoot Commitment of <code>FeeMerkleTree</code>\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet the results of the auction for this Header. Only used …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe Espresso block header includes information a bout the …\nThe Espresso block header includes a reference to the …\nBuild a header with the parent validate state, …\nreward_balance at the moment is only implemented as a …\nBid Recipient is not set on <code>ChainConfig</code>\nFailure cases of transaction execution\nInsufficient funds or MerkleTree error.\nTransaction submitted during incorrect Marketplace Phase\nTransaction Signature could not be verified.\nCould not resolve <code>ChainConfig</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nSerialization (and deserialization) of primitive unsigned …\nTypes related to a namespace table.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nTypes related to a namespace payload and its transaction …\nCrazy boilerplate code to make it so that [<code>NsPayloadOwned</code>] …\nImpl <code>serde</code> for type <code>$T</code> with methods named <code>$to_bytes</code>, …\nCan <code>n</code> fit into <code>byte_len</code> bytes?\nDeserialize <code>bytes</code> in little-endian form into a <code>$T</code>, padding …\nReturn the largest <code>$T</code> value that can fit into <code>byte_len</code> …\nSerialize <code>n</code> into <code>BYTE_LEN</code> bytes in little-endian form, …\nCan <code>n</code> fit into <code>byte_len</code> bytes?\nDeserialize <code>bytes</code> in little-endian form into a <code>$T</code>, padding …\nReturn the largest <code>$T</code> value that can fit into <code>byte_len</code> …\nSerialize <code>n</code> into <code>BYTE_LEN</code> bytes in little-endian form, …\nPossible charge fee failures\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nRepresents the immutable state of a node.\nCurrent version of the sequencer.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nMap containing all planned and executed upgrades.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nPossible builder validation failures\nPossible proposal validation failures\nThis enum is not used in code but functions as an index of …\nUpdates the <code>ValidatedState</code> if a protocol upgrade has …\nFrontier of Block Merkle Tree\nCharge a fee to an account, transferring the funds to the …\nFee Merkle Tree\nFind accounts that are not in memory.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nConstruct the state with the given block header.\nConstruct a genesis validated state.\nRetrieves the <code>ChainConfig</code>.\nInsert a fee deposit receipt\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCheck if the merkle tree is available\nPrefund an account with a given amount. Only for demo …\nValidate parent against known values (from state) and …\nValidate builder accounts by verifying signatures. All …\nData that can be deserialized from a subslice of namespace …\nTypes which can be deserialized from either integers or …\nSpecifies a subslice of namespace payload bytes to read.\nFetch the given list of accounts, retrying on transient …\nDeserialize <code>Self</code> from namespace payload bytes.\nUpdate storage based on an event from consensus.\nUse this storage as a state catchup backend, if supported.\nLoad the latest leaf saved with <code>save_anchor_leaf</code>.\nLoad the orchestrator config from storage.\nLoad the latest known consensus state.\nLoad the highest view saved with <code>save_voted_view</code>.\nLoad the proposals saved by consensus\nLoad undecided state saved by consensus before we shut …\nRange relative to this ns payload\nFetch and remember the blocks frontier, retrying on …\nSaves the latest decided leaf.\nSave the orchestrator config to storage.\nTry to fetch the given account state, failing without …\nTry to fetch and remember the blocks frontier, failing …\nInformation about the genesis state which feeds into the …\nExponential backoff base delay.\nExponential backoff exponent.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nExponential backoff jitter as a ratio of the backoff …\nExponential max delay.\nGlobal variables for an Espresso blockchain.\nA proof of the balance of an account in the fee ledger.\n<code>FeeInfo</code> holds data related to builder fees.\nA header is like a [<code>Block</code>] with the body replaced by a …\nCartesian product of <code>NsIter</code>, <code>TxIter</code>.\nAn Http Provider and configuration to interact with the L1.\nByte lengths for the different items that could appear in …\nByte lengths for the different items that could appear in …\nIndex for an entry in a ns table.\nReturn type for [<code>Payload::ns_iter</code>].\nRaw binary data for a single namespace’s payload.\nBuild an individual namespace payload one transaction at a …\nByte length of a namespace payload.\nIndex range for a namespace payload inside a block payload.\nProof of correctness for namespace payload bytes in a …\nRaw binary data for a namespace table.\nReturn type for <code>NsTable::validate</code>.\nNumber of entries in a namespace table.\nNumber of txs in a namespace.\nByte range for the part of a tx table that declares the …\nThe part of a tx table that declares the number of txs in …\nRaw payload data for an entire block.\nByte length of a block payload, which includes all …\nUpgrade based on unix timestamp.\nRepresents an upgrade based on time (unix timestamp).\nIndex for an entry in a tx table.\nA transaction’s payload data.\nByte range for a transaction’s payload data.\nProof of correctness for transaction bytes in a block.\nEntries from a tx table in a namespace for use in a …\nByte range for entries from a tx table for use in a …\nRepresents a general upgrade with mode and type.\nRepresents the specific type of upgrade.\nRepresents the specific type of upgrade.\nUpgrade based on view.\nRepresents an upgrade based on view.\nMinimum fee in WEI per byte of payload\nA commitment to a ChainConfig or a full ChainConfig.\nEspresso chain ID\nMaximum number of L1 blocks that can be scanned for events …\nFee contract address on L1.\nAccount that receives sequencing fees.\nThe snapshot also includes information about the latest …\nThe relevant snapshot of the L1 includes a reference to …\nMaximum size in bytes of a block\nThe mode of the upgrade (time-based or view-based).\n<code>Provider</code> from <code>ethers-provider</code>.\nR value\nS Value\nthe earliest unix timestamp in which the node can propose …\nthe earliest view in which the node can propose an upgrade\nThe timestamp at which voting for the upgrade proposal …\nThe view at which voting for the upgrade proposal starts\ntimestamp after which the node stops proposing an upgrade\nview after which the node stops proposing an upgrade\nThe timestamp at which voting for the upgrade proposal …\nThe view at which voting for the upgrade proposal stops\nThe type of the upgrade.\nV value\nCartesian product of <code>NsIter</code>, <code>TxIter</code>.\nByte lengths for the different items that could appear in …\nByte lengths for the different items that could appear in …\nIndex for an entry in a ns table.\nReturn type for [<code>Payload::ns_iter</code>].\nRaw binary data for a single namespace’s payload.\nBuild an individual namespace payload one transaction at a …\nByte length of a namespace payload.\nIndex range for a namespace payload inside a block payload.\nProof of correctness for namespace payload bytes in a …\nRaw binary data for a namespace table.\nReturn type for <code>NsTable::validate</code>.\nNumber of entries in a namespace table.\nNumber of txs in a namespace.\nByte range for the part of a tx table that declares the …\nThe part of a tx table that declares the number of txs in …\nRaw payload data for an entire block.\nByte length of a block payload, which includes all …\nIndex for an entry in a tx table.\nA transaction’s payload data.\nByte range for a transaction’s payload data.\nProof of correctness for transaction bytes in a block.\nEntries from a tx table in a namespace for use in a …\nByte range for entries from a tx table for use in a …\nAdd an entry to the namespace table.\nAdd a transaction’s payload to this namespace\nAccess the underlying index range for this namespace …\nConvert a <code>NsPayloadBytesRange</code> into a range that’s …\nReturn the byte length of this namespace.\nByte length of a single namespace table entry.\nReturn all transactions in this namespace. The namespace …\nReturn all transactions in the namespace whose payload is …\nReturn a transaction from this namespace. Set its …\nSearch the namespace table for the ns_index belonging to …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nNeed a sync version of <code>BlockPayload::from_transactions</code> in …\nExtract payload byte length from a <code>VidCommon</code> and construct …\nByte length of a namespace table header.\nDoes the <code>index</code>th entry exist in the namespace table?\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nSerialize to bytes and consume self.\nSerialize to bytes and consume self.\nIs the payload byte length declared in a <code>VidCommon</code> equal …\nIterator over all transactions in this namespace.\nIterator over all unique namespaces in the namespace table.\nPrivate helper\nNumber of entries in the namespace table.\nReturns the minimum of:\nTODO restrict visibility?\nReturns the <code>Transaction</code> indicated by <code>index</code>, along with a …\nReturns the payload bytes for the <code>index</code>th namespace, along …\nConvenience wrapper for <code>Self::read_ns_payload</code>.\nRead subslice range for the <code>index</code>th namespace from the …\nRead and parse bytes from the ns payload.\nRead the namespace id from the <code>index</code>th entry from the …\nLike <code>Self::read_ns_id</code> except <code>index</code> is not checked. Use …\nRead the namespace offset from the <code>index</code>th entry from the …\nRead the number of namespaces declared in the namespace …\nPrivate helper. (Could be pub if desired.)\nLike <code>QueryablePayload::transaction_with_proof</code> except …\nPrivate helper\nByte length of a single tx table entry.\nByte length of a tx table header.\nAre the bytes of this <code>NsTable</code> uncorrupted?\nHelper for <code>NsTable::validate</code>, used in our custom <code>serde</code> …\nVerify a <code>TxProof</code> for <code>tx</code> against a payload commitment. …\nVerify a <code>NsProof</code> against a payload commitment. Returns <code>None</code>…\nGlobal variables for an Espresso blockchain.\nMinimum fee in WEI per byte of payload\nEspresso chain ID\nFee contract address on L1.\nAccount that receives sequencing fees.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nMaximum size in bytes of a block\nA proof of the balance of an account in the fee ledger.\n<code>FeeInfo</code> holds data related to builder fees.\nReturn inner <code>Address</code>\nReturn byte slice representation of inner <code>Address</code> type\nThe minimum fee paid by the given builder account for a …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet a <code>Vec&lt;FeeInfo&gt;</code> from <code>Vec&lt;BuilderFee&gt;</code>\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturn array containing underlying bytes of inner <code>Address</code> …\nA header is like a [<code>Block</code>] with the body replaced by a …\nA commitment to a ChainConfig or a full ChainConfig.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nUpgrade based on unix timestamp.\nRepresents an upgrade based on time (unix timestamp).\nRepresents a general upgrade with mode and type.\nRepresents the specific type of upgrade.\nRepresents the specific type of upgrade.\nUpgrade based on view.\nRepresents an upgrade based on view.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe mode of the upgrade (time-based or view-based).\nthe earliest unix timestamp in which the node can propose …\nthe earliest view in which the node can propose an upgrade\nThe timestamp at which voting for the upgrade proposal …\nThe view at which voting for the upgrade proposal starts\ntimestamp after which the node stops proposing an upgrade\nview after which the node stops proposing an upgrade\nThe timestamp at which voting for the upgrade proposal …\nThe view at which voting for the upgrade proposal stops\nThe type of the upgrade.\nAn Http Provider and configuration to interact with the L1.\nMaximum number of L1 blocks that can be scanned for events …\nThe snapshot also includes information about the latest …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nProxy to <code>Provider.get_block_number</code>.\nProxy to <code>get_finalized_block</code>.\nGet fee info for each <code>Deposit</code> occurring between <code>prev</code> and …\nThe relevant snapshot of the L1 includes a reference to …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nInstantiate an <code>L1Client</code> for a given <code>Url</code>.\n<code>Provider</code> from <code>ethers-provider</code>.\nGet a snapshot from the l1.\nGet information about the given block.\nR value\nS Value\nV value\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nUseful for when we want to test size of transaction(s)\nA transaction to bid for the sequencing rights of a …\nA transaction body holding data required for bid …\nGlobal variables for an Espresso blockchain.\nWrapper enum for Full Network Transactions. Each …\nA header is like a [<code>Block</code>] with the body replaced by a …\nMethods for use w/ Vec\nA commitment to a ChainConfig or a full ChainConfig.\nThe results of an Auction\nAccount responsible for the signature\nMinimum fee in WEI per byte of payload\nThe bid amount designated in Wei.  This is different than …\nAccount that receives sequencing bids.\nA commitment to a ChainConfig or a full ChainConfig.\nEspresso chain ID\nFee contract address on L1.\nAccount that receives sequencing fees.\nFee to be sequenced in the network.  Different than the …\nMaximum size in bytes of a block\nThe set of namespace ids the sequencer is bidding for\nA list of reserve sequencers being used\nThe URL the HotShot leader will use to request a bundle …\nThe slot this bid is for\nview number the results are for\nA list of the bid txs that won\nA transaction to bid for the sequencing rights of a …\nA transaction body holding data required for bid …\nWrapper enum for Full Network Transactions. Each …\nThe results of an Auction\nGet account submitting the bid\nget bid account\nAccount responsible for the signature\nGet amount of bid\nget bid amount\nThe bid amount designated in Wei.  This is different than …\nReturn the body of the transaction\nCharge Bid. Only winning bids are charged in JIT.\nExecute <code>BidTx</code>.\nProxy for <code>execute</code> method of each transaction variant.\nFetch the auction results.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nget gas price\nFee to be sequenced in the network.  Different than the …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe set of namespace ids the sequencer is bidding for\nConstruct a new <code>BidTxBody</code>.\nA list of reserve sequencers being used\nSign <code>BidTxBody</code> and return the signature.\nSign Body and return a <code>BidTx</code>. This is the expected way to …\nGet the cloned <code>url</code> field.\nGet the <code>url</code> field from the body.\nThe URL the HotShot leader will use to request a bundle …\nCryptographic signature verification\nget the view number\nget the view number\nThe slot this bid is for\nview number the results are for\nA list of the bid txs that won\nInstantiate a <code>BidTxBody</code> containing the values of <code>self</code> with …\nInstantiate a <code>BidTx</code> containing the values of <code>self</code> with a …\nGlobal variables for an Espresso blockchain.\nA commitment to a ChainConfig or a full ChainConfig.\nMinimum fee in WEI per byte of payload\nAccount that receives sequencing bids.\nEspresso chain ID\nFee contract address on L1.\nAccount that receives sequencing fees.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nMaximum size in bytes of a block\nMethods for use w/ Vec\nA header is like a [<code>Block</code>] with the body replaced by a …\nA commitment to a ChainConfig or a full ChainConfig.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.")