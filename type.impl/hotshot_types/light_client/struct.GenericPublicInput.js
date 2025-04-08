(function() {
    var type_impls = Object.fromEntries([["hotshot_types",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-GenericPublicInput%3CF%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#213\">Source</a><a href=\"#impl-Clone-for-GenericPublicInput%3CF%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + PrimeField&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"hotshot_types/light_client/struct.GenericPublicInput.html\" title=\"struct hotshot_types::light_client::GenericPublicInput\">GenericPublicInput</a>&lt;F&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#213\">Source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"hotshot_types/light_client/struct.GenericPublicInput.html\" title=\"struct hotshot_types::light_client::GenericPublicInput\">GenericPublicInput</a>&lt;F&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.86.0/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.86.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","hotshot_types::light_client::PublicInput"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-GenericPublicInput%3CF%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#213\">Source</a><a href=\"#impl-Debug-for-GenericPublicInput%3CF%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + PrimeField&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"hotshot_types/light_client/struct.GenericPublicInput.html\" title=\"struct hotshot_types::light_client::GenericPublicInput\">GenericPublicInput</a>&lt;F&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#213\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/1.86.0/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.86.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","hotshot_types::light_client::PublicInput"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CVec%3CF%3E%3E-for-GenericPublicInput%3CF%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#273-298\">Source</a><a href=\"#impl-From%3CVec%3CF%3E%3E-for-GenericPublicInput%3CF%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;F: PrimeField&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;F&gt;&gt; for <a class=\"struct\" href=\"hotshot_types/light_client/struct.GenericPublicInput.html\" title=\"struct hotshot_types::light_client::GenericPublicInput\">GenericPublicInput</a>&lt;F&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#274-297\">Source</a><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(v: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;F&gt;) -&gt; Self</h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<Vec<F>>","hotshot_types::light_client::PublicInput"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-GenericPublicInput%3CF%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#223-253\">Source</a><a href=\"#impl-GenericPublicInput%3CF%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;F: PrimeField&gt; <a class=\"struct\" href=\"hotshot_types/light_client/struct.GenericPublicInput.html\" title=\"struct hotshot_types::light_client::GenericPublicInput\">GenericPublicInput</a>&lt;F&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#225-235\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_types/light_client/struct.GenericPublicInput.html#tymethod.new\" class=\"fn\">new</a>(\n    lc_state: <a class=\"struct\" href=\"hotshot_types/light_client/struct.GenericLightClientState.html\" title=\"struct hotshot_types::light_client::GenericLightClientState\">GenericLightClientState</a>&lt;F&gt;,\n    voting_st_state: <a class=\"struct\" href=\"hotshot_types/light_client/struct.GenericStakeTableState.html\" title=\"struct hotshot_types::light_client::GenericStakeTableState\">GenericStakeTableState</a>&lt;F&gt;,\n    next_st_state: <a class=\"struct\" href=\"hotshot_types/light_client/struct.GenericStakeTableState.html\" title=\"struct hotshot_types::light_client::GenericStakeTableState\">GenericStakeTableState</a>&lt;F&gt;,\n) -&gt; Self</h4></section></summary><div class=\"docblock\"><p>Construct a public input from light client state and static stake table state</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.to_vec\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_types/light_client.rs.html#238-252\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_types/light_client/struct.GenericPublicInput.html#tymethod.to_vec\" class=\"fn\">to_vec</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;F&gt;</h4></section></summary><div class=\"docblock\"><p>Convert to a vector of field elements</p>\n</div></details></div></details>",0,"hotshot_types::light_client::PublicInput"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[8337]}