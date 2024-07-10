(function() {var type_impls = {
"espresso_types":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Clone-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + NodeType,\n    &lt;TYPES as NodeType&gt;::Time: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,\n    &lt;TYPES as NodeType&gt;::BlockHeader: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,\n    &lt;TYPES as NodeType&gt;::BlockPayload: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; Leaf&lt;TYPES&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.79.0/src/core/clone.rs.html#169\">source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.reference.html\">&amp;Self</a>)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.79.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Committable-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Committable-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; Committable for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.commit\" class=\"method trait-impl\"><a href=\"#method.commit\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">commit</a>(&amp;self) -&gt; Commitment&lt;Leaf&lt;TYPES&gt;&gt;</h4></section></summary><div class='docblock'>Create a binding commitment to <code>self</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.tag\" class=\"method trait-impl\"><a href=\"#method.tag\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">tag</a>() -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a></h4></section></summary><div class='docblock'>Tag that should be used when serializing commitments to this type. <a>Read more</a></div></details></div></details>","Committable","espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Debug-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + NodeType,\n    &lt;TYPES as NodeType&gt;::Time: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,\n    &lt;TYPES as NodeType&gt;::BlockHeader: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,\n    &lt;TYPES as NodeType&gt;::BlockPayload: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Deserialize%3C'de%3E-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Deserialize%3C'de%3E-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'de, TYPES&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize\" class=\"method trait-impl\"><a href=\"#method.deserialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.204/serde/de/trait.Deserialize.html#tymethod.deserialize\" class=\"fn\">deserialize</a>&lt;__D&gt;(\n    __deserializer: __D\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Leaf&lt;TYPES&gt;, &lt;__D as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.204/serde/de/trait.Deserializer.html#associatedtype.Error\" title=\"type serde::de::Deserializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __D: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;,</div></h4></section></summary><div class='docblock'>Deserialize this value from the given Serde deserializer. <a href=\"https://docs.rs/serde/1.0.204/serde/de/trait.Deserialize.html#tymethod.deserialize\">Read more</a></div></details></div></details>","Deserialize<'de>","espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Display-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Display-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Display.html\" title=\"trait core::fmt::Display\">Display</a> for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Display.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.79.0/core/fmt/trait.Display.html#tymethod.fmt\">Read more</a></div></details></div></details>","Display","espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Hash-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Hash-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash\" class=\"method trait-impl\"><a href=\"#method.hash\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hash.html#tymethod.hash\" class=\"fn\">hash</a>&lt;H&gt;(&amp;self, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.reference.html\">&amp;mut H</a>)<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,</div></h4></section></summary><div class='docblock'>Feeds this value into the given <a href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hash.html#tymethod.hash\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash_slice\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.3.0\">1.3.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.79.0/src/core/hash/mod.rs.html#238-240\">source</a></span><a href=\"#method.hash_slice\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hash.html#method.hash_slice\" class=\"fn\">hash_slice</a>&lt;H&gt;(data: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.slice.html\">[Self]</a>, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.reference.html\">&amp;mut H</a>)<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h4></section></summary><div class='docblock'>Feeds a slice of this type into the given <a href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.79.0/core/hash/trait.Hash.html#method.hash_slice\">Read more</a></div></details></div></details>","Hash","espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from_quorum_proposal\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">from_quorum_proposal</a>(\n    quorum_proposal: &amp;QuorumProposal&lt;TYPES&gt;\n) -&gt; Leaf&lt;TYPES&gt;</h4></section></summary><div class=\"docblock\"><p>Constructs a leaf from a given quorum proposal.</p>\n</div></details></div></details>",0,"espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.genesis\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">genesis</a>(\n    validated_state: &amp;&lt;TYPES as NodeType&gt;::ValidatedState,\n    instance_state: &amp;&lt;TYPES as NodeType&gt;::InstanceState\n) -&gt; Leaf&lt;TYPES&gt;</h4></section></summary><div class=\"docblock\"><p>Create a new leaf from its components.</p>\n<h5 id=\"panics\"><a class=\"doc-anchor\" href=\"#panics\">§</a>Panics</h5>\n<p>Panics if the genesis payload (<code>TYPES::BlockPayload::genesis()</code>) is malformed (unable to be\ninterpreted as bytes).</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.view_number\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">view_number</a>(&amp;self) -&gt; &lt;TYPES as NodeType&gt;::Time</h4></section></summary><div class=\"docblock\"><p>Time when this leaf was created.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.height\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">height</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.u64.html\">u64</a></h4></section></summary><div class=\"docblock\"><p>Height of this leaf in the chain.</p>\n<p>Equivalently, this is the number of leaves before this one in the chain.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.justify_qc\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">justify_qc</a>(\n    &amp;self\n) -&gt; SimpleCertificate&lt;TYPES, QuorumData&lt;TYPES&gt;, SuccessThreshold&gt;</h4></section></summary><div class=\"docblock\"><p>The QC linking this leaf to its parent in the chain.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.upgrade_certificate\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">upgrade_certificate</a>(\n    &amp;self\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;SimpleCertificate&lt;TYPES, UpgradeProposalData&lt;TYPES&gt;, UpgradeThreshold&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>The QC linking this leaf to its parent in the chain.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.parent_commitment\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">parent_commitment</a>(&amp;self) -&gt; Commitment&lt;Leaf&lt;TYPES&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Commitment to this leaf’s parent.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.block_header\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">block_header</a>(&amp;self) -&gt; &amp;&lt;TYPES as NodeType&gt;::BlockHeader</h4></section></summary><div class=\"docblock\"><p>The block header contained in this leaf.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.block_header_mut\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">block_header_mut</a>(&amp;mut self) -&gt; &amp;mut &lt;TYPES as NodeType&gt;::BlockHeader</h4></section></summary><div class=\"docblock\"><p>Get a mutable reference to the block header contained in this leaf.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.fill_block_payload\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">fill_block_payload</a>(\n    &amp;mut self,\n    block_payload: &lt;TYPES as NodeType&gt;::BlockPayload,\n    num_storage_nodes: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.usize.html\">usize</a>\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>, BlockError&gt;</h4></section></summary><div class=\"docblock\"><p>Fill this leaf with the block payload.</p>\n<h5 id=\"errors\"><a class=\"doc-anchor\" href=\"#errors\">§</a>Errors</h5>\n<p>Fails if the payload commitment doesn’t match <code>self.block_header.payload_commitment()</code>\nor if the transactions are of invalid length</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.fill_block_payload_unchecked\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">fill_block_payload_unchecked</a>(\n    &amp;mut self,\n    block_payload: &lt;TYPES as NodeType&gt;::BlockPayload\n)</h4></section></summary><div class=\"docblock\"><p>Fill this leaf with the block payload, without checking\nheader and payload consistency</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.block_payload\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">block_payload</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;TYPES as NodeType&gt;::BlockPayload&gt;</h4></section></summary><div class=\"docblock\"><p>Optional block payload.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.payload_commitment\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">payload_commitment</a>(&amp;self) -&gt; &lt;VidSchemeType as VidScheme&gt;::Commit</h4></section></summary><div class=\"docblock\"><p>A commitment to the block payload contained in this leaf.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.temp_extends_upgrade\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">temp_extends_upgrade</a>(\n    &amp;self,\n    parent: &amp;Leaf&lt;TYPES&gt;,\n    decided_upgrade_certificate: &amp;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;SimpleCertificate&lt;TYPES, UpgradeProposalData&lt;TYPES&gt;, UpgradeThreshold&gt;&gt;\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.85/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Validate that a leaf has the right upgrade certificate to be the immediate child of another leaf</p>\n<p>This may not be a complete function. Please double-check that it performs the checks you expect before subtituting validation logic with it.</p>\n<h5 id=\"errors-1\"><a class=\"doc-anchor\" href=\"#errors-1\">§</a>Errors</h5>\n<p>Returns an error if the certificates are not identical, or that when we no longer see a\ncert, it’s for the right reason.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.extends_upgrade\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">extends_upgrade</a>(\n    &amp;self,\n    parent: &amp;Leaf&lt;TYPES&gt;,\n    decided_upgrade_certificate: &amp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.79.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;SimpleCertificate&lt;TYPES, UpgradeProposalData&lt;TYPES&gt;, UpgradeThreshold&gt;&gt;&gt;&gt;\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.85/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Validate that a leaf has the right upgrade certificate to be the immediate child of another leaf</p>\n<p>This may not be a complete function. Please double-check that it performs the checks you expect before subtituting validation logic with it.</p>\n<h5 id=\"errors-2\"><a class=\"doc-anchor\" href=\"#errors-2\">§</a>Errors</h5>\n<p>Returns an error if the certificates are not identical, or that when we no longer see a\ncert, it’s for the right reason.</p>\n</div></details></div></details>",0,"espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-PartialEq-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;Leaf&lt;TYPES&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>This method tests for <code>self</code> and <code>other</code> values to be equal, and is used\nby <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.79.0/src/core/cmp.rs.html#263\">source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.79.0/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>This method tests for <code>!=</code>. The default implementation is almost always\nsufficient, and should not be overridden without very good reason.</div></details></div></details>","PartialEq","espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Serialize-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType + <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,\n    &lt;TYPES as NodeType&gt;::Time: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,\n    &lt;TYPES as NodeType&gt;::BlockHeader: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;__S&gt;(\n    &amp;self,\n    __serializer: __S\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.79.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, &lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __S: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</div></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"https://docs.rs/serde/1.0.204/serde/ser/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","espresso_types::v0::Leaf"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-TestableLeaf-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-TestableLeaf-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; TestableLeaf for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,\n    &lt;TYPES as NodeType&gt;::ValidatedState: TestableState&lt;TYPES&gt;,\n    &lt;TYPES as NodeType&gt;::BlockPayload: TestableBlock&lt;TYPES&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.NodeType\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.NodeType\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">NodeType</a> = TYPES</h4></section></summary><div class='docblock'>Type of nodes participating in the network.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.create_random_transaction\" class=\"method trait-impl\"><a href=\"#method.create_random_transaction\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">create_random_transaction</a>(\n    &amp;self,\n    rng: &amp;mut dyn <a class=\"trait\" href=\"https://rust-random.github.io/rand/rand_core/trait.RngCore.html\" title=\"trait rand_core::RngCore\">RngCore</a>,\n    padding: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.79.0/std/primitive.u64.html\">u64</a>\n) -&gt; &lt;&lt;&lt;Leaf&lt;TYPES&gt; as TestableLeaf&gt;::NodeType as NodeType&gt;::BlockPayload as BlockPayload&lt;&lt;Leaf&lt;TYPES&gt; as TestableLeaf&gt;::NodeType&gt;&gt;::Transaction</h4></section></summary><div class='docblock'>Create a transaction that can be added to the block contained in this leaf.</div></details></div></details>","TestableLeaf","espresso_types::v0::Leaf"],["<section id=\"impl-Eq-for-Leaf%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Eq-for-Leaf%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for Leaf&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> + NodeType,\n    &lt;TYPES as NodeType&gt;::Time: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a>,\n    &lt;TYPES as NodeType&gt;::BlockHeader: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a>,\n    &lt;TYPES as NodeType&gt;::BlockPayload: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.79.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a>,</div></h3></section>","Eq","espresso_types::v0::Leaf"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()