(function() {
    var type_impls = Object.fromEntries([["hotshot_task_impls",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-BuilderClient%3CTYPES,+Ver%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_task_impls/builder.rs.html#70-127\">Source</a><a href=\"#impl-BuilderClient%3CTYPES,+Ver%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES: NodeType, Ver: StaticVersionType&gt; <a class=\"struct\" href=\"hotshot_task_impls/builder/struct.BuilderClient.html\" title=\"struct hotshot_task_impls::builder::BuilderClient\">BuilderClient</a>&lt;TYPES, Ver&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_task_impls/builder.rs.html#76-85\">Source</a><h4 class=\"code-header\">pub fn <a href=\"hotshot_task_impls/builder/struct.BuilderClient.html#tymethod.new\" class=\"fn\">new</a>(base_url: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/convert/trait.Into.html\" title=\"trait core::convert::Into\">Into</a>&lt;<a class=\"struct\" href=\"https://docs.rs/url/2.5.4/url/struct.Url.html\" title=\"struct url::Url\">Url</a>&gt;) -&gt; Self</h4></section></summary><div class=\"docblock\"><p>Construct a new client from base url</p>\n<h5 id=\"panics\"><a class=\"doc-anchor\" href=\"#panics\">§</a>Panics</h5>\n<p>If the URL is malformed.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.connect\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_task_impls/builder.rs.html#90-104\">Source</a><h4 class=\"code-header\">pub async fn <a href=\"hotshot_task_impls/builder/struct.BuilderClient.html#tymethod.connect\" class=\"fn\">connect</a>(&amp;self, timeout: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class=\"docblock\"><p>Wait for server to become available\nReturns <code>false</code> if server doesn’t respond\nwith OK healthcheck before <code>timeout</code></p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.available_blocks\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_task_impls/builder.rs.html#111-126\">Source</a><h4 class=\"code-header\">pub async fn <a href=\"hotshot_task_impls/builder/struct.BuilderClient.html#tymethod.available_blocks\" class=\"fn\">available_blocks</a>(\n    &amp;self,\n    parent: VidCommitment,\n    view_number: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u64.html\">u64</a>,\n    sender: TYPES::SignatureKey,\n    signature: &amp;&lt;&lt;TYPES as NodeType&gt;::SignatureKey as SignatureKey&gt;::PureAssembledSignatureType,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"hotshot_builder_api/v0_1/block_info/struct.AvailableBlockInfo.html\" title=\"struct hotshot_builder_api::v0_1::block_info::AvailableBlockInfo\">AvailableBlockInfo</a>&lt;TYPES&gt;&gt;, <a class=\"enum\" href=\"hotshot_task_impls/builder/enum.BuilderClientError.html\" title=\"enum hotshot_task_impls::builder::BuilderClientError\">BuilderClientError</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Query builder for available blocks</p>\n<h5 id=\"errors\"><a class=\"doc-anchor\" href=\"#errors\">§</a>Errors</h5>\n<ul>\n<li><a href=\"hotshot_task_impls/builder/enum.BuilderClientError.html#variant.BlockNotFound\" title=\"variant hotshot_task_impls::builder::BuilderClientError::BlockNotFound\"><code>BuilderClientError::BlockNotFound</code></a> if blocks aren’t available for this parent</li>\n<li><a href=\"hotshot_task_impls/builder/enum.BuilderClientError.html#variant.Api\" title=\"variant hotshot_task_impls::builder::BuilderClientError::Api\"><code>BuilderClientError::Api</code></a> if API isn’t responding or responds incorrectly</li>\n</ul>\n</div></details></div></details>",0,"hotshot_task_impls::builder::v0_1::BuilderClient","hotshot_task_impls::builder::v0_99::BuilderClient"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[4454]}