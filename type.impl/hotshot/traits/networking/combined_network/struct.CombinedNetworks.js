(function() {
    var type_impls = Object.fromEntries([["sequencer",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-CombinedNetworks%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-Clone-for-CombinedNetworks%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for CombinedNetworks&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; CombinedNetworks&lt;TYPES&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.1/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","sequencer::network::Production"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-CombinedNetworks%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-CombinedNetworks%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; CombinedNetworks&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">new</a>(\n    primary_network: PushCdnNetwork&lt;&lt;TYPES as NodeType&gt;::SignatureKey&gt;,\n    secondary_network: Libp2pNetwork&lt;TYPES&gt;,\n    delay_duration: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a>&gt;,\n) -&gt; CombinedNetworks&lt;TYPES&gt;</h4></section></summary><div class=\"docblock\"><p>Constructor</p>\n<h5 id=\"panics\"><a class=\"doc-anchor\" href=\"#panics\">§</a>Panics</h5>\n<p>Panics if <code>COMBINED_NETWORK_CACHE_SIZE</code> is 0</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.primary\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">primary</a>(&amp;self) -&gt; &amp;PushCdnNetwork&lt;&lt;TYPES as NodeType&gt;::SignatureKey&gt;</h4></section></summary><div class=\"docblock\"><p>Get a ref to the primary network</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.secondary\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">secondary</a>(&amp;self) -&gt; &amp;Libp2pNetwork&lt;TYPES&gt;</h4></section></summary><div class=\"docblock\"><p>Get a ref to the backup network</p>\n</div></details></div></details>",0,"sequencer::network::Production"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-ConnectedNetwork%3C%3CTYPES+as+NodeType%3E::SignatureKey%3E-for-CombinedNetworks%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-ConnectedNetwork%3C%3CTYPES+as+NodeType%3E::SignatureKey%3E-for-CombinedNetworks%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; ConnectedNetwork&lt;&lt;TYPES as NodeType&gt;::SignatureKey&gt; for CombinedNetworks&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.recv_message\" class=\"method trait-impl\"><a href=\"#method.recv_message\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">recv_message</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>&gt;, NetworkError&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    CombinedNetworks&lt;TYPES&gt;: 'async_trait,</div></h4></section></summary><div class=\"docblock\"><p>Receive one or many messages from the underlying network.</p>\n<h5 id=\"errors\"><a class=\"doc-anchor\" href=\"#errors\">§</a>Errors</h5>\n<p>Does not error</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.pause\" class=\"method trait-impl\"><a href=\"#method.pause\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">pause</a>(&amp;self)</h4></section></summary><div class='docblock'>Pauses the underlying network</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.resume\" class=\"method trait-impl\"><a href=\"#method.resume\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">resume</a>(&amp;self)</h4></section></summary><div class='docblock'>Resumes the underlying network</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.wait_for_ready\" class=\"method trait-impl\"><a href=\"#method.wait_for_ready\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">wait_for_ready</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    CombinedNetworks&lt;TYPES&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Blocks until the network is successfully initialized</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.shut_down\" class=\"method trait-impl\"><a href=\"#method.shut_down\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">shut_down</a>&lt;'a, 'b&gt;(\n    &amp;'a self,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + 'b&gt;&gt;<div class=\"where\">where\n    'a: 'b,\n    CombinedNetworks&lt;TYPES&gt;: 'b,</div></h4></section></summary><div class='docblock'>Blocks until the network is shut down</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.broadcast_message\" class=\"method trait-impl\"><a href=\"#method.broadcast_message\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">broadcast_message</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    message: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>&gt;,\n    topic: Topic,\n    broadcast_delay: BroadcastDelay,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, NetworkError&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    CombinedNetworks&lt;TYPES&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>broadcast message to some subset of nodes\nblocking</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.da_broadcast_message\" class=\"method trait-impl\"><a href=\"#method.da_broadcast_message\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">da_broadcast_message</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    message: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>&gt;,\n    recipients: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;&lt;TYPES as NodeType&gt;::SignatureKey&gt;,\n    broadcast_delay: BroadcastDelay,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, NetworkError&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    CombinedNetworks&lt;TYPES&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>broadcast a message only to a DA committee\nblocking</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.direct_message\" class=\"method trait-impl\"><a href=\"#method.direct_message\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">direct_message</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    message: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>&gt;,\n    recipient: &lt;TYPES as NodeType&gt;::SignatureKey,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, NetworkError&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    CombinedNetworks&lt;TYPES&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Sends a direct message to a specific node\nblocking</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.vid_broadcast_message\" class=\"method trait-impl\"><a href=\"#method.vid_broadcast_message\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">vid_broadcast_message</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    messages: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/std/collections/hash/map/struct.HashMap.html\" title=\"struct std::collections::hash::map::HashMap\">HashMap</a>&lt;&lt;TYPES as NodeType&gt;::SignatureKey, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>&gt;&gt;,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, NetworkError&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    CombinedNetworks&lt;TYPES&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>send messages with vid shares to its recipients\nblocking</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.queue_node_lookup\" class=\"method trait-impl\"><a href=\"#method.queue_node_lookup\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">queue_node_lookup</a>(\n    &amp;self,\n    view_number: ViewNumber,\n    pk: &lt;TYPES as NodeType&gt;::SignatureKey,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, TrySendError&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;(ViewNumber, &lt;TYPES as NodeType&gt;::SignatureKey)&gt;&gt;&gt;</h4></section></summary><div class='docblock'>queues lookup of a node <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.update_view\" class=\"method trait-impl\"><a href=\"#method.update_view\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">update_view</a>&lt;'a, 'async_trait, T&gt;(\n    &amp;'a self,\n    view: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u64.html\">u64</a>,\n    epoch: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u64.html\">u64</a>&gt;,\n    membership: EpochMembershipCoordinator&lt;T&gt;,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'a: 'async_trait,\n    T: NodeType&lt;SignatureKey = &lt;TYPES as NodeType&gt;::SignatureKey&gt; + 'a + 'async_trait,\n    CombinedNetworks&lt;TYPES&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Update view can be used for any reason, but mostly it’s for canceling tasks,\nand looking up the address of the leader of a future view.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.is_primary_down\" class=\"method trait-impl\"><a href=\"#method.is_primary_down\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">is_primary_down</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Is primary network down? Makes sense only for combined network</div></details></div></details>","ConnectedNetwork<<TYPES as NodeType>::SignatureKey>","sequencer::network::Production"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-TestableNetworkingImplementation%3CTYPES%3E-for-CombinedNetworks%3CTYPES%3E\" class=\"impl\"><a href=\"#impl-TestableNetworkingImplementation%3CTYPES%3E-for-CombinedNetworks%3CTYPES%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES&gt; TestableNetworkingImplementation&lt;TYPES&gt; for CombinedNetworks&lt;TYPES&gt;<div class=\"where\">where\n    TYPES: NodeType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.in_flight_message_count\" class=\"method trait-impl\"><a href=\"#method.in_flight_message_count\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">in_flight_message_count</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.usize.html\">usize</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Get the number of messages in-flight.</p>\n<p>Some implementations will not be able to tell how many messages there are in-flight. These implementations should return <code>None</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.generator\" class=\"method trait-impl\"><a href=\"#method.generator\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">generator</a>(\n    expected_node_count: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.usize.html\">usize</a>,\n    num_bootstrap: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.usize.html\">usize</a>,\n    network_id: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.usize.html\">usize</a>,\n    da_committee_size: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.usize.html\">usize</a>,\n    reliability_config: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn NetworkReliability&gt;&gt;,\n    secondary_network_delay: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/ops/function/trait.Fn.html\" title=\"trait core::ops::function::Fn\">Fn</a>(<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u64.html\">u64</a>) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;CombinedNetworks&lt;TYPES&gt;&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a>&gt;&gt;</h4></section></summary><div class='docblock'>generates a network given an expected node count</div></details></div></details>","TestableNetworkingImplementation<TYPES>","sequencer::network::Production"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[24049]}