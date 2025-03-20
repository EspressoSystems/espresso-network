(function() {
    var type_impls = Object.fromEntries([["sequencer",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-AnyProvider%3CTypes%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#130-132\">Source</a><a href=\"#impl-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.with_provider\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#135-137\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html#tymethod.with_provider\" class=\"fn\">with_provider</a>&lt;P&gt;(self, provider: P) -&gt; <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    P: <a class=\"trait\" href=\"hotshot_query_service/data_source/fetching/trait.AvailabilityProvider.html\" title=\"trait hotshot_query_service::data_source::fetching::AvailabilityProvider\">AvailabilityProvider</a>&lt;Types&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Add a sub-provider which fetches both blocks and leaves.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.with_block_provider\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#147-149\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html#tymethod.with_block_provider\" class=\"fn\">with_block_provider</a>&lt;P&gt;(self, provider: P) -&gt; <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    P: <a class=\"trait\" href=\"hotshot_query_service/fetching/provider/trait.Provider.html\" title=\"trait hotshot_query_service::fetching::provider::Provider\">Provider</a>&lt;Types, <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.PayloadRequest.html\" title=\"struct hotshot_query_service::fetching::request::PayloadRequest\">PayloadRequest</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Add a sub-provider which fetches blocks.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.with_leaf_provider\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#156-158\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html#tymethod.with_leaf_provider\" class=\"fn\">with_leaf_provider</a>&lt;P&gt;(self, provider: P) -&gt; <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    P: <a class=\"trait\" href=\"hotshot_query_service/fetching/provider/trait.Provider.html\" title=\"trait hotshot_query_service::fetching::provider::Provider\">Provider</a>&lt;Types, <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.LeafRequest.html\" title=\"struct hotshot_query_service::fetching::request::LeafRequest\">LeafRequest</a>&lt;Types&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Add a sub-provider which fetches leaves.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.with_vid_common_provider\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#165-167\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html#tymethod.with_vid_common_provider\" class=\"fn\">with_vid_common_provider</a>&lt;P&gt;(self, provider: P) -&gt; <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    P: <a class=\"trait\" href=\"hotshot_query_service/fetching/provider/trait.Provider.html\" title=\"trait hotshot_query_service::fetching::provider::Provider\">Provider</a>&lt;Types, <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.VidCommonRequest.html\" title=\"struct hotshot_query_service::fetching::request::VidCommonRequest\">VidCommonRequest</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Add a sub-provider which fetches VID common data.</p>\n</div></details></div></details>",0,"sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#89\">Source</a><a href=\"#impl-Clone-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#89\">Source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.1/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#51\">Source</a><a href=\"#impl-Debug-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#51\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, __f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#89\">Source</a><a href=\"#impl-Default-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#89\">Source</a><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;</h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/1.85.1/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Provider%3CTypes,+LeafRequest%3CTypes%3E%3E-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#111-113\">Source</a><a href=\"#impl-Provider%3CTypes,+LeafRequest%3CTypes%3E%3E-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"hotshot_query_service/fetching/provider/trait.Provider.html\" title=\"trait hotshot_query_service::fetching::provider::Provider\">Provider</a>&lt;Types, <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.LeafRequest.html\" title=\"struct hotshot_query_service::fetching::request::LeafRequest\">LeafRequest</a>&lt;Types&gt;&gt; for <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#110\">Source</a><a href=\"#method.fetch\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/fetching/provider/trait.Provider.html#tymethod.fetch\" class=\"fn\">fetch</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    req: <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.LeafRequest.html\" title=\"struct hotshot_query_service::fetching::request::LeafRequest\">LeafRequest</a>&lt;Types&gt;,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"hotshot_query_service/availability/query_data/struct.LeafQueryData.html\" title=\"struct hotshot_query_service::availability::query_data::LeafQueryData\">LeafQueryData</a>&lt;Types&gt;&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Fetch a resource.</div></details></div></details>","Provider<Types, LeafRequest<Types>>","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Provider%3CTypes,+PayloadRequest%3E-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#101-103\">Source</a><a href=\"#impl-Provider%3CTypes,+PayloadRequest%3E-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"hotshot_query_service/fetching/provider/trait.Provider.html\" title=\"trait hotshot_query_service::fetching::provider::Provider\">Provider</a>&lt;Types, <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.PayloadRequest.html\" title=\"struct hotshot_query_service::fetching::request::PayloadRequest\">PayloadRequest</a>&gt; for <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#100\">Source</a><a href=\"#method.fetch\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/fetching/provider/trait.Provider.html#tymethod.fetch\" class=\"fn\">fetch</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    req: <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.PayloadRequest.html\" title=\"struct hotshot_query_service::fetching::request::PayloadRequest\">PayloadRequest</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;Types as NodeType&gt;::BlockPayload&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Fetch a resource.</div></details></div></details>","Provider<Types, PayloadRequest>","sequencer::api::data_source::Provider"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Provider%3CTypes,+VidCommonRequest%3E-for-AnyProvider%3CTypes%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#121-123\">Source</a><a href=\"#impl-Provider%3CTypes,+VidCommonRequest%3E-for-AnyProvider%3CTypes%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types&gt; <a class=\"trait\" href=\"hotshot_query_service/fetching/provider/trait.Provider.html\" title=\"trait hotshot_query_service::fetching::provider::Provider\">Provider</a>&lt;Types, <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.VidCommonRequest.html\" title=\"struct hotshot_query_service::fetching::request::VidCommonRequest\">VidCommonRequest</a>&gt; for <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;<div class=\"where\">where\n    Types: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/fetching/provider/any.rs.html#120\">Source</a><a href=\"#method.fetch\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/fetching/provider/trait.Provider.html#tymethod.fetch\" class=\"fn\">fetch</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    req: <a class=\"struct\" href=\"hotshot_query_service/fetching/request/struct.VidCommonRequest.html\" title=\"struct hotshot_query_service::fetching::request::VidCommonRequest\">VidCommonRequest</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"enum\" href=\"hotshot_query_service/enum.VidCommon.html\" title=\"enum hotshot_query_service::VidCommon\">VidCommon</a>&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    <a class=\"struct\" href=\"hotshot_query_service/fetching/provider/any/struct.AnyProvider.html\" title=\"struct hotshot_query_service::fetching::provider::any::AnyProvider\">AnyProvider</a>&lt;Types&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Fetch a resource.</div></details></div></details>","Provider<Types, VidCommonRequest>","sequencer::api::data_source::Provider"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[21349]}