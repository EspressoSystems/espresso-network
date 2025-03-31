searchState.loadedDescShard("sequencer", 0, "The Sequencer node is generic over the hotshot CommChannel.\nThe address where a CDN marshal is located\nShould probably rename this to “external” or something\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe address to send to other Libp2p nodes to contact us\nThe address to bind to for Libp2p\nThe (optional) bootstrap node addresses for Libp2p. If …\nThe time period that Libp2p message hashes are stored in …\nTime to live for Libp2p fanout peers\nIf enabled newly created messages will always be sent to …\nHow many Libp2p peers we will emit gossip to at each …\nMinimum number of Libp2p peers to emit gossip to during a …\nHow many times we will allow a peer to request the same …\nInitial delay in each Libp2p heartbeat\nThe heartbeat interval\nThe number of past heartbeats to gossip about\nThe number of past heartbeats to remember the full …\nThe time to wait for a Libp2p message requested through …\nThe maximum direct message size\nThe maximum gossip message size\nThe maximum number of IHAVE messages to accept from a …\nThe maximum number of IHAVE messages to accept from a …\nThe maximum number of Libp2p messages we will process in a …\nThe target number of peers in the mesh\nThe maximum number of peers in the mesh\nThe minimum number of peers in the mesh\nThe minimum number of mesh peers that must be outbound\nThe time period that message hashes are stored in the cache\nSequencer node persistence.\nThe address to advertise as our public API’s URL\nUtilities for generating and storing the most recent light …\nSequencer-specific API endpoint handlers.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet the stake table for a given epoch\nGet the stake table for the current epoch if not provided\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nSequencer-specific API options and initialization.\nUpdate loop for query API state.\nProvider for fetching missing data for the query service.\nA data source with sequencer-specific functionality.\nInstantiate a data source from command line options.\nGet the state of the requested <code>account</code>.\nGet the state of the requested <code>accounts</code>.\nGet the blocks Merkle tree frontier.\nGet the state of the requested <code>account</code>.\nGet the stake table for a given epoch\nGet the stake table for  the current epoch if not provided\nCreate a provider for fetching missing data from a list of …\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nOptions for the catchup API module.\nOptions for the config API module.\nOptions for the explorer API module.\nOptions for the Hotshot events streaming API module.\nThe minimal HTTP API.\nOptions for the query API module.\nOptions for the state API module.\nOptions for the status API module.\nOptions for the submission API module.\nAdd a catchup API module.\nAdd a config API module.\nPort that the HTTP Hotshot Event streaming API will use.\nAdd an explorer API module.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nWhether these options will run the query API.\nAdd a Hotshot events streaming API module.\nInitialize the modules for interacting with HotShot.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nMaximum number of concurrent HTTP connections the server …\nPeers for fetching missing data for the query service.\nPort that the HTTP API will use.\nAdd a query API module backed by the file system.\nAdd a query API module backed by a Postgres database.\nStart the server.\nAdd a status API module.\nAdd a submit API module.\nDefault options for running a web server on the given port.\nDefault options for running a web server on the given port.\nGet the dependencies needed to apply the STF to the given …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nDisable catchup entirely.\nA score of a catchup peer, based on our interactions with …\nAdd a chain config preimage which can be fetched by hash …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet the state of the requested <code>accounts</code>.\nGet the blocks Merkle tree frontier.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nA catchup implementation that falls back to a remote …\nThe consensus handle\nThe sequencer context contains a consensus handle and …\nReturn a reference to the underlying consensus handle.\nAllow this node to continue participating in consensus …\nNumber of blocks in an epoch, zero means there are no …\nStream consensus events.\nget event streamer\nevents streamer to stream hotshot events to external …\nReturns the argument unchanged.\nReturns the argument unchanged.\nThe consensus handle\nInternal reference to the underlying [<code>SystemContext</code>]\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nWait for consensus to complete.\nWait for all background tasks to complete.\nMemberships used by consensus\nNetworks used by the instance of hotshot\nGet the network config\nConstructor\nThe request-response protocol\nStop participating in consensus.\nStop all background tasks.\nSpawn a background task attached to this context.\nSpawn a background task attached to this <code>TaskList</code>.\nSpawn a short-lived background task attached to this …\nSpawn a short-lived background task attached to this …\nStart participating in consensus.\nReturn a reference to the consensus state signer.\nContext for generating state signatures.\nBackground tasks to shut down when the node is dropped.\nWait for a signal from the orchestrator before starting …\nAn orchestrator to wait for before starting consensus.\nAdd a list of tasks to the given context.\nThe external event handler\nAn external message that can be sent to or received from a …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nHandles an event\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCreates a new <code>ExternalEventHandler</code> with the given network\nThe main loop for sending outbound messages.\nThe type phantom\nThe sender to the request-response protocol\nComplete block info.\nGenesis of an Espresso chain.\nAn L1 block from which an Espresso chain should start …\nAn L1 block number to sync from.\nInitial configuration of an Espresso stake table.\nA time from which to start syncing L1 blocks.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe broker definition for the Push CDN. Uses the TCP …\nThe client definition for the Push CDN. Uses the Quic …\nThe DA topic\nThe global topic\nThe production run definition for the Push CDN. Uses the …\nThe testing run definition for the Push CDN. Uses the real …\nThe enum for the topics we can subscribe to in the Push CDN\nThe user definition for the Push CDN. Uses the Quic …\nThe (parallel, TCP) user definition for the Push CDN. Uses …\nA wrapped <code>SignatureKey</code>. We need to implement the Push CDN…\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nSign a message of arbitrary data and return the serialized …\nVerify a message of arbitrary data and return the result\nSplit off the peer ID from a multiaddress, returning the …\nRun the state catchup API module.\nRun the config API module.\nRun the explorer API module.\nRun the hotshot events API module.\nRun an HTTP server.\nIdentity represents identifying information concerning the …\nRun the query API module.\nRun the status API module.\nAlias for storage-fs.\nUse the file system for persistent storage.\nUse a Postgres database for persistent storage.\nRun the transaction submission API module.\nAdd this as an optional module. Return the next optional …\nAPI path of marketplace-solver auction results\nURL of the Auction Results Solver\nExponential backoff for fetching missing state from peers.\nThe socket address of the HotShot CDN’s main entry point …\nPeer nodes use to fetch missing config\nURL of generic builder\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nPath to TOML file containing genesis state.\nget_default_node_type returns the current public facing …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nWhether or not we are a DA node.\nPath to file containing private keys.\nConfiguration for the L1 client.\nUrl we will use for RPC communication with L1.\nThe address we advertise to other nodes as being a Libp2p …\nThe address to bind to for Libp2p (in <code>host:port</code> form)\nA comma-separated list of Libp2p multiaddresses to use as …\nThe time period that Libp2p message hashes are stored in …\nTime to live for Libp2p fanout peers\nIf enabled newly created messages will always be sent to …\nHow many Libp2p peers we will emit gossip to at each …\nMinimum number of Libp2p peers to emit gossip to during a …\nHow many times we will allow a Libp2p peer to request the …\nInitial delay in each Libp2p heartbeat\nTime between each Libp2p heartbeat\nNumber of past heartbeats to gossip about on Libp2p\nNumber of heartbeats to keep in the Libp2p <code>memcache</code>\nTime to wait for a Libp2p message requested through IWANT …\nThe maximum number of bytes we will send in a single …\nThe maximum number of bytes we will send in a single …\nThe maximum number of messages to include in a Libp2p …\nThe maximum number of IHAVE messages to accept from a …\nThe maximum number of Libp2p messages we will process in a …\nTarget number of peers for the Libp2p mesh network\nMaximum number of peers in the Libp2p mesh network before …\nMinimum number of peers in the Libp2p mesh network before …\nMinimum number of outbound Libp2p peers in the mesh …\nLibp2p published message ids time cache duration\nAPI path of marketplace-solver\nAdd optional modules to the service.\nAdd more optional modules.\nURL of the HotShot orchestrator.\nPrivate staking key.\nPrivate state signing key.\nThe URL we advertise to other nodes as being for our …\nPeer nodes use to fetch missing state\nURL of the Light Client State Relay Server\nOptions for file system backed persistence.\nFile system backed persistence.\nNumber of views to retain in consensus storage before data …\nPath to a directory containing decided leaves.\nGet all paths under <code>dir</code> whose name is of the form .txt. …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGenerate events based on persisted decided leaves.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe path from previous versions where there was only a …\nUpdate a <code>NetworkConfig</code> that may have originally been …\nStorage path for persistent data.\nOverwrite a file if a condition is met.\nGet all paths under <code>dir</code> whose name is of the form .txt.\nPruning parameters for ephemeral consensus storage.\nOptions for database-backed persistence, supporting both …\nPostgres-backed persistence.\nOptions for Postgres-backed persistence.\nPruning parameters.\nThe minimum delay between active fetches in a stream.\nDisable pruning and reconstruct previously pruned data.\nBatch size for pruning. This is the number of blocks data …\nThe minimum delay between loading chunks in a stream.\nThe maximum lifetime of a database connection.\nPruning parameters for ephemeral consensus storage.\nName of database to connect to.\nSpecifies the maximum number of concurrent fetch requests …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nHostname for the remote Postgres database server.\nThe maximum idle time of a database connection.\nInterval for running the pruner.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nTurns on leaf only data storage\nThe maximum number of database connections to maintain at …\nMaximum disk usage (in basis points).\nEnsure the <code>leaf_hash</code> column is populated for all existing …\nThe minimum number of database connections to maintain at …\nMinimum retention period. Data is retained for at least …\nMinimum number of views to try to retain in consensus …\nPassword for Postgres user.\nBase directory for the SQLite database. The SQLite file …\nPort for the remote Postgres database server.\nThis will enable the pruner and set the default pruning …\nPruning parameters.\nThreshold for pruning, specified in bytes. If the disk …\nTarget retention period. Data older than this is pruned to …\nNumber of views to try to retain in consensus storage …\nAmount (in bytes) of data to retain in consensus storage …\nDatabase URI for Postgres or SQLite.\nUse TLS for an encrypted connection to the database.\nPostgres user to connect as.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThis file contains the <code>DataSource</code> trait. This trait allows …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nA wrapper type that we will implement the <code>Sender</code> trait for\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nThe outermost request type. This an enum that contains all …\nThe outermost response type. This an enum that contains …\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCapacity for the in memory signature storage.\nType for stake table commitment\nA rolling in-memory storage for the most recent light …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturn a signature of a light client state at given height.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nA relay server that’s collecting and serving the light …\nThe state relay server url\nKey for signing a new light client state\nSign the light client state at given height and store it.\nThe most recent light client state signatures\nCommitment for current fixed stake table\nHelper function for stake table commitment\nKey for verifying a light client state\nConnect to the given state relay server to send signed …\nconfigurability options for the web server\nState that checks the light client state update and the …\npath to API\nSignatures bundles for each block height\nSet up APIs for relay server\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet the latest available signatures bundle.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nStake table\nThe latest state signatures bundle whose total weight …\nThe block height of the latest available state signature …\nPost a signature to the relay server\nA ordered queue of block heights, used for garbage …\nshutdown signal\nMinimum weight to form an available state signature bundle")