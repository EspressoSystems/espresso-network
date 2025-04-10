(function() {
    var type_impls = Object.fromEntries([["hotshot_query_service",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-BlockHeader%3CTYPES%3E-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#317-324\">Source</a><a href=\"#impl-BlockHeader%3CTYPES%3E-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; BlockHeader&lt;TYPES&gt; for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a><div class=\"where\">where\n    TYPES: NodeType&lt;BlockHeader = <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a>, BlockPayload = <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockPayload.html\" title=\"struct hotshot_example_types::block_types::TestBlockPayload\">TestBlockPayload</a>, InstanceState = <a class=\"struct\" href=\"hotshot_example_types/state_types/struct.TestInstanceState.html\" title=\"struct hotshot_example_types::state_types::TestInstanceState\">TestInstanceState</a>, AuctionResult = <a class=\"struct\" href=\"hotshot_example_types/auction_results_provider_types/struct.TestAuctionResult.html\" title=\"struct hotshot_example_types::auction_results_provider_types::TestAuctionResult\">TestAuctionResult</a>&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Error\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#326\">Source</a><a href=\"#associatedtype.Error\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Error</a> = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/convert/enum.Infallible.html\" title=\"enum core::convert::Infallible\">Infallible</a></h4></section></summary><div class='docblock'>Error type for this type of block header</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.new_legacy\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#328-338\">Source</a><a href=\"#method.new_legacy\" class=\"anchor\">§</a><h4 class=\"code-header\">async fn <a class=\"fn\">new_legacy</a>(\n    _parent_state: &amp;&lt;TYPES as NodeType&gt;::ValidatedState,\n    instance_state: &amp;&lt;&lt;TYPES as NodeType&gt;::ValidatedState as ValidatedState&lt;TYPES&gt;&gt;::Instance,\n    parent_leaf: &amp;<a class=\"struct\" href=\"hotshot_query_service/struct.Leaf2.html\" title=\"struct hotshot_query_service::Leaf2\">Leaf2</a>&lt;TYPES&gt;,\n    payload_commitment: VidCommitment,\n    builder_commitment: BuilderCommitment,\n    metadata: &lt;&lt;TYPES as NodeType&gt;::BlockPayload as BlockPayload&lt;TYPES&gt;&gt;::Metadata,\n    _builder_fee: BuilderFee&lt;TYPES&gt;,\n    _version: Version,\n    _view_number: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.u64.html\">u64</a>,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a>, &lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a> as BlockHeader&lt;TYPES&gt;&gt;::Error&gt;</h4></section></summary><div class='docblock'>Build a header with the parent validate state, instance-level state, parent leaf, payload\nand builder commitments, and metadata. This is only used in pre-marketplace versions</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.new_marketplace\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#348-359\">Source</a><a href=\"#method.new_marketplace\" class=\"anchor\">§</a><h4 class=\"code-header\">async fn <a class=\"fn\">new_marketplace</a>(\n    _parent_state: &amp;&lt;TYPES as NodeType&gt;::ValidatedState,\n    instance_state: &amp;&lt;&lt;TYPES as NodeType&gt;::ValidatedState as ValidatedState&lt;TYPES&gt;&gt;::Instance,\n    parent_leaf: &amp;<a class=\"struct\" href=\"hotshot_query_service/struct.Leaf2.html\" title=\"struct hotshot_query_service::Leaf2\">Leaf2</a>&lt;TYPES&gt;,\n    payload_commitment: VidCommitment,\n    builder_commitment: BuilderCommitment,\n    metadata: &lt;&lt;TYPES as NodeType&gt;::BlockPayload as BlockPayload&lt;TYPES&gt;&gt;::Metadata,\n    _builder_fee: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;BuilderFee&lt;TYPES&gt;&gt;,\n    _view_number: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.u64.html\">u64</a>,\n    _auction_results: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;TYPES as NodeType&gt;::AuctionResult&gt;,\n    _version: Version,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a>, &lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a> as BlockHeader&lt;TYPES&gt;&gt;::Error&gt;</h4></section></summary><div class='docblock'>Build a header with the parent validate state, instance-level state, parent leaf, payload\nand builder commitments, metadata, and auction results. This is only used in post-marketplace\nversions</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.genesis\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#369-374\">Source</a><a href=\"#method.genesis\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">genesis</a>(\n    _instance_state: &amp;&lt;&lt;TYPES as NodeType&gt;::ValidatedState as ValidatedState&lt;TYPES&gt;&gt;::Instance,\n    payload_commitment: VidCommitment,\n    builder_commitment: BuilderCommitment,\n    _metadata: &lt;&lt;TYPES as NodeType&gt;::BlockPayload as BlockPayload&lt;TYPES&gt;&gt;::Metadata,\n) -&gt; <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h4></section></summary><div class='docblock'>Build the genesis header, payload, and metadata.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.block_number\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#389\">Source</a><a href=\"#method.block_number\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">block_number</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.u64.html\">u64</a></h4></section></summary><div class='docblock'>Get the block number.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.payload_commitment\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#393\">Source</a><a href=\"#method.payload_commitment\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">payload_commitment</a>(&amp;self) -&gt; VidCommitment</h4></section></summary><div class='docblock'>Get the payload commitment.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.metadata\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#397\">Source</a><a href=\"#method.metadata\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">metadata</a>(\n    &amp;self,\n) -&gt; &amp;&lt;&lt;TYPES as NodeType&gt;::BlockPayload as BlockPayload&lt;TYPES&gt;&gt;::Metadata</h4></section></summary><div class='docblock'>Get the metadata.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.builder_commitment\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#401\">Source</a><a href=\"#method.builder_commitment\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">builder_commitment</a>(&amp;self) -&gt; BuilderCommitment</h4></section></summary><div class='docblock'>Get the builder commitment</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_auction_results\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#405\">Source</a><a href=\"#method.get_auction_results\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">get_auction_results</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;TYPES as NodeType&gt;::AuctionResult&gt;</h4></section></summary><div class='docblock'>Get the results of the auction for this Header. Only used in post-marketplace versions</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_light_client_state\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#409\">Source</a><a href=\"#method.get_light_client_state\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">get_light_client_state</a>(\n    &amp;self,\n    view: &lt;TYPES as NodeType&gt;::View,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;GenericLightClientState&lt;Fp&lt;MontBackend&lt;FrConfig, 4&gt;, 4&gt;&gt;, <a class=\"struct\" href=\"hotshot_query_service/data_source/sql/struct.Error.html\" title=\"struct hotshot_query_service::data_source::sql::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Get the light client state</div></details></div></details>","BlockHeader<TYPES>","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#impl-Clone-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.86.0/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Committable-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#418\">Source</a><a href=\"#impl-Committable-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl Committable for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.commit\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#419\">Source</a><a href=\"#method.commit\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">commit</a>(&amp;self) -&gt; Commitment&lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a>&gt;</h4></section></summary><div class='docblock'>Create a binding commitment to <code>self</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.tag\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#432\">Source</a><a href=\"#method.tag\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">tag</a>() -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a></h4></section></summary><div class='docblock'>Tag that should be used when serializing commitments to this type. <a>Read more</a></div></details></div></details>","Committable","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#impl-Debug-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#301\">Source</a><a href=\"#impl-Default-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#302\">Source</a><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/1.86.0/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Deserialize%3C'de%3E-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#impl-Deserialize%3C'de%3E-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'de&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#method.deserialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserialize.html#tymethod.deserialize\" class=\"fn\">deserialize</a>&lt;__D&gt;(\n    __deserializer: __D,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a>, &lt;__D as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserializer.html#associatedtype.Error\" title=\"type serde::de::Deserializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __D: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;,</div></h4></section></summary><div class='docblock'>Deserialize this value from the given Serde deserializer. <a href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserialize.html#tymethod.deserialize\">Read more</a></div></details></div></details>","Deserialize<'de>","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Hash-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#impl-Hash-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#method.hash\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hash.html#tymethod.hash\" class=\"fn\">hash</a>&lt;__H&gt;(&amp;self, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;mut __H</a>)<div class=\"where\">where\n    __H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,</div></h4></section></summary><div class='docblock'>Feeds this value into the given <a href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hash.html#tymethod.hash\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash_slice\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.3.0\">1.3.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.86.0/src/core/hash/mod.rs.html#235-237\">Source</a></span><a href=\"#method.hash_slice\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hash.html#method.hash_slice\" class=\"fn\">hash_slice</a>&lt;H&gt;(data: &amp;[Self], state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;mut H</a>)<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Feeds a slice of this type into the given <a href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.86.0/core/hash/trait.Hash.html#method.hash_slice\">Read more</a></div></details></div></details>","Hash","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#impl-PartialEq-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>self</code> and <code>other</code> values to be equal, and is used by <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.86.0/src/core/cmp.rs.html#261\">Source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>!=</code>. The default implementation is almost always sufficient,\nand should not be overridden without very good reason.</div></details></div></details>","PartialEq","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#impl-Serialize-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;__S&gt;(\n    &amp;self,\n    __serializer: __S,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, &lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __S: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</div></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#273\">Source</a><a href=\"#impl-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><section id=\"method.new\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#274-279\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html#tymethod.new\" class=\"fn\">new</a>&lt;TYPES&gt;(\n    parent_leaf: &amp;<a class=\"struct\" href=\"hotshot_query_service/struct.Leaf2.html\" title=\"struct hotshot_query_service::Leaf2\">Leaf2</a>&lt;TYPES&gt;,\n    payload_commitment: VidCommitment,\n    builder_commitment: BuilderCommitment,\n    metadata: <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestMetadata.html\" title=\"struct hotshot_example_types::block_types::TestMetadata\">TestMetadata</a>,\n) -&gt; <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a><div class=\"where\">where\n    TYPES: NodeType&lt;BlockHeader = <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a>&gt;,</div></h4></section></div></details>",0,"hotshot_query_service::testing::mocks::MockHeader"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-TestableDelay-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#438\">Source</a><a href=\"#impl-TestableDelay-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"hotshot_example_types/testable_delay/trait.TestableDelay.html\" title=\"trait hotshot_example_types::testable_delay::TestableDelay\">TestableDelay</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.run_delay_settings_from_config\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#439\">Source</a><a href=\"#method.run_delay_settings_from_config\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_example_types/testable_delay/trait.TestableDelay.html#tymethod.run_delay_settings_from_config\" class=\"fn\">run_delay_settings_from_config</a>&lt;'life0, 'async_trait&gt;(\n    delay_config: &amp;'life0 <a class=\"struct\" href=\"hotshot_example_types/testable_delay/struct.DelayConfig.html\" title=\"struct hotshot_example_types::testable_delay::DelayConfig\">DelayConfig</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,</div></h4></section></summary><div class='docblock'>Look for settings in the config and run it</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.handle_async_delay\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/testable_delay.rs.html#86\">Source</a><a href=\"#method.handle_async_delay\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_example_types/testable_delay/trait.TestableDelay.html#method.handle_async_delay\" class=\"fn\">handle_async_delay</a>&lt;'life0, 'async_trait&gt;(\n    settings: &amp;'life0 <a class=\"struct\" href=\"hotshot_example_types/testable_delay/struct.DelaySettings.html\" title=\"struct hotshot_example_types::testable_delay::DelaySettings\">DelaySettings</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,</div></h4></section></summary><div class='docblock'>Add a delay from settings</div></details></div></details>","TestableDelay","hotshot_query_service::testing::mocks::MockHeader"],["<section id=\"impl-Eq-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#impl-Eq-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section>","Eq","hotshot_query_service::testing::mocks::MockHeader"],["<section id=\"impl-StructuralPartialEq-for-TestBlockHeader\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#257\">Source</a><a href=\"#impl-StructuralPartialEq-for-TestBlockHeader\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.StructuralPartialEq.html\" title=\"trait core::marker::StructuralPartialEq\">StructuralPartialEq</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestBlockHeader.html\" title=\"struct hotshot_example_types::block_types::TestBlockHeader\">TestBlockHeader</a></h3></section>","StructuralPartialEq","hotshot_query_service::testing::mocks::MockHeader"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[36264]}