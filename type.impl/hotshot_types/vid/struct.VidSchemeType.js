(function() {
    var type_impls = Object.fromEntries([["hotshot_query_service",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Callback%3C%3CVidSchemeType+as+VidScheme%3E::Common%3E-for-VidCommonCallback%3CTypes,+S,+P%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fetching/vid.rs.html#238-250\">Source</a><a href=\"#impl-Callback%3C%3CVidSchemeType+as+VidScheme%3E::Common%3E-for-VidCommonCallback%3CTypes,+S,+P%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types: NodeType, S, P&gt; <a class=\"trait\" href=\"hotshot_query_service/fetching/trait.Callback.html\" title=\"trait hotshot_query_service::fetching::Callback\">Callback</a>&lt;&lt;VidSchemeType as VidScheme&gt;::Common&gt; for <a class=\"struct\" href=\"hotshot_query_service/data_source/fetching/vid/struct.VidCommonCallback.html\" title=\"struct hotshot_query_service::data_source::fetching::vid::VidCommonCallback\">VidCommonCallback</a>&lt;Types, S, P&gt;<div class=\"where\">where\n    <a class=\"type\" href=\"hotshot_query_service/type.Payload.html\" title=\"type hotshot_query_service::Payload\">Payload</a>&lt;Types&gt;: <a class=\"trait\" href=\"hotshot_query_service/availability/query_data/trait.QueryablePayload.html\" title=\"trait hotshot_query_service::availability::query_data::QueryablePayload\">QueryablePayload</a>&lt;Types&gt;,\n    S: <a class=\"trait\" href=\"hotshot_query_service/data_source/update/trait.VersionedDataSource.html\" title=\"trait hotshot_query_service::data_source::update::VersionedDataSource\">VersionedDataSource</a> + 'static,\n    for&lt;'a&gt; S::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/update/trait.VersionedDataSource.html#associatedtype.Transaction\" title=\"type hotshot_query_service::data_source::update::VersionedDataSource::Transaction\">Transaction</a>&lt;'a&gt;: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/trait.UpdateAvailabilityStorage.html\" title=\"trait hotshot_query_service::data_source::storage::UpdateAvailabilityStorage\">UpdateAvailabilityStorage</a>&lt;Types&gt;,\n    P: <a class=\"trait\" href=\"hotshot_query_service/data_source/fetching/trait.AvailabilityProvider.html\" title=\"trait hotshot_query_service::data_source::fetching::AvailabilityProvider\">AvailabilityProvider</a>&lt;Types&gt;,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.run\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fetching/vid.rs.html#245-249\">Source</a><a href=\"#method.run\" class=\"anchor\">§</a><h4 class=\"code-header\">async fn <a href=\"hotshot_query_service/fetching/trait.Callback.html#tymethod.run\" class=\"fn\">run</a>(self, common: <a class=\"type\" href=\"hotshot_query_service/type.VidCommon.html\" title=\"type hotshot_query_service::VidCommon\">VidCommon</a>)</h4></section></div></details>","Callback<<VidSchemeType as VidScheme>::Common>","hotshot_query_service::VidCommitment","hotshot_query_service::VidCommon","hotshot_query_service::VidShare"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-VidSchemeType\" class=\"impl\"><a href=\"#impl-Clone-for-VidSchemeType\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for VidSchemeType</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; VidSchemeType</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.84.1/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","hotshot_query_service::VidCommitment","hotshot_query_service::VidCommon","hotshot_query_service::VidShare"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PayloadProver%3CLargeRangeProofType%3E-for-VidSchemeType\" class=\"impl\"><a href=\"#impl-PayloadProver%3CLargeRangeProofType%3E-for-VidSchemeType\" class=\"anchor\">§</a><h3 class=\"code-header\">impl PayloadProver&lt;LargeRangeProofType&gt; for VidSchemeType</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.payload_proof\" class=\"method trait-impl\"><a href=\"#method.payload_proof\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">payload_proof</a>&lt;B&gt;(\n    &amp;self,\n    payload: B,\n    range: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/range/struct.Range.html\" title=\"struct core::ops::range::Range\">Range</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.usize.html\">usize</a>&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;LargeRangeProofType, VidError&gt;<div class=\"where\">where\n    B: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u8.html\">u8</a>]&gt;,</div></h4></section></summary><div class='docblock'>Compute a proof for a subslice of payload data. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.payload_verify\" class=\"method trait-impl\"><a href=\"#method.payload_verify\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">payload_verify</a>(\n    &amp;self,\n    stmt: Statement&lt;'_, VidSchemeType&gt;,\n    proof: &amp;LargeRangeProofType,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>&gt;, VidError&gt;</h4></section></summary><div class='docblock'>Verify a proof made by [<code>PayloadProver::payload_proof</code>]. <a>Read more</a></div></details></div></details>","PayloadProver<LargeRangeProofType>","hotshot_query_service::VidCommitment","hotshot_query_service::VidCommon","hotshot_query_service::VidShare"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PayloadProver%3CSmallRangeProofType%3E-for-VidSchemeType\" class=\"impl\"><a href=\"#impl-PayloadProver%3CSmallRangeProofType%3E-for-VidSchemeType\" class=\"anchor\">§</a><h3 class=\"code-header\">impl PayloadProver&lt;SmallRangeProofType&gt; for VidSchemeType</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.payload_proof\" class=\"method trait-impl\"><a href=\"#method.payload_proof\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">payload_proof</a>&lt;B&gt;(\n    &amp;self,\n    payload: B,\n    range: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/range/struct.Range.html\" title=\"struct core::ops::range::Range\">Range</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.usize.html\">usize</a>&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;SmallRangeProofType, VidError&gt;<div class=\"where\">where\n    B: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u8.html\">u8</a>]&gt;,</div></h4></section></summary><div class='docblock'>Compute a proof for a subslice of payload data. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.payload_verify\" class=\"method trait-impl\"><a href=\"#method.payload_verify\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">payload_verify</a>(\n    &amp;self,\n    stmt: Statement&lt;'_, VidSchemeType&gt;,\n    proof: &amp;SmallRangeProofType,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>&gt;, VidError&gt;</h4></section></summary><div class='docblock'>Verify a proof made by [<code>PayloadProver::payload_proof</code>]. <a>Read more</a></div></details></div></details>","PayloadProver<SmallRangeProofType>","hotshot_query_service::VidCommitment","hotshot_query_service::VidCommon","hotshot_query_service::VidShare"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-VidScheme-for-VidSchemeType\" class=\"impl\"><a href=\"#impl-VidScheme-for-VidSchemeType\" class=\"anchor\">§</a><h3 class=\"code-header\">impl VidScheme for VidSchemeType</h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Commit\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Commit\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Commit</a> = &lt;AdvzInternal&lt;Bn&lt;Config&gt;, CoreWrapper&lt;CtVariableCoreWrapper&lt;Sha256VarCore, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UTerm.html\" title=\"struct typenum::uint::UTerm\">UTerm</a>, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B1.html\" title=\"struct typenum::bit::B1\">B1</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, OidSha256&gt;&gt;, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>&gt; as VidScheme&gt;::Commit</h4></section></summary><div class='docblock'>Payload commitment.</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Share\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Share\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Share</a> = &lt;AdvzInternal&lt;Bn&lt;Config&gt;, CoreWrapper&lt;CtVariableCoreWrapper&lt;Sha256VarCore, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UTerm.html\" title=\"struct typenum::uint::UTerm\">UTerm</a>, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B1.html\" title=\"struct typenum::bit::B1\">B1</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, OidSha256&gt;&gt;, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>&gt; as VidScheme&gt;::Share</h4></section></summary><div class='docblock'>Share-specific data sent to a storage node.</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.Common\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Common\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Common</a> = &lt;AdvzInternal&lt;Bn&lt;Config&gt;, CoreWrapper&lt;CtVariableCoreWrapper&lt;Sha256VarCore, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UInt.html\" title=\"struct typenum::uint::UInt\">UInt</a>&lt;<a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/uint/struct.UTerm.html\" title=\"struct typenum::uint::UTerm\">UTerm</a>, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B1.html\" title=\"struct typenum::bit::B1\">B1</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, <a class=\"struct\" href=\"https://docs.rs/typenum/1.17.0/typenum/bit/struct.B0.html\" title=\"struct typenum::bit::B0\">B0</a>&gt;, OidSha256&gt;&gt;, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>&gt; as VidScheme&gt;::Common</h4></section></summary><div class='docblock'>Common data sent to all storage nodes.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.commit_only\" class=\"method trait-impl\"><a href=\"#method.commit_only\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">commit_only</a>&lt;B&gt;(\n    &amp;mut self,\n    payload: B,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;VidSchemeType as VidScheme&gt;::Commit, VidError&gt;<div class=\"where\">where\n    B: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u8.html\">u8</a>]&gt;,</div></h4></section></summary><div class='docblock'>Compute a payload commitment</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.disperse\" class=\"method trait-impl\"><a href=\"#method.disperse\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">disperse</a>&lt;B&gt;(\n    &amp;mut self,\n    payload: B,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;VidDisperse&lt;VidSchemeType&gt;, VidError&gt;<div class=\"where\">where\n    B: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/convert/trait.AsRef.html\" title=\"trait core::convert::AsRef\">AsRef</a>&lt;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u8.html\">u8</a>]&gt;,</div></h4></section></summary><div class='docblock'>Compute shares to send to the storage nodes</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.verify_share\" class=\"method trait-impl\"><a href=\"#method.verify_share\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">verify_share</a>(\n    &amp;self,\n    share: &amp;&lt;VidSchemeType as VidScheme&gt;::Share,\n    common: &amp;&lt;VidSchemeType as VidScheme&gt;::Common,\n    commit: &amp;&lt;VidSchemeType as VidScheme&gt;::Commit,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>, <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>&gt;, VidError&gt;</h4></section></summary><div class='docblock'>Verify a share. Used by both storage node and retrieval client.\nWhy is return type a nested <code>Result</code>? See <a href=\"https://sled.rs/errors\">https://sled.rs/errors</a>\nReturns: <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.recover_payload\" class=\"method trait-impl\"><a href=\"#method.recover_payload\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">recover_payload</a>(\n    &amp;self,\n    shares: &amp;[&lt;VidSchemeType as VidScheme&gt;::Share],\n    common: &amp;&lt;VidSchemeType as VidScheme&gt;::Common,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u8.html\">u8</a>&gt;, VidError&gt;</h4></section></summary><div class='docblock'>Recover payload from shares.\nDo not verify shares or check recovered payload against anything.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.is_consistent\" class=\"method trait-impl\"><a href=\"#method.is_consistent\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">is_consistent</a>(\n    commit: &amp;&lt;VidSchemeType as VidScheme&gt;::Commit,\n    common: &amp;&lt;VidSchemeType as VidScheme&gt;::Common,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>, VidError&gt;</h4></section></summary><div class='docblock'>Check that a [<code>VidScheme::Common</code>] is consistent with a\n[<code>VidScheme::Commit</code>]. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_payload_byte_len\" class=\"method trait-impl\"><a href=\"#method.get_payload_byte_len\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">get_payload_byte_len</a>(common: &amp;&lt;VidSchemeType as VidScheme&gt;::Common) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u32.html\">u32</a></h4></section></summary><div class='docblock'>Extract the payload byte length data from a [<code>VidScheme::Common</code>].</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_num_storage_nodes\" class=\"method trait-impl\"><a href=\"#method.get_num_storage_nodes\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">get_num_storage_nodes</a>(common: &amp;&lt;VidSchemeType as VidScheme&gt;::Common) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u32.html\">u32</a></h4></section></summary><div class='docblock'>Extract the number of storage nodes from a [<code>VidScheme::Common</code>].</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_multiplicity\" class=\"method trait-impl\"><a href=\"#method.get_multiplicity\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">get_multiplicity</a>(common: &amp;&lt;VidSchemeType as VidScheme&gt;::Common) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u32.html\">u32</a></h4></section></summary><div class='docblock'>Extract the number of poly evals per share [<code>VidScheme::Common</code>].</div></details></div></details>","VidScheme","hotshot_query_service::VidCommitment","hotshot_query_service::VidCommon","hotshot_query_service::VidShare"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[24083]}