(function() {
    var type_impls = Object.fromEntries([["hotshot_state_prover",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-CanonicalDeserialize-for-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-CanonicalDeserialize-for-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; CanonicalDeserialize for VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: Pairing,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize_with_mode\" class=\"method trait-impl\"><a href=\"#method.deserialize_with_mode\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">deserialize_with_mode</a>&lt;R&gt;(\n    reader: R,\n    compress: Compress,\n    validate: Validate,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;VerifyingKey&lt;E&gt;, SerializationError&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/std/io/trait.Read.html\" title=\"trait std::io::Read\">Read</a>,</div></h4></section></summary><div class='docblock'>The general deserialize method that takes in customization flags.</div></details><section id=\"method.deserialize_compressed\" class=\"method trait-impl\"><a href=\"#method.deserialize_compressed\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">deserialize_compressed</a>&lt;R&gt;(reader: R) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Self, SerializationError&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/std/io/trait.Read.html\" title=\"trait std::io::Read\">Read</a>,</div></h4></section><section id=\"method.deserialize_compressed_unchecked\" class=\"method trait-impl\"><a href=\"#method.deserialize_compressed_unchecked\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">deserialize_compressed_unchecked</a>&lt;R&gt;(\n    reader: R,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Self, SerializationError&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/std/io/trait.Read.html\" title=\"trait std::io::Read\">Read</a>,</div></h4></section><section id=\"method.deserialize_uncompressed\" class=\"method trait-impl\"><a href=\"#method.deserialize_uncompressed\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">deserialize_uncompressed</a>&lt;R&gt;(reader: R) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Self, SerializationError&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/std/io/trait.Read.html\" title=\"trait std::io::Read\">Read</a>,</div></h4></section><section id=\"method.deserialize_uncompressed_unchecked\" class=\"method trait-impl\"><a href=\"#method.deserialize_uncompressed_unchecked\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">deserialize_uncompressed_unchecked</a>&lt;R&gt;(\n    reader: R,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Self, SerializationError&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/std/io/trait.Read.html\" title=\"trait std::io::Read\">Read</a>,</div></h4></section></div></details>","CanonicalDeserialize","hotshot_state_prover::snark::VerifyingKey"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-CanonicalSerialize-for-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-CanonicalSerialize-for-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; CanonicalSerialize for VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: Pairing,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize_with_mode\" class=\"method trait-impl\"><a href=\"#method.serialize_with_mode\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">serialize_with_mode</a>&lt;W&gt;(\n    &amp;self,\n    writer: W,\n    compress: Compress,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, SerializationError&gt;<div class=\"where\">where\n    W: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/std/io/trait.Write.html\" title=\"trait std::io::Write\">Write</a>,</div></h4></section></summary><div class='docblock'>The general serialize method that takes in customization flags.</div></details><section id=\"method.serialized_size\" class=\"method trait-impl\"><a href=\"#method.serialized_size\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">serialized_size</a>(&amp;self, compress: Compress) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a></h4></section><section id=\"method.serialize_compressed\" class=\"method trait-impl\"><a href=\"#method.serialize_compressed\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">serialize_compressed</a>&lt;W&gt;(&amp;self, writer: W) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, SerializationError&gt;<div class=\"where\">where\n    W: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/std/io/trait.Write.html\" title=\"trait std::io::Write\">Write</a>,</div></h4></section><section id=\"method.compressed_size\" class=\"method trait-impl\"><a href=\"#method.compressed_size\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">compressed_size</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a></h4></section><section id=\"method.serialize_uncompressed\" class=\"method trait-impl\"><a href=\"#method.serialize_uncompressed\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">serialize_uncompressed</a>&lt;W&gt;(&amp;self, writer: W) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, SerializationError&gt;<div class=\"where\">where\n    W: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/std/io/trait.Write.html\" title=\"trait std::io::Write\">Write</a>,</div></h4></section><section id=\"method.uncompressed_size\" class=\"method trait-impl\"><a href=\"#method.uncompressed_size\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">uncompressed_size</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a></h4></section></div></details>","CanonicalSerialize","hotshot_state_prover::snark::VerifyingKey"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-Clone-for-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + Pairing,\n    &lt;E as Pairing&gt;::ScalarField: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; VerifyingKey&lt;E&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.86.0/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","hotshot_state_prover::snark::VerifyingKey"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-Debug-for-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + Pairing,\n    &lt;E as Pairing&gt;::ScalarField: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","hotshot_state_prover::snark::VerifyingKey"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CParsedVerifyingKey%3E-for-VerifyingKey%3CBn%3CConfig%3E%3E\" class=\"impl\"><a href=\"#impl-From%3CParsedVerifyingKey%3E-for-VerifyingKey%3CBn%3CConfig%3E%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;ParsedVerifyingKey&gt; for VerifyingKey&lt;Bn&lt;Config&gt;&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(vk: ParsedVerifyingKey) -&gt; VerifyingKey&lt;Bn&lt;Config&gt;&gt;</h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<ParsedVerifyingKey>","hotshot_state_prover::snark::VerifyingKey"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-PartialEq-for-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> + Pairing,\n    &lt;E as Pairing&gt;::ScalarField: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;VerifyingKey&lt;E&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>self</code> and <code>other</code> values to be equal, and is used by <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.86.0/src/core/cmp.rs.html#261\">Source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>!=</code>. The default implementation is almost always sufficient,\nand should not be overridden without very good reason.</div></details></div></details>","PartialEq","hotshot_state_prover::snark::VerifyingKey"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Valid-for-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-Valid-for-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; Valid for VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: Pairing,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.check\" class=\"method trait-impl\"><a href=\"#method.check\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">check</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, SerializationError&gt;</h4></section><section id=\"method.batch_check\" class=\"method trait-impl\"><a href=\"#method.batch_check\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">batch_check</a>&lt;'a&gt;(\n    batch: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a>&lt;Item = &amp;'a VerifyingKey&lt;E&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, SerializationError&gt;<div class=\"where\">where\n    VerifyingKey&lt;E&gt;: 'a,</div></h4></section></div></details>","Valid","hotshot_state_prover::snark::VerifyingKey"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: Pairing,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.dummy\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">dummy</a>(num_inputs: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a>, domain_size: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a>) -&gt; VerifyingKey&lt;E&gt;</h4></section></summary><div class=\"docblock\"><p>Create a dummy TurboPlonk verification key for a circuit with\n<code>num_inputs</code> public inputs and domain size <code>domain_size</code>.</p>\n</div></details></div></details>",0,"hotshot_state_prover::snark::VerifyingKey"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, F, P&gt; VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: Pairing&lt;BaseField = F, G1Affine = Affine&lt;P&gt;&gt;,\n    F: SWToTEConParam,\n    P: SWCurveConfig&lt;BaseField = F&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.convert_te_coordinates_to_scalars\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">convert_te_coordinates_to_scalars</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;F&gt;</h4></section></summary><div class=\"docblock\"><p>Convert the group elements to a list of scalars that represent the\nTwisted Edwards coordinates.</p>\n</div></details></div></details>",0,"hotshot_state_prover::snark::VerifyingKey"],["<section id=\"impl-Eq-for-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-Eq-for-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> + Pairing,\n    &lt;E as Pairing&gt;::ScalarField: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a>,</div></h3></section>","Eq","hotshot_state_prover::snark::VerifyingKey"],["<section id=\"impl-StructuralPartialEq-for-VerifyingKey%3CE%3E\" class=\"impl\"><a href=\"#impl-StructuralPartialEq-for-VerifyingKey%3CE%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.StructuralPartialEq.html\" title=\"trait core::marker::StructuralPartialEq\">StructuralPartialEq</a> for VerifyingKey&lt;E&gt;<div class=\"where\">where\n    E: Pairing,</div></h3></section>","StructuralPartialEq","hotshot_state_prover::snark::VerifyingKey"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[20348]}