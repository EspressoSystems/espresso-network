(function() {
    var type_impls = Object.fromEntries([["espresso_types",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Client%3CE,+VER%3E\" class=\"impl\"><a href=\"#impl-Client%3CE,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, VER&gt; Client&lt;E, VER&gt;<div class=\"where\">where\n    E: Error,\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">new</a>(base_url: <a class=\"struct\" href=\"https://docs.rs/url/2.5.4/url/struct.Url.html\" title=\"struct url::Url\">Url</a>) -&gt; Client&lt;E, VER&gt;</h4></section></summary><div class=\"docblock\"><p>Create a client and connect to the Tide Disco server at <code>base_url</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.builder\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">builder</a>(base_url: <a class=\"struct\" href=\"https://docs.rs/url/2.5.4/url/struct.Url.html\" title=\"struct url::Url\">Url</a>) -&gt; ClientBuilder&lt;E, VER&gt;</h4></section></summary><div class=\"docblock\"><p>Create a client with customization.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.connect\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">connect</a>(&amp;self, timeout: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a>&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class=\"docblock\"><p>Connect to the server, retrying if the server is not running.</p>\n<p>It is not necessary to call this function when creating a new client. The client will\nautomatically connect when a request is made, if the server is available. However, this can\nbe useful to wait for the server to come up, if the server may be offline when the client is\ncreated.</p>\n<p>This function will make an HTTP <code>GET</code> request to the server’s <code>/healthcheck</code> endpoint, to\ntest if the server is available. If this request succeeds, <a href=\"Self::connect\">connect</a> returns\n<code>true</code>. Otherwise, the client will continue retrying <code>/healthcheck</code> requests until <code>timeout</code>\nhas elapsed (or forever, if <code>timeout</code> is <code>None</code>). If the timeout expires before a\n<code>/healthcheck</code> request succeeds, <a href=\"Self::connect\">connect</a> will return <code>false</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.wait_for_health\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">wait_for_health</a>&lt;H&gt;(\n    &amp;self,\n    healthy: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/ops/function/trait.Fn.html\" title=\"trait core::ops::function::Fn\">Fn</a>(<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.reference.html\">&amp;H</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.bool.html\">bool</a>,\n    timeout: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a>&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;H&gt;<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a> + HealthCheck,</div></h4></section></summary><div class=\"docblock\"><p>Connect to the server, retrying until the server is <code>healthy</code>.</p>\n<p>This function is similar to <a href=\"Self::connect\">connect</a>. It will make requests to the\n<code>/healthcheck</code> endpoint until a request succeeds. However, it will then continue retrying\nuntil the response from <code>/healthcheck</code> satisfies the <code>healthy</code> predicate.</p>\n<p>On success, returns the response from <code>/healthcheck</code>. On timeout, returns <code>None</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">get</a>&lt;T&gt;(&amp;self, route: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>) -&gt; Request&lt;T, E, VER&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,</div></h4></section></summary><div class=\"docblock\"><p>Build an HTTP <code>GET</code> request.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.post\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">post</a>&lt;T&gt;(&amp;self, route: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>) -&gt; Request&lt;T, E, VER&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,</div></h4></section></summary><div class=\"docblock\"><p>Build an HTTP <code>POST</code> request.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.healthcheck\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">healthcheck</a>&lt;H&gt;(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;H, E&gt;<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a> + HealthCheck,</div></h4></section></summary><div class=\"docblock\"><p>Query the server’s healthcheck endpoint.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.request\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">request</a>&lt;T&gt;(&amp;self, method: Method, route: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>) -&gt; Request&lt;T, E, VER&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,</div></h4></section></summary><div class=\"docblock\"><p>Build an HTTP request with the specified method.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.socket\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">socket</a>(&amp;self, route: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>) -&gt; SocketRequest&lt;E, VER&gt;</h4></section></summary><div class=\"docblock\"><p>Build a streaming connection request.</p>\n<h5 id=\"panics\"><a class=\"doc-anchor\" href=\"#panics\">§</a>Panics</h5>\n<p>This will panic if a malformed URL is passed.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.module\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">module</a>&lt;ModError&gt;(\n    &amp;self,\n    prefix: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Client&lt;ModError, VER&gt;, <a class=\"enum\" href=\"https://docs.rs/url/2.5.4/url/parser/enum.ParseError.html\" title=\"enum url::parser::ParseError\">ParseError</a>&gt;<div class=\"where\">where\n    ModError: Error,</div></h4></section></summary><div class=\"docblock\"><p>Create a client for a sub-module of the connected application.</p>\n</div></details></div></details>",0,"espresso_types::v0::impls::auction::SurfClient"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-Client%3CE,+VER%3E\" class=\"impl\"><a href=\"#impl-Clone-for-Client%3CE,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for Client&lt;E, VER&gt;<div class=\"where\">where\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; Client&lt;E, VER&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.1/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","espresso_types::v0::impls::auction::SurfClient"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-Client%3CE,+VER%3E\" class=\"impl\"><a href=\"#impl-Debug-for-Client%3CE,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for Client&lt;E, VER&gt;<div class=\"where\">where\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, __f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","espresso_types::v0::impls::auction::SurfClient"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CClientBuilder%3CE,+VER%3E%3E-for-Client%3CE,+VER%3E\" class=\"impl\"><a href=\"#impl-From%3CClientBuilder%3CE,+VER%3E%3E-for-Client%3CE,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;ClientBuilder&lt;E, VER&gt;&gt; for Client&lt;E, VER&gt;<div class=\"where\">where\n    E: Error,\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(builder: ClientBuilder&lt;E, VER&gt;) -&gt; Client&lt;E, VER&gt;</h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<ClientBuilder<E, VER>>","espresso_types::v0::impls::auction::SurfClient"]]],["verify_headers",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Client%3CE,+VER%3E\" class=\"impl\"><a href=\"#impl-Client%3CE,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, VER&gt; Client&lt;E, VER&gt;<div class=\"where\">where\n    E: Error,\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">new</a>(base_url: <a class=\"struct\" href=\"https://docs.rs/url/2.5.4/url/struct.Url.html\" title=\"struct url::Url\">Url</a>) -&gt; Client&lt;E, VER&gt;</h4></section></summary><div class=\"docblock\"><p>Create a client and connect to the Tide Disco server at <code>base_url</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.builder\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">builder</a>(base_url: <a class=\"struct\" href=\"https://docs.rs/url/2.5.4/url/struct.Url.html\" title=\"struct url::Url\">Url</a>) -&gt; ClientBuilder&lt;E, VER&gt;</h4></section></summary><div class=\"docblock\"><p>Create a client with customization.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.connect\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">connect</a>(&amp;self, timeout: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a>&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class=\"docblock\"><p>Connect to the server, retrying if the server is not running.</p>\n<p>It is not necessary to call this function when creating a new client. The client will\nautomatically connect when a request is made, if the server is available. However, this can\nbe useful to wait for the server to come up, if the server may be offline when the client is\ncreated.</p>\n<p>This function will make an HTTP <code>GET</code> request to the server’s <code>/healthcheck</code> endpoint, to\ntest if the server is available. If this request succeeds, <a href=\"Self::connect\">connect</a> returns\n<code>true</code>. Otherwise, the client will continue retrying <code>/healthcheck</code> requests until <code>timeout</code>\nhas elapsed (or forever, if <code>timeout</code> is <code>None</code>). If the timeout expires before a\n<code>/healthcheck</code> request succeeds, <a href=\"Self::connect\">connect</a> will return <code>false</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.wait_for_health\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">wait_for_health</a>&lt;H&gt;(\n    &amp;self,\n    healthy: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/ops/function/trait.Fn.html\" title=\"trait core::ops::function::Fn\">Fn</a>(<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.reference.html\">&amp;H</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.bool.html\">bool</a>,\n    timeout: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a>&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;H&gt;<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a> + HealthCheck,</div></h4></section></summary><div class=\"docblock\"><p>Connect to the server, retrying until the server is <code>healthy</code>.</p>\n<p>This function is similar to <a href=\"Self::connect\">connect</a>. It will make requests to the\n<code>/healthcheck</code> endpoint until a request succeeds. However, it will then continue retrying\nuntil the response from <code>/healthcheck</code> satisfies the <code>healthy</code> predicate.</p>\n<p>On success, returns the response from <code>/healthcheck</code>. On timeout, returns <code>None</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">get</a>&lt;T&gt;(&amp;self, route: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>) -&gt; Request&lt;T, E, VER&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,</div></h4></section></summary><div class=\"docblock\"><p>Build an HTTP <code>GET</code> request.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.post\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">post</a>&lt;T&gt;(&amp;self, route: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>) -&gt; Request&lt;T, E, VER&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,</div></h4></section></summary><div class=\"docblock\"><p>Build an HTTP <code>POST</code> request.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.healthcheck\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">healthcheck</a>&lt;H&gt;(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;H, E&gt;<div class=\"where\">where\n    H: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a> + HealthCheck,</div></h4></section></summary><div class=\"docblock\"><p>Query the server’s healthcheck endpoint.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.request\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">request</a>&lt;T&gt;(&amp;self, method: Method, route: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>) -&gt; Request&lt;T, E, VER&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.218/serde/de/trait.DeserializeOwned.html\" title=\"trait serde::de::DeserializeOwned\">DeserializeOwned</a>,</div></h4></section></summary><div class=\"docblock\"><p>Build an HTTP request with the specified method.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.socket\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">socket</a>(&amp;self, route: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>) -&gt; SocketRequest&lt;E, VER&gt;</h4></section></summary><div class=\"docblock\"><p>Build a streaming connection request.</p>\n<h5 id=\"panics\"><a class=\"doc-anchor\" href=\"#panics\">§</a>Panics</h5>\n<p>This will panic if a malformed URL is passed.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.module\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">module</a>&lt;ModError&gt;(\n    &amp;self,\n    prefix: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.str.html\">str</a>,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Client&lt;ModError, VER&gt;, <a class=\"enum\" href=\"https://docs.rs/url/2.5.4/url/parser/enum.ParseError.html\" title=\"enum url::parser::ParseError\">ParseError</a>&gt;<div class=\"where\">where\n    ModError: Error,</div></h4></section></summary><div class=\"docblock\"><p>Create a client for a sub-module of the connected application.</p>\n</div></details></div></details>",0,"verify_headers::SequencerClient"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-Client%3CE,+VER%3E\" class=\"impl\"><a href=\"#impl-Clone-for-Client%3CE,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for Client&lt;E, VER&gt;<div class=\"where\">where\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; Client&lt;E, VER&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.85.1/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.85.1/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","verify_headers::SequencerClient"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-Client%3CE,+VER%3E\" class=\"impl\"><a href=\"#impl-Debug-for-Client%3CE,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for Client&lt;E, VER&gt;<div class=\"where\">where\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, __f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.85.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.85.1/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","verify_headers::SequencerClient"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CClientBuilder%3CE,+VER%3E%3E-for-Client%3CE,+VER%3E\" class=\"impl\"><a href=\"#impl-From%3CClientBuilder%3CE,+VER%3E%3E-for-Client%3CE,+VER%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;E, VER&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;ClientBuilder&lt;E, VER&gt;&gt; for Client&lt;E, VER&gt;<div class=\"where\">where\n    E: Error,\n    VER: StaticVersionType,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.85.1/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(builder: ClientBuilder&lt;E, VER&gt;) -&gt; Client&lt;E, VER&gt;</h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<ClientBuilder<E, VER>>","verify_headers::SequencerClient"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[13428,13369]}