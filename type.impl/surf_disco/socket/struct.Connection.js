(function() {
    var type_impls = Object.fromEntries([["marketplace_builder_shared",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Drop-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Drop-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    VER: StaticVersionType,\n    ToServer: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.drop\" class=\"method trait-impl\"><a href=\"#method.drop\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html#tymethod.drop\" class=\"fn\">drop</a>(&amp;mut self)</h4></section></summary><div class='docblock'>Executes the destructor for this type. <a href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html#tymethod.drop\">Read more</a></div></details></div></details>","Drop","marketplace_builder_shared::utils::event_service_wrapper::EventServiceConnection"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Sink%3C%26ToServer%3E-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Sink%3C%26ToServer%3E-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt; for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    ToServer: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,\n    E: Error,\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Error\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Error\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Error</a> = E</h4></section></summary><div class='docblock'>The type of value produced by the sink when an error occurs.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_ready\" class=\"method trait-impl\"><a href=\"#method.poll_ready\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_ready</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Attempts to prepare the <code>Sink</code> to receive a value. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.start_send\" class=\"method trait-impl\"><a href=\"#method.start_send\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">start_send</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    item: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;</h4></section></summary><div class='docblock'>Begin the process of sending a value to the sink.\nEach call to this function must be preceded by a successful call to\n<code>poll_ready</code> which returned <code>Poll::Ready(Ok(()))</code>. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_flush\" class=\"method trait-impl\"><a href=\"#method.poll_flush\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_flush</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output from this sink. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_close\" class=\"method trait-impl\"><a href=\"#method.poll_close\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_close</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output and close this sink, if necessary. <a>Read more</a></div></details></div></details>","Sink<&ToServer>","marketplace_builder_shared::utils::event_service_wrapper::EventServiceConnection"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Stream-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Stream-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; Stream for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    FromServer: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,\n    E: Error,\n    VER: StaticVersionType,\n    ToServer: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Item\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Item\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Item</a> = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;FromServer, E&gt;</h4></section></summary><div class='docblock'>Values yielded by the stream.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_next\" class=\"method trait-impl\"><a href=\"#method.poll_next\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_next</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Stream&gt;::Item&gt;&gt;</h4></section></summary><div class='docblock'>Attempt to pull out the next value of this stream, registering the\ncurrent task for wakeup if the value is not yet available, and returning\n<code>None</code> if the stream is exhausted. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.size_hint\" class=\"method trait-impl\"><a href=\"#method.size_hint\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">size_hint</a>(&amp;self) -&gt; (<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a>, <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a>&gt;)</h4></section></summary><div class='docblock'>Returns the bounds on the remaining length of the stream. <a>Read more</a></div></details></div></details>","Stream","marketplace_builder_shared::utils::event_service_wrapper::EventServiceConnection"]]],["nasty_client",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Drop-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Drop-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    VER: StaticVersionType,\n    ToServer: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.drop\" class=\"method trait-impl\"><a href=\"#method.drop\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html#tymethod.drop\" class=\"fn\">drop</a>(&amp;mut self)</h4></section></summary><div class='docblock'>Executes the destructor for this type. <a href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html#tymethod.drop\">Read more</a></div></details></div></details>","Drop","nasty_client::Connection"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Sink%3C%26ToServer%3E-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Sink%3C%26ToServer%3E-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt; for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    ToServer: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,\n    E: Error,\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Error\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Error\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Error</a> = E</h4></section></summary><div class='docblock'>The type of value produced by the sink when an error occurs.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_ready\" class=\"method trait-impl\"><a href=\"#method.poll_ready\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_ready</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Attempts to prepare the <code>Sink</code> to receive a value. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.start_send\" class=\"method trait-impl\"><a href=\"#method.start_send\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">start_send</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    item: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;</h4></section></summary><div class='docblock'>Begin the process of sending a value to the sink.\nEach call to this function must be preceded by a successful call to\n<code>poll_ready</code> which returned <code>Poll::Ready(Ok(()))</code>. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_flush\" class=\"method trait-impl\"><a href=\"#method.poll_flush\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_flush</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output from this sink. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_close\" class=\"method trait-impl\"><a href=\"#method.poll_close\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_close</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output and close this sink, if necessary. <a>Read more</a></div></details></div></details>","Sink<&ToServer>","nasty_client::Connection"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Stream-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Stream-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; Stream for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    FromServer: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,\n    E: Error,\n    VER: StaticVersionType,\n    ToServer: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Item\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Item\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Item</a> = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;FromServer, E&gt;</h4></section></summary><div class='docblock'>Values yielded by the stream.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_next\" class=\"method trait-impl\"><a href=\"#method.poll_next\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_next</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Stream&gt;::Item&gt;&gt;</h4></section></summary><div class='docblock'>Attempt to pull out the next value of this stream, registering the\ncurrent task for wakeup if the value is not yet available, and returning\n<code>None</code> if the stream is exhausted. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.size_hint\" class=\"method trait-impl\"><a href=\"#method.size_hint\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">size_hint</a>(&amp;self) -&gt; (<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a>, <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a>&gt;)</h4></section></summary><div class='docblock'>Returns the bounds on the remaining length of the stream. <a>Read more</a></div></details></div></details>","Stream","nasty_client::Connection"]]],["node_metrics",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Drop-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Drop-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html\" title=\"trait core::ops::drop::Drop\">Drop</a> for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    VER: StaticVersionType,\n    ToServer: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.drop\" class=\"method trait-impl\"><a href=\"#method.drop\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html#tymethod.drop\" class=\"fn\">drop</a>(&amp;mut self)</h4></section></summary><div class='docblock'>Executes the destructor for this type. <a href=\"https://doc.rust-lang.org/1.86.0/core/ops/drop/trait.Drop.html#tymethod.drop\">Read more</a></div></details></div></details>","Drop","node_metrics::api::node_validator::v0::AvailabilityConnection"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Sink%3C%26ToServer%3E-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Sink%3C%26ToServer%3E-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt; for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    ToServer: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,\n    E: Error,\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Error\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Error\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Error</a> = E</h4></section></summary><div class='docblock'>The type of value produced by the sink when an error occurs.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_ready\" class=\"method trait-impl\"><a href=\"#method.poll_ready\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_ready</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Attempts to prepare the <code>Sink</code> to receive a value. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.start_send\" class=\"method trait-impl\"><a href=\"#method.start_send\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">start_send</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    item: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;</h4></section></summary><div class='docblock'>Begin the process of sending a value to the sink.\nEach call to this function must be preceded by a successful call to\n<code>poll_ready</code> which returned <code>Poll::Ready(Ok(()))</code>. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_flush\" class=\"method trait-impl\"><a href=\"#method.poll_flush\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_flush</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output from this sink. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_close\" class=\"method trait-impl\"><a href=\"#method.poll_close\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_close</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.unit.html\">()</a>, &lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Sink&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.reference.html\">&amp;ToServer</a>&gt;&gt;::Error&gt;&gt;</h4></section></summary><div class='docblock'>Flush any remaining output and close this sink, if necessary. <a>Read more</a></div></details></div></details>","Sink<&ToServer>","node_metrics::api::node_validator::v0::AvailabilityConnection"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Stream-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"impl\"><a href=\"#impl-Stream-for-Connection%3CFromServer,+ToServer,+E,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;FromServer, ToServer, E, VER&gt; Stream for Connection&lt;FromServer, ToServer, E, VER&gt;<div class=\"where\">where\n    FromServer: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.219/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,\n    E: Error,\n    VER: StaticVersionType,\n    ToServer: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Item\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Item\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">Item</a> = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;FromServer, E&gt;</h4></section></summary><div class='docblock'>Values yielded by the stream.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.poll_next\" class=\"method trait-impl\"><a href=\"#method.poll_next\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">poll_next</a>(\n    self: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;&amp;mut Connection&lt;FromServer, ToServer, E, VER&gt;&gt;,\n    cx: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.86.0/core/task/wake/struct.Context.html\" title=\"struct core::task::wake::Context\">Context</a>&lt;'_&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/task/poll/enum.Poll.html\" title=\"enum core::task::poll::Poll\">Poll</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;Connection&lt;FromServer, ToServer, E, VER&gt; as Stream&gt;::Item&gt;&gt;</h4></section></summary><div class='docblock'>Attempt to pull out the next value of this stream, registering the\ncurrent task for wakeup if the value is not yet available, and returning\n<code>None</code> if the stream is exhausted. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.size_hint\" class=\"method trait-impl\"><a href=\"#method.size_hint\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">size_hint</a>(&amp;self) -&gt; (<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a>, <a class=\"enum\" href=\"https://doc.rust-lang.org/1.86.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/std/primitive.usize.html\">usize</a>&gt;)</h4></section></summary><div class='docblock'>Returns the bounds on the remaining length of the stream. <a>Read more</a></div></details></div></details>","Stream","node_metrics::api::node_validator::v0::AvailabilityConnection"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[11509,11328,11439]}