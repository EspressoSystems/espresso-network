searchState.loadedDescShard("vid", 0, "Verifiable Information Retrieval (VID).\ninvalid args: {0}\nVID commitment type\nContains the error value\nContains the error value\ninternal error: {0}\nContains the success value\nContains the success value\nVID Parameters\nVID Share type\nA glorified <code>bool</code> that leverages compile lints to encourage …\nThe error type for <code>VidScheme</code> methods.\nAlias\nTrait definition for a Verifiable Information Dispersal …\nThis module implements the AVID-M scheme, whose name came …\nCommit to a <code>payload</code> without generating shares.\nDisperse the given <code>payload</code> according to the weights in …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nRecover the payload from the given <code>shares</code>.\nVerify the given VID <code>share</code> against the VID <code>commit</code>.\nCommit type for AVID-M scheme.\nPublic parameters of the AVID-M scheme.\nDummy struct for AVID-M scheme.\nShare type to be distributed among the parties.\nShare type to be distributed among the parties.\nRoot commitment of the Merkle tree.\nThis module configures base fields, Merkle tree, etc for …\nContent of this AvidMShare.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nIndex number of the given share.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nMerkle proof of the content.\nThis file implements the namespaced AvidM scheme.\nConstruct a new <code>AvidMParam</code>.\nShort hand for <code>pad_to_field</code> and <code>raw_encode</code>.\nHelper function. Transform the payload bytes into a list …\nActual share content.\nThe length of payload in bytes.\nHelper: initialize a FFT domain\nRange of this share in the encoded payload.\nHelper function. Let <code>k = recovery_threshold</code> and …\nRecover payload data from shares.\nMinimum collective weights required to recover the …\nSetup an instance for AVID-M scheme\nTotal weights of all storage nodes\nConfiguration of Keccak256 based AVID-M scheme\nConfiguration of Poseidon2 based AVID-M scheme\nConfiguration of Sha256 based AVID-M scheme\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nDigest the raw shares into the element type for Merkle …\nNamespaced commitment type\nNamespaced parameter type\nDummy struct for namespaced AvidM scheme\nNamespaced share for each storage node\nCommit to a payload given namespace table.\nRoot commitment of the Merkle tree.\nActual share content\nReturns the argument unchanged.\nReturns the argument unchanged.\nIndex number of the given share.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nThe list of all namespace commitments\nDisperse a payload according to a distribution table and a …\nThe size of each namespace\nRecover the payload for a given namespace\nReturn the length of underlying payload in bytes\nRecover the entire payload from enough share\nMinimum collective weights required to recover the …\nSetup an instance for AVID-M scheme\nTotal weights of all storage nodes\nVerify a namespaced share\nDeterministic, infallible, invertible iterator adaptor to …\nReturn the number of bytes that can be encoded into a …\nDeterministic, infallible inverse of <code>bytes_to_field</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.")