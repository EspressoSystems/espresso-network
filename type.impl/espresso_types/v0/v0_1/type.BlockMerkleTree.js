(function() {
    var type_impls = Object.fromEntries([["espresso_types",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-MerklizedState%3CSeqTypes,+%7B+Self::ARITY+%7D%3E-for-LightWeightMerkleTree%3CCommitment%3CHeader%3E,+Sha3Digest,+u64,+3,+Sha3Node%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1102-1132\">Source</a><a href=\"#impl-MerklizedState%3CSeqTypes,+%7B+Self::ARITY+%7D%3E-for-LightWeightMerkleTree%3CCommitment%3CHeader%3E,+Sha3Digest,+u64,+3,+Sha3Node%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html\" title=\"trait hotshot_query_service::merklized_state::data_source::MerklizedState\">MerklizedState</a>&lt;<a class=\"struct\" href=\"espresso_types/v0/struct.SeqTypes.html\" title=\"struct espresso_types::v0::SeqTypes\">SeqTypes</a>, { Self::ARITY }&gt; for <a class=\"type\" href=\"espresso_types/v0/v0_1/type.BlockMerkleTree.html\" title=\"type espresso_types::v0::v0_1::BlockMerkleTree\">BlockMerkleTree</a></h3></section></summary><div class=\"impl-items\"><section id=\"associatedtype.Key\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1103\">Source</a><a href=\"#associatedtype.Key\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.Key\" class=\"associatedtype\">Key</a> = &lt;LightWeightMerkleTree&lt;Commitment&lt;<a class=\"enum\" href=\"espresso_types/v0/enum.Header.html\" title=\"enum espresso_types::v0::Header\">Header</a>&gt;, Sha3Digest, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.u64.html\">u64</a>, 3, Sha3Node&gt; as MerkleTreeScheme&gt;::Index</h4></section><section id=\"associatedtype.Entry\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1104\">Source</a><a href=\"#associatedtype.Entry\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.Entry\" class=\"associatedtype\">Entry</a> = Commitment&lt;<a class=\"enum\" href=\"espresso_types/v0/enum.Header.html\" title=\"enum espresso_types::v0::Header\">Header</a>&gt;</h4></section><section id=\"associatedtype.T\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1105\">Source</a><a href=\"#associatedtype.T\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.T\" class=\"associatedtype\">T</a> = Sha3Node</h4></section><section id=\"associatedtype.Commit\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1106\">Source</a><a href=\"#associatedtype.Commit\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.Commit\" class=\"associatedtype\">Commit</a> = &lt;LightWeightMerkleTree&lt;Commitment&lt;<a class=\"enum\" href=\"espresso_types/v0/enum.Header.html\" title=\"enum espresso_types::v0::Header\">Header</a>&gt;, Sha3Digest, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.u64.html\">u64</a>, 3, Sha3Node&gt; as MerkleTreeScheme&gt;::Commitment</h4></section><section id=\"associatedtype.Digest\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1107\">Source</a><a href=\"#associatedtype.Digest\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.Digest\" class=\"associatedtype\">Digest</a> = Sha3Digest</h4></section><details class=\"toggle method-toggle\" open><summary><section id=\"method.state_type\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1109-1111\">Source</a><a href=\"#method.state_type\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#tymethod.state_type\" class=\"fn\">state_type</a>() -&gt; &amp;'static <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.str.html\">str</a></h4></section></summary><div class='docblock'>Retrieves the name of the state being queried.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.header_state_commitment_field\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1113-1115\">Source</a><a href=\"#method.header_state_commitment_field\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#tymethod.header_state_commitment_field\" class=\"fn\">header_state_commitment_field</a>() -&gt; &amp;'static <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.str.html\">str</a></h4></section></summary><div class='docblock'>Retrieves the field in the header containing the Merkle tree commitment\nfor the state implementing this trait.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.tree_height\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1117-1119\">Source</a><a href=\"#method.tree_height\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#tymethod.tree_height\" class=\"fn\">tree_height</a>() -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a></h4></section></summary><div class='docblock'>Get the height of the tree</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.insert_path\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/espresso_types/v0/impls/state.rs.html#1121-1131\">Source</a><a href=\"#method.insert_path\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#tymethod.insert_path\" class=\"fn\">insert_path</a>(\n    &amp;mut self,\n    key: Self::<a class=\"associatedtype\" href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.Key\" title=\"type hotshot_query_service::merklized_state::data_source::MerklizedState::Key\">Key</a>,\n    proof: &amp;MerkleProof&lt;Self::<a class=\"associatedtype\" href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.Entry\" title=\"type hotshot_query_service::merklized_state::data_source::MerklizedState::Entry\">Entry</a>, Self::<a class=\"associatedtype\" href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.Key\" title=\"type hotshot_query_service::merklized_state::data_source::MerklizedState::Key\">Key</a>, Self::<a class=\"associatedtype\" href=\"hotshot_query_service/merklized_state/data_source/trait.MerklizedState.html#associatedtype.T\" title=\"type hotshot_query_service::merklized_state::data_source::MerklizedState::T\">T</a>, { Self::ARITY }&gt;,\n) -&gt; <a class=\"type\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/type.Result.html\" title=\"type anyhow::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class='docblock'>Insert a forgotten path into the tree.</div></details></div></details>","MerklizedState<SeqTypes, { Self::ARITY }>","espresso_types::v0::v0_1::state::BlockMerkleCommitment"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[7976]}