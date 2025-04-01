(function() {
    var type_impls = Object.fromEntries([["sequencer",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-ConsensusApi%3CTYPES,+I%3E-for-SystemContextHandle%3CTYPES,+I,+V%3E\" class=\"impl\"><a href=\"#impl-ConsensusApi%3CTYPES,+I%3E-for-SystemContextHandle%3CTYPES,+I,+V%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES, I, V&gt; ConsensusApi&lt;TYPES, I&gt; for SystemContextHandle&lt;TYPES, I, V&gt;<div class=\"where\">where\n    TYPES: NodeType,\n    I: NodeImplementation&lt;TYPES&gt;,\n    V: Versions,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.total_nodes\" class=\"method trait-impl\"><a href=\"#method.total_nodes\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">total_nodes</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/num/nonzero/struct.NonZero.html\" title=\"struct core::num::nonzero::NonZero\">NonZero</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.usize.html\">usize</a>&gt;</h4></section></summary><div class='docblock'>Total number of nodes in the network. Also known as <code>n</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.builder_timeout\" class=\"method trait-impl\"><a href=\"#method.builder_timeout\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">builder_timeout</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a></h4></section></summary><div class='docblock'>The maximum amount of time a leader can wait to get a block from a builder.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.send_event\" class=\"method trait-impl\"><a href=\"#method.send_event\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">send_event</a>&lt;'life0, 'async_trait&gt;(\n    &amp;'life0 self,\n    event: Event&lt;TYPES&gt;,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    SystemContextHandle&lt;TYPES, I, V&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Notify the system of an event within <code>hotshot-consensus</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.public_key\" class=\"method trait-impl\"><a href=\"#method.public_key\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">public_key</a>(&amp;self) -&gt; &amp;&lt;TYPES as NodeType&gt;::SignatureKey</h4></section></summary><div class='docblock'>Get a reference to the public key.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.private_key\" class=\"method trait-impl\"><a href=\"#method.private_key\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">private_key</a>(\n    &amp;self,\n) -&gt; &amp;&lt;&lt;TYPES as NodeType&gt;::SignatureKey as SignatureKey&gt;::PrivateKey</h4></section></summary><div class='docblock'>Get a reference to the private key.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.state_private_key\" class=\"method trait-impl\"><a href=\"#method.state_private_key\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">state_private_key</a>(\n    &amp;self,\n) -&gt; &amp;&lt;&lt;TYPES as NodeType&gt;::StateSignatureKey as StateSignatureKey&gt;::StatePrivateKey</h4></section></summary><div class='docblock'>Get a reference to the light client signing key.</div></details></div></details>","ConsensusApi<TYPES, I>","sequencer::context::Consensus"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-SystemContextHandle%3CTYPES,+I,+V%3E\" class=\"impl\"><a href=\"#impl-SystemContextHandle%3CTYPES,+I,+V%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;TYPES, I, V&gt; SystemContextHandle&lt;TYPES, I, V&gt;<div class=\"where\">where\n    TYPES: NodeType,\n    I: NodeImplementation&lt;TYPES&gt; + 'static,\n    V: Versions,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.add_task\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">add_task</a>&lt;S&gt;(&amp;mut self, task_state: S)<div class=\"where\">where\n    S: TaskState&lt;Event = HotShotEvent&lt;TYPES&gt;&gt; + 'static,</div></h4></section></summary><div class=\"docblock\"><p>Adds a hotshot consensus-related task to the <code>SystemContextHandle</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.event_stream\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">event_stream</a>(&amp;self) -&gt; impl Stream&lt;Item = Event&lt;TYPES&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>obtains a stream to expose to the user</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.send_external_message\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">send_external_message</a>(\n    &amp;self,\n    msg: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u8.html\">u8</a>&gt;,\n    recipients: RecipientList&lt;&lt;TYPES as NodeType&gt;::SignatureKey&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Message other participants with a serialized message from the application\nReceivers of this message will get an <code>Event::ExternalMessageReceived</code> via\nthe event stream.</p>\n<h5 id=\"errors\"><a class=\"doc-anchor\" href=\"#errors\">§</a>Errors</h5>\n<p>Errors if serializing the request fails, or the request fails to be sent</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.request_proposal\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">request_proposal</a>(\n    &amp;self,\n    view: &lt;TYPES as NodeType&gt;::View,\n    leaf_commitment: Commitment&lt;Leaf2&lt;TYPES&gt;&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Proposal&lt;TYPES, QuorumProposalWrapper&lt;TYPES&gt;&gt;, <a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;&gt;, <a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Request a proposal from the all other nodes.  Will block until some node\nreturns a valid proposal with the requested commitment.  If nobody has the\nproposal this will block forever</p>\n<h5 id=\"errors-1\"><a class=\"doc-anchor\" href=\"#errors-1\">§</a>Errors</h5>\n<p>Errors if signing the request for proposal fails</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.event_stream_known_impl\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">event_stream_known_impl</a>(&amp;self) -&gt; Receiver&lt;Event&lt;TYPES&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>HACK so we can know the types when running tests…\nthere are two cleaner solutions:</p>\n<ul>\n<li>make the stream generic and in nodetypes or nodeimpelmentation</li>\n<li>type wrapper</li>\n</ul>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.internal_event_stream_sender\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">internal_event_stream_sender</a>(&amp;self) -&gt; Sender&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;HotShotEvent&lt;TYPES&gt;&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>HACK so we can create dependency tasks when running tests</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.internal_event_stream_receiver_known_impl\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">internal_event_stream_receiver_known_impl</a>(\n    &amp;self,\n) -&gt; Receiver&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;HotShotEvent&lt;TYPES&gt;&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>HACK so we can know the types when running tests…\nthere are two cleaner solutions:</p>\n<ul>\n<li>make the stream generic and in nodetypes or nodeimpelmentation</li>\n<li>type wrapper</li>\n</ul>\n<p>NOTE: this is only used for sanity checks in our tests</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.decided_state\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">decided_state</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;&lt;TYPES as NodeType&gt;::ValidatedState&gt;</h4></section></summary><div class=\"docblock\"><p>Get the last decided validated state of the [<code>SystemContext</code>] instance.</p>\n<h5 id=\"panics\"><a class=\"doc-anchor\" href=\"#panics\">§</a>Panics</h5>\n<p>If the internal consensus is in an inconsistent state.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.state\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">state</a>(\n    &amp;self,\n    view: &lt;TYPES as NodeType&gt;::View,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;&lt;TYPES as NodeType&gt;::ValidatedState&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Get the validated state from a given <code>view</code>.</p>\n<p>Returns the requested state, if the [<code>SystemContext</code>] is tracking this view. Consensus\ntracks views that have not yet been decided but could be in the future. This function may\nreturn <a href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html#variant.None\" title=\"variant core::option::Option::None\"><code>None</code></a> if the requested view has already been decided (but see\n<a href=\"Self::decided_state\"><code>decided_state</code></a>) or if there is no path for the requested\nview to ever be decided.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.decided_leaf\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">decided_leaf</a>(&amp;self) -&gt; Leaf2&lt;TYPES&gt;</h4></section></summary><div class=\"docblock\"><p>Get the last decided leaf of the [<code>SystemContext</code>] instance.</p>\n<h5 id=\"panics-1\"><a class=\"doc-anchor\" href=\"#panics-1\">§</a>Panics</h5>\n<p>If the internal consensus is in an inconsistent state.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_decided_leaf\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_decided_leaf</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;Leaf2&lt;TYPES&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Tries to get the most recent decided leaf, returning instantly\nif we can’t acquire the lock.</p>\n<h5 id=\"panics-2\"><a class=\"doc-anchor\" href=\"#panics-2\">§</a>Panics</h5>\n<p>Panics if internal consensus is in an inconsistent state.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.submit_transaction\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">submit_transaction</a>(\n    &amp;self,\n    tx: &lt;TYPES as NodeType&gt;::Transaction,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, HotShotError&lt;TYPES&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Submits a transaction to the backing [<code>SystemContext</code>] instance.</p>\n<p>The current node broadcasts the transaction to all nodes on the network.</p>\n<h5 id=\"errors-2\"><a class=\"doc-anchor\" href=\"#errors-2\">§</a>Errors</h5>\n<p>Will return a [<code>HotShotError</code>] if some error occurs in the underlying\n[<code>SystemContext</code>] instance.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.consensus\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">consensus</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;Consensus&lt;TYPES&gt;&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Get the underlying consensus state for this [<code>SystemContext</code>]</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.shut_down\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">shut_down</a>(&amp;mut self)</h4></section></summary><div class=\"docblock\"><p>Shut down the inner hotshot and wait until all background threads are closed.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.next_view_timeout\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">next_view_timeout</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.u64.html\">u64</a></h4></section></summary><div class=\"docblock\"><p>return the timeout for a view of the underlying <code>SystemContext</code></p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.leader\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">leader</a>(\n    &amp;self,\n    view_number: &lt;TYPES as NodeType&gt;::View,\n    epoch_number: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;TYPES as NodeType&gt;::Epoch&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;TYPES as NodeType&gt;::SignatureKey, <a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Wrapper for <code>HotShotConsensusApi</code>’s <code>leader</code> function</p>\n<h5 id=\"errors-3\"><a class=\"doc-anchor\" href=\"#errors-3\">§</a>Errors</h5>\n<p>Returns an error if the leader cannot be calculated</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.public_key\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">public_key</a>(&amp;self) -&gt; &lt;TYPES as NodeType&gt;::SignatureKey</h4></section></summary><div class=\"docblock\"><p>Wrapper to get this node’s public key</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.external_channel_sender\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">external_channel_sender</a>(&amp;self) -&gt; Sender&lt;Event&lt;TYPES&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Get the sender side of the external event stream for testing purpose</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.internal_channel_sender\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">internal_channel_sender</a>(&amp;self) -&gt; Sender&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;HotShotEvent&lt;TYPES&gt;&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Get the sender side of the internal event stream for testing purpose</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.cur_view\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">cur_view</a>(&amp;self) -&gt; &lt;TYPES as NodeType&gt;::View</h4></section></summary><div class=\"docblock\"><p>Wrapper to get the view number this node is on.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.cur_epoch\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">cur_epoch</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;TYPES as NodeType&gt;::Epoch&gt;</h4></section></summary><div class=\"docblock\"><p>Wrapper to get the epoch number this node is on.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.storage\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">storage</a>(&amp;self) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;&lt;I as NodeImplementation&lt;TYPES&gt;&gt;::Storage&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Provides a reference to the underlying storage for this [<code>SystemContext</code>], allowing access to\nhistorical data</p>\n</div></details></div></details>",0,"sequencer::context::Consensus"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[19393]}