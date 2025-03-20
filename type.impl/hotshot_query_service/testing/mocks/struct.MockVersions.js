(function() {
    var type_impls = Object.fromEntries([["hotshot_query_service",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-MockVersions\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#145\">Source</a><a href=\"#impl-Clone-for-MockVersions\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"hotshot_query_service/testing/mocks/struct.MockVersions.html\" title=\"struct hotshot_query_service::testing::mocks::MockVersions\">MockVersions</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#145\">Source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"hotshot_query_service/testing/mocks/struct.MockVersions.html\" title=\"struct hotshot_query_service::testing::mocks::MockVersions\">MockVersions</a></h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.1/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","hotshot_query_service::testing::mocks::MockBase"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-MockVersions\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#145\">Source</a><a href=\"#impl-Debug-for-MockVersions\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"hotshot_query_service/testing/mocks/struct.MockVersions.html\" title=\"struct hotshot_query_service::testing::mocks::MockVersions\">MockVersions</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#145\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","hotshot_query_service::testing::mocks::MockBase"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Versions-for-MockVersions\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#148-158\">Source</a><a href=\"#impl-Versions-for-MockVersions\" class=\"anchor\">§</a><h3 class=\"code-header\">impl Versions for <a class=\"struct\" href=\"hotshot_query_service/testing/mocks/struct.MockVersions.html\" title=\"struct hotshot_query_service::testing::mocks::MockVersions\">MockVersions</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedconstant.UPGRADE_HASH\" class=\"associatedconstant trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#151-154\">Source</a><a href=\"#associatedconstant.UPGRADE_HASH\" class=\"anchor\">§</a><h4 class=\"code-header\">const <a class=\"constant\">UPGRADE_HASH</a>: [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.array.html\">32</a>]</h4></section></summary><div class='docblock'>The hash for the upgrade.</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Base\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#149\">Source</a><a href=\"#associatedtype.Base\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Base</a> = StaticVersion&lt;0, 1&gt;</h4></section></summary><div class='docblock'>The base version of HotShot this node is instantiated with.</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Upgrade\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#150\">Source</a><a href=\"#associatedtype.Upgrade\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Upgrade</a> = StaticVersion&lt;0, 2&gt;</h4></section></summary><div class='docblock'>The version of HotShot this node may be upgraded to. Set equal to <code>Base</code> to disable upgrades.</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Marketplace\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#156\">Source</a><a href=\"#associatedtype.Marketplace\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Marketplace</a> = StaticVersion&lt;0, 3&gt;</h4></section></summary><div class='docblock'>The version at which to switch over to marketplace logic</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Epochs\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#157\">Source</a><a href=\"#associatedtype.Epochs\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Epochs</a> = StaticVersion&lt;0, 4&gt;</h4></section></summary><div class='docblock'>The version at which to switch over to epochs logic</div></details></div></details>","Versions","hotshot_query_service::testing::mocks::MockBase"],["<section id=\"impl-Copy-for-MockVersions\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/mocks.rs.html#145\">Source</a><a href=\"#impl-Copy-for-MockVersions\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a> for <a class=\"struct\" href=\"hotshot_query_service/testing/mocks/struct.MockVersions.html\" title=\"struct hotshot_query_service::testing::mocks::MockVersions\">MockVersions</a></h3></section>","Copy","hotshot_query_service::testing::mocks::MockBase"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[7936]}