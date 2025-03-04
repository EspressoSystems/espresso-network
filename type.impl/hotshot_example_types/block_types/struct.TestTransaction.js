(function() {
    var type_impls = Object.fromEntries([["hotshot_query_service",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-Clone-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.0/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Committable-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#111\">Source</a><a href=\"#impl-Committable-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl Committable for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.commit\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#112\">Source</a><a href=\"#method.commit\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">commit</a>(&amp;self) -&gt; Commitment&lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a>&gt;</h4></section></summary><div class='docblock'>Create a binding commitment to <code>self</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.tag\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#120\">Source</a><a href=\"#method.tag\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">tag</a>() -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a></h4></section></summary><div class='docblock'>Tag that should be used when serializing commitments to this type. <a>Read more</a></div></details></div></details>","Committable","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-Debug-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-Default-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/1.85.0/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Deserialize%3C'de%3E-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-Deserialize%3C'de%3E-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'de&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#method.deserialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.218/serde/de/trait.Deserialize.html#tymethod.deserialize\" class=\"fn\">deserialize</a>&lt;__D&gt;(\n    __deserializer: __D,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a>, &lt;__D as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.Deserializer.html#associatedtype.Error\" title=\"type serde::de::Deserializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __D: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;,</div></h4></section></summary><div class='docblock'>Deserialize this value from the given Serde deserializer. <a href=\"https://docs.rs/serde/1.0.218/serde/de/trait.Deserialize.html#tymethod.deserialize\">Read more</a></div></details></div></details>","Deserialize<'de>","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Hash-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-Hash-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#method.hash\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hash.html#tymethod.hash\" class=\"fn\">hash</a>&lt;__H&gt;(&amp;self, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.reference.html\">&amp;mut __H</a>)<div class=\"where\">where\n    __H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,</div></h4></section></summary><div class='docblock'>Feeds this value into the given <a href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hash.html#tymethod.hash\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash_slice\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.3.0\">1.3.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.0/src/core/hash/mod.rs.html#235-237\">Source</a></span><a href=\"#method.hash_slice\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hash.html#method.hash_slice\" class=\"fn\">hash_slice</a>&lt;H&gt;(data: &amp;[Self], state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.reference.html\">&amp;mut H</a>)<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Feeds a slice of this type into the given <a href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.85.0/core/hash/trait.Hash.html#method.hash_slice\">Read more</a></div></details></div></details>","Hash","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-PartialEq-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>self</code> and <code>other</code> values to be equal, and is used by <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.0/src/core/cmp.rs.html#261\">Source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>!=</code>. The default implementation is almost always sufficient,\nand should not be overridden without very good reason.</div></details></div></details>","PartialEq","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-Serialize-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.218/serde/ser/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;__S&gt;(\n    &amp;self,\n    __serializer: __S,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.218/serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, &lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.218/serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __S: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</div></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"https://docs.rs/serde/1.0.218/serde/ser/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#58\">Source</a><a href=\"#impl-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#63\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_example_types/block_types/struct.TestTransaction.html#tymethod.new\" class=\"fn\">new</a>(bytes: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u8.html\">u8</a>&gt;) -&gt; <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h4></section></summary><div class=\"docblock\"><p>Construct a new transaction</p>\n<h5 id=\"panics\"><a class=\"doc-anchor\" href=\"#panics\">§</a>Panics</h5>\n<p>If <code>bytes.len()</code> &gt; <code>u32::MAX</code></p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_new\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#70\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_example_types/block_types/struct.TestTransaction.html#tymethod.try_new\" class=\"fn\">try_new</a>(bytes: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u8.html\">u8</a>&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Construct a new transaction.\nReturns <code>None</code> if <code>bytes.len()</code> &gt; <code>u32::MAX</code>\nfor cross-platform compatibility</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.bytes\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#79\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_example_types/block_types/struct.TestTransaction.html#tymethod.bytes\" class=\"fn\">bytes</a>(&amp;self) -&gt; &amp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u8.html\">u8</a>&gt; <a href=\"#\" class=\"tooltip\" data-notable-ty=\"&amp;Vec&lt;u8&gt;\">ⓘ</a></h4></section></summary><div class=\"docblock\"><p>Get reference to raw bytes of transaction</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.into_bytes\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#84\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_example_types/block_types/struct.TestTransaction.html#tymethod.into_bytes\" class=\"fn\">into_bytes</a>(self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u8.html\">u8</a>&gt; <a href=\"#\" class=\"tooltip\" data-notable-ty=\"Vec&lt;u8&gt;\">ⓘ</a></h4></section></summary><div class=\"docblock\"><p>Convert transaction to raw vector of bytes</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.encode\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#92\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_example_types/block_types/struct.TestTransaction.html#tymethod.encode\" class=\"fn\">encode</a>(transactions: &amp;[<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a>]) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u8.html\">u8</a>&gt; <a href=\"#\" class=\"tooltip\" data-notable-ty=\"Vec&lt;u8&gt;\">ⓘ</a></h4></section></summary><div class=\"docblock\"><p>Encode a list of transactions into bytes.</p>\n<h5 id=\"errors\"><a class=\"doc-anchor\" href=\"#errors\">§</a>Errors</h5>\n<p>If the transaction length conversion fails.</p>\n</div></details></div></details>",0,"hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Transaction-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#125\">Source</a><a href=\"#impl-Transaction-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl Transaction for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.minimum_block_size\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#126\">Source</a><a href=\"#method.minimum_block_size\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">minimum_block_size</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u64.html\">u64</a></h4></section></summary><div class='docblock'>The function to estimate the transaction size\nIt takes in the transaction itself and a boolean indicating if the transaction adds a new namespace\nSince each new namespace adds overhead\njust ignore this parameter by default and use it when needed</div></details></div></details>","Transaction","hotshot_query_service::testing::mocks::MockTransaction"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-TryFrom%3CVec%3Cu8%3E%3E-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#50\">Source</a><a href=\"#impl-TryFrom%3CVec%3Cu8%3E%3E-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/convert/trait.TryFrom.html\" title=\"trait core::convert::TryFrom\">TryFrom</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u8.html\">u8</a>&gt;&gt; for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Error\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#51\">Source</a><a href=\"#associatedtype.Error\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"https://doc.rust-lang.org/1.85.0/core/convert/trait.TryFrom.html#associatedtype.Error\" class=\"associatedtype\">Error</a> = <a class=\"enum\" href=\"hotshot_example_types/block_types/enum.TransactionError.html\" title=\"enum hotshot_example_types::block_types::TransactionError\">TransactionError</a></h4></section></summary><div class='docblock'>The type returned in the event of a conversion error.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#53\">Source</a><a href=\"#method.try_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/convert/trait.TryFrom.html#tymethod.try_from\" class=\"fn\">try_from</a>(\n    value: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u8.html\">u8</a>&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a>, &lt;<a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a> as <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/convert/trait.TryFrom.html\" title=\"trait core::convert::TryFrom\">TryFrom</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.u8.html\">u8</a>&gt;&gt;&gt;::<a class=\"associatedtype\" href=\"https://doc.rust-lang.org/1.85.0/core/convert/trait.TryFrom.html#associatedtype.Error\" title=\"type core::convert::TryFrom::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Performs the conversion.</div></details></div></details>","TryFrom<Vec<u8>>","hotshot_query_service::testing::mocks::MockTransaction"],["<section id=\"impl-Eq-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-Eq-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section>","Eq","hotshot_query_service::testing::mocks::MockTransaction"],["<section id=\"impl-StructuralPartialEq-for-TestTransaction\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_example_types/block_types.rs.html#40\">Source</a><a href=\"#impl-StructuralPartialEq-for-TestTransaction\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/marker/trait.StructuralPartialEq.html\" title=\"trait core::marker::StructuralPartialEq\">StructuralPartialEq</a> for <a class=\"struct\" href=\"hotshot_example_types/block_types/struct.TestTransaction.html\" title=\"struct hotshot_example_types::block_types::TestTransaction\">TestTransaction</a></h3></section>","StructuralPartialEq","hotshot_query_service::testing::mocks::MockTransaction"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[30199]}