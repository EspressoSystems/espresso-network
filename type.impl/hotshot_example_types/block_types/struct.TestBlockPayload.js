(function() {
    var type_impls = Object.fromEntries([["hotshot_query_service",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-BlockPayload%3CTYPES%3E-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-BlockPayload%3CTYPES%3E-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt; for TestBlockPayload<div class=\"where\">where\n    TYPES: <a class=\"trait\" href=\"hotshot_types/traits/node_implementation/trait.NodeType.html\" title=\"trait hotshot_types::traits::node_implementation::NodeType\">NodeType</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Error\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Error\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Error\" class=\"associatedtype\">Error</a> = <a class=\"enum\" href=\"hotshot_types/data/enum.BlockError.html\" title=\"enum hotshot_types::data::BlockError\">BlockError</a></h4></section></summary><div class='docblock'>The error type for this type of block</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Instance\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Instance\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Instance\" class=\"associatedtype\">Instance</a> = TestInstanceState</h4></section></summary><div class='docblock'>The type of the instance-level state this state is associated with</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Transaction\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Transaction\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Transaction\" class=\"associatedtype\">Transaction</a> = TestTransaction</h4></section></summary><div class='docblock'>The type of the transitions we are applying</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Metadata\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Metadata\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Metadata\" class=\"associatedtype\">Metadata</a> = TestMetadata</h4></section></summary><div class='docblock'>Data created during block building which feeds into the block header</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.ValidatedState\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.ValidatedState\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.ValidatedState\" class=\"associatedtype\">ValidatedState</a> = TestValidatedState</h4></section></summary><div class='docblock'>Validated State</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.from_transactions\" class=\"method trait-impl\"><a href=\"#method.from_transactions\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#tymethod.from_transactions\" class=\"fn\">from_transactions</a>&lt;'life0, 'life1, 'async_trait&gt;(\n    transactions: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/iter/traits/collect/trait.IntoIterator.html\" title=\"trait core::iter::traits::collect::IntoIterator\">IntoIterator</a>&lt;Item = &lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Transaction\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Transaction\">Transaction</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait,\n    _validated_state: &amp;'life0 &lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.ValidatedState\" title=\"type hotshot_types::traits::block_contents::BlockPayload::ValidatedState\">ValidatedState</a>,\n    _instance_state: &amp;'life1 &lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Instance\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Instance\">Instance</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;(TestBlockPayload, &lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Metadata\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Metadata\">Metadata</a>), &lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Error\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Error\">Error</a>&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    'life1: 'async_trait,\n    TestBlockPayload: 'async_trait,</div></h4></section></summary><div class='docblock'>Build a payload and associated metadata with the transactions.\nThis function is asynchronous because it may need to request updated state from the peers via GET requests. <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#tymethod.from_transactions\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.from_bytes\" class=\"method trait-impl\"><a href=\"#method.from_bytes\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#tymethod.from_bytes\" class=\"fn\">from_bytes</a>(\n    encoded_transactions: &amp;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>],\n    _metadata: &amp;&lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Metadata\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Metadata\">Metadata</a>,\n) -&gt; TestBlockPayload</h4></section></summary><div class='docblock'>Build a payload with the encoded transaction bytes, metadata,\nand the associated number of VID storage nodes</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.empty\" class=\"method trait-impl\"><a href=\"#method.empty\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#tymethod.empty\" class=\"fn\">empty</a>() -&gt; (TestBlockPayload, &lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Metadata\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Metadata\">Metadata</a>)</h4></section></summary><div class='docblock'>Build the payload and metadata for genesis/null block.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.builder_commitment\" class=\"method trait-impl\"><a href=\"#method.builder_commitment\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#tymethod.builder_commitment\" class=\"fn\">builder_commitment</a>(\n    &amp;self,\n    _metadata: &amp;&lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Metadata\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Metadata\">Metadata</a>,\n) -&gt; <a class=\"struct\" href=\"hotshot_types/utils/struct.BuilderCommitment.html\" title=\"struct hotshot_types::utils::BuilderCommitment\">BuilderCommitment</a></h4></section></summary><div class='docblock'>Generate commitment that builders use to sign block options.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.transactions\" class=\"method trait-impl\"><a href=\"#method.transactions\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#tymethod.transactions\" class=\"fn\">transactions</a>&lt;'a&gt;(\n    &amp;'a self,\n    _metadata: &amp;'a &lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Metadata\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Metadata\">Metadata</a>,\n) -&gt; impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a>&lt;Item = &lt;TestBlockPayload as <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html\" title=\"trait hotshot_types::traits::block_contents::BlockPayload\">BlockPayload</a>&lt;TYPES&gt;&gt;::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Transaction\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Transaction\">Transaction</a>&gt; + 'a</h4></section></summary><div class='docblock'>Get the transactions in the payload.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.transaction_commitments\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_types/traits/block_contents.rs.html#109-112\">Source</a><a href=\"#method.transaction_commitments\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#method.transaction_commitments\" class=\"fn\">transaction_commitments</a>(\n    &amp;self,\n    metadata: &amp;Self::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Metadata\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Metadata\">Metadata</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;Commitment&lt;Self::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Transaction\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Transaction\">Transaction</a>&gt;&gt;</h4></section></summary><div class='docblock'>List of transaction commitments.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.num_transactions\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_types/traits/block_contents.rs.html#117\">Source</a><a href=\"#method.num_transactions\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#method.num_transactions\" class=\"fn\">num_transactions</a>(&amp;self, metadata: &amp;Self::<a class=\"associatedtype\" href=\"hotshot_types/traits/block_contents/trait.BlockPayload.html#associatedtype.Metadata\" title=\"type hotshot_types::traits::block_contents::BlockPayload::Metadata\">Metadata</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.usize.html\">usize</a></h4></section></summary><div class='docblock'>Number of transactions in the block.</div></details></div></details>","BlockPayload<TYPES>","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-Clone-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; TestBlockPayload</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.1/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-Debug-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Deserialize%3C'de%3E-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-Deserialize%3C'de%3E-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'de&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize\" class=\"method trait-impl\"><a href=\"#method.deserialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserialize.html#tymethod.deserialize\" class=\"fn\">deserialize</a>&lt;__D&gt;(\n    __deserializer: __D,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;TestBlockPayload, &lt;__D as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserializer.html#associatedtype.Error\" title=\"type serde::de::Deserializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __D: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;,</div></h4></section></summary><div class='docblock'>Deserialize this value from the given Serde deserializer. <a href=\"https://docs.rs/serde/1.0.219/serde/de/trait.Deserialize.html#tymethod.deserialize\">Read more</a></div></details></div></details>","Deserialize<'de>","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Display-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-Display-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Display.html\" title=\"trait core::fmt::Display\">Display</a> for TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Display.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Display.html#tymethod.fmt\">Read more</a></div></details></div></details>","Display","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-EncodeBytes-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-EncodeBytes-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.EncodeBytes.html\" title=\"trait hotshot_types::traits::block_contents::EncodeBytes\">EncodeBytes</a> for TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.encode\" class=\"method trait-impl\"><a href=\"#method.encode\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.EncodeBytes.html#tymethod.encode\" class=\"fn\">encode</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>]&gt;</h4></section></summary><div class='docblock'>Encode <code>&amp;self</code></div></details></div></details>","EncodeBytes","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Hash-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-Hash-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> for TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash\" class=\"method trait-impl\"><a href=\"#method.hash\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hash.html#tymethod.hash\" class=\"fn\">hash</a>&lt;__H&gt;(&amp;self, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.reference.html\">&amp;mut __H</a>)<div class=\"where\">where\n    __H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,</div></h4></section></summary><div class='docblock'>Feeds this value into the given <a href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hash.html#tymethod.hash\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash_slice\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.3.0\">1.3.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.1/src/core/hash/mod.rs.html#235-237\">Source</a></span><a href=\"#method.hash_slice\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hash.html#method.hash_slice\" class=\"fn\">hash_slice</a>&lt;H&gt;(data: &amp;[Self], state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.reference.html\">&amp;mut H</a>)<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Feeds a slice of this type into the given <a href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.85.1/core/hash/trait.Hash.html#method.hash_slice\">Read more</a></div></details></div></details>","Hash","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-PartialEq-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;TestBlockPayload) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>self</code> and <code>other</code> values to be equal, and is used by <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.1/src/core/cmp.rs.html#261\">Source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>!=</code>. The default implementation is almost always sufficient,\nand should not be overridden without very good reason.</div></details></div></details>","PartialEq","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-Serialize-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;__S&gt;(\n    &amp;self,\n    __serializer: __S,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, &lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __S: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</div></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-TestBlockPayload\" class=\"impl\"><a href=\"#impl-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl TestBlockPayload</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.genesis\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">genesis</a>() -&gt; TestBlockPayload</h4></section></summary><div class=\"docblock\"><p>Create a genesis block payload with bytes <code>vec![0]</code>, to be used for\nconsensus task initiation.</p>\n<h5 id=\"panics\"><a class=\"doc-anchor\" href=\"#panics\">§</a>Panics</h5>\n<p>If the <code>VidScheme</code> construction fails.</p>\n</div></details></div></details>",0,"hotshot_query_service::testing::mocks::MockPayload"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-TestableBlock%3CTYPES%3E-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-TestableBlock%3CTYPES%3E-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"hotshot_types/traits/block_contents/trait.TestableBlock.html\" title=\"trait hotshot_types::traits::block_contents::TestableBlock\">TestableBlock</a>&lt;TYPES&gt; for TestBlockPayload<div class=\"where\">where\n    TYPES: <a class=\"trait\" href=\"hotshot_types/traits/node_implementation/trait.NodeType.html\" title=\"trait hotshot_types::traits::node_implementation::NodeType\">NodeType</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.genesis\" class=\"method trait-impl\"><a href=\"#method.genesis\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.TestableBlock.html#tymethod.genesis\" class=\"fn\">genesis</a>() -&gt; TestBlockPayload</h4></section></summary><div class='docblock'>generate a genesis block</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.txn_count\" class=\"method trait-impl\"><a href=\"#method.txn_count\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_types/traits/block_contents/trait.TestableBlock.html#tymethod.txn_count\" class=\"fn\">txn_count</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u64.html\">u64</a></h4></section></summary><div class='docblock'>the number of transactions in this block</div></details></div></details>","TestableBlock<TYPES>","hotshot_query_service::testing::mocks::MockPayload"],["<section id=\"impl-Eq-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-Eq-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for TestBlockPayload</h3></section>","Eq","hotshot_query_service::testing::mocks::MockPayload"],["<section id=\"impl-StructuralPartialEq-for-TestBlockPayload\" class=\"impl\"><a href=\"#impl-StructuralPartialEq-for-TestBlockPayload\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.StructuralPartialEq.html\" title=\"trait core::marker::StructuralPartialEq\">StructuralPartialEq</a> for TestBlockPayload</h3></section>","StructuralPartialEq","hotshot_query_service::testing::mocks::MockPayload"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[32770]}