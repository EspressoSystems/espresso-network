searchState.loadedDescShard("hotshot_stake_table", 0, "This crate contains some stake table implementations for …\nConfiguration file for stake table\nA stake table implementation that’s based on Append-only …\nUtilities to help building a stake table.\nA vector based stake table implementation. The commitment …\nCapacity of a stake table Capacity of the stake table, …\nLocally maintained stake table, generic over public key …\nUpdate the stake table when the epoch number advances, …\nConfig file for stake table\nThe snapshot of stake table at the beginning of the …\nReturns the argument unchanged.\nThe most up-to-date stake table, where the incoming …\nHeight of the underlying merkle tree, determines the …\nUtilities and internals for maintaining a local stake table\nCalls <code>U::from(self)</code>.\nThe stake table used for leader election.\nThe mapping from public keys to their location in the …\nInitiating an empty stake table. Overall capacity is …\nreturns the root of stake table at <code>version</code>\nAlmost uniformly samples a key weighted by its stake from …\nSet the stake withheld by <code>key</code> to be <code>value</code>. Return the …\nHash algorithm used in Merkle tree, using a RATE-3 rescue\nInternal type of Merkle node value(commitment)\nBranch of merkle tree. Set to 3 because we are currently …\nA branch\nA branch\nEmpty\nAn owning iterator over the (key, value) entries of a …\nCommon trait bounds for generic key type <code>K</code> for …\nA leaf\nA leaf\nA succinct commitment for Merkle tree\nPath from a Merkle root to a leaf\nA compressed Merkle node for Merkle path\nAn existential proof\nA persistent merkle tree tailored for the stake table. …\nMerkle tree digest\nReturns the succinct commitment of this subtree\nCompute the root of this Merkle proof.\nReturns the digest of the tree\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nConvert a list of Merkle path branches back to an index\nHeight of a tree\nReturns the index of the given key\nIndex for the given key\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns the public key of the associated stake table …\nImagine that the keys in this subtree is sorted, returns …\nReturns the associated stake table entry, if there’s any.\nReturns a Merkle proof to the given location\ncreate a new merkle tree iterator from a <code>root</code>. This …\nCreates a new merkle commitment\nReturns the total number of keys in this subtree\nA Merkle path for the given leaf\nInsert a new <code>key</code> into the Merkle tree\nSet the stake of <code>key</code> to be <code>value</code>. Return the previous stake\nReturns the stakes withhelded by a public key, None if the …\nReturns the number of leaves\nNumber of leaves\nConvert an index to a list of Merkle path branches\nReturns the total stakes in this subtree\nReturns the height of the tree\nReturns the height of the tree\nThe unvisited key values\nUpdate the stake of the <code>key</code> with …\nReturns the stake amount of the associated stake table …\nVerify the Merkle proof against the provided Merkle …\nthe key\nPosition in tree\nSiblings\nthe value\nchildren\nfield type\nfield type\nthe key\nnumber of keys\ntotal stake\nthe value\nThe number of field elements needed to represent the given …\nA trait that converts into a field element.\nA helper function to compute the quorum threshold given a …\nConvert the given struct into a list of field elements.\nconvert a U256 to a field element.\nLocally maintained stake table, generic over public key …\na snapshot of the stake table\nUpdate the stake table when the epoch number advances, …\nbls keys\nThe mapping from public keys to their location in the …\nupper bound on table size\nHelper function to recompute the stake table commitment …\nConfig file for stake table\nThe snapshot of stake table at the beginning of the …\nCommitment of the stake table snapshot version <code>EpochStart</code> …\nTotal stakes in the snapshot version <code>EpochStart</code>\nReturns the argument unchanged.\nReturns the argument unchanged.\nThe most up-to-date stake table, where the incoming …\nTotal stakes in the most update-to-date stake table\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe stake table used for leader election.\nCommitment of the stake table snapshot version …\nTotal stakes in the snapshot version <code>LastEpochStart</code>\nReturn the index of a given key. Err if the key doesn’t …\nInitiating an empty stake table.\nReturns the stake table state used for voting in the next …\nschnorr\nSet the stake withheld by <code>key</code> to be <code>value</code>. Return the …\namount of stake\nreturns the snapshot version\nReturns the stake table state used for voting\nType for commitment\nBLS verification key as indexing key Signature public …\nSchnorr verification key as auxiliary information …\nReturns the argument unchanged.\nGet the internal of verifying key, namely a curve Point\nCalls <code>U::from(self)</code>.\nConvert the verification key into the affine form.\nThis should be compatible with our legacy implementation.\nSignature verification function")