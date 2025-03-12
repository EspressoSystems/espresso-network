(function() {
    var type_impls = Object.fromEntries([["sequencer",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-AnyProvider%3CTypes%3E\" class=\"impl\"><a href=\"#impl-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; AnyProvider&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.with_provider\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">with_provider</a>&lt;P&gt;(self, provider: P) -&gt; AnyProvider&lt;Types&gt;<div class=\"where\">where\n    P: AvailabilityProvider&lt;Types&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Add a sub-provider which fetches both blocks and leaves.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.with_block_provider\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">with_block_provider</a>&lt;P&gt;(self, provider: P) -&gt; AnyProvider&lt;Types&gt;<div class=\"where\">where\n    P: Provider&lt;Types, PayloadRequest&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Add a sub-provider which fetches blocks.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.with_leaf_provider\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">with_leaf_provider</a>&lt;P&gt;(self, provider: P) -&gt; AnyProvider&lt;Types&gt;<div class=\"where\">where\n    P: Provider&lt;Types, LeafRequest&lt;Types&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Add a sub-provider which fetches leaves.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.with_vid_common_provider\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">with_vid_common_provider</a>&lt;P&gt;(self, provider: P) -&gt; AnyProvider&lt;Types&gt;<div class=\"where\">where\n    P: Provider&lt;Types, VidCommonRequest&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Add a sub-provider which fetches VID common data.</p>\n</div></details></div></details>",0,"sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a href=\"#impl-Clone-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for AnyProvider&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; AnyProvider&lt;Types&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.0/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.85.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a href=\"#impl-Debug-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for AnyProvider&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, __f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.85.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a href=\"#impl-Default-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for AnyProvider&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.0/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; AnyProvider&lt;Types&gt;</h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/1.85.0/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Provider%3CTypes,+LeafRequest%3CTypes%3E%3E-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a href=\"#impl-Provider%3CTypes,+LeafRequest%3CTypes%3E%3E-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; Provider&lt;Types, LeafRequest&lt;Types&gt;&gt; for AnyProvider&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch\" class=\"method trait-impl\"><a href=\"#method.fetch\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">fetch</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    req: LeafRequest&lt;Types&gt;,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;LeafQueryData&lt;Types&gt;&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    AnyProvider&lt;Types&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Fetch a resource.</div></details></div></details>","Provider<Types, LeafRequest<Types>>","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Provider%3CTypes,+PayloadRequest%3E-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a href=\"#impl-Provider%3CTypes,+PayloadRequest%3E-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; Provider&lt;Types, PayloadRequest&gt; for AnyProvider&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch\" class=\"method trait-impl\"><a href=\"#method.fetch\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">fetch</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    req: PayloadRequest,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;Types as NodeType&gt;::BlockPayload&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    AnyProvider&lt;Types&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Fetch a resource.</div></details></div></details>","Provider<Types, PayloadRequest>","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Provider%3CTypes,+VidCommonRequest%3E-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a href=\"#impl-Provider%3CTypes,+VidCommonRequest%3E-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; Provider&lt;Types, VidCommonRequest&gt; for AnyProvider&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch\" class=\"method trait-impl\"><a href=\"#method.fetch\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">fetch</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    req: VidCommonRequest,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;VidCommon&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    AnyProvider&lt;Types&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Fetch a resource.</div></details></div></details>","Provider<Types, VidCommonRequest>","sequencer::api::data_source::Provider"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[12992]}