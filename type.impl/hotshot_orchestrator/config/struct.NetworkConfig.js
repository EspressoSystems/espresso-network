(function() {var type_impls = {
"espresso_types":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-NetworkConfig%3CKEY%3E\" class=\"impl\"><a href=\"#impl-Clone-for-NetworkConfig%3CKEY%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;KEY&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for NetworkConfig&lt;KEY&gt;<div class=\"where\">where\n    KEY: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + SignatureKey,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.81.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; NetworkConfig&lt;KEY&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.81.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.81.0/src/core/clone.rs.html#172\">source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.81.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.reference.html\">&amp;Self</a>)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.81.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","espresso_types::v0::NetworkConfig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-NetworkConfig%3CKEY%3E\" class=\"impl\"><a href=\"#impl-Debug-for-NetworkConfig%3CKEY%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;KEY&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for NetworkConfig&lt;KEY&gt;<div class=\"where\">where\n    KEY: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + SignatureKey,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.81.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.81.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.81.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.81.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","espresso_types::v0::NetworkConfig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-NetworkConfig%3CK%3E\" class=\"impl\"><a href=\"#impl-Default-for-NetworkConfig%3CK%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;K&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for NetworkConfig&lt;K&gt;<div class=\"where\">where\n    K: SignatureKey,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.81.0/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; NetworkConfig&lt;K&gt;</h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/1.81.0/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","espresso_types::v0::NetworkConfig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Deserialize%3C'de%3E-for-NetworkConfig%3CKEY%3E\" class=\"impl\"><a href=\"#impl-Deserialize%3C'de%3E-for-NetworkConfig%3CKEY%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'de, KEY&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for NetworkConfig&lt;KEY&gt;<div class=\"where\">where\n    KEY: SignatureKey,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize\" class=\"method trait-impl\"><a href=\"#method.deserialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserialize.html#tymethod.deserialize\" class=\"fn\">deserialize</a>&lt;__D&gt;(\n    __deserializer: __D,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;NetworkConfig&lt;KEY&gt;, &lt;__D as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserializer.html#associatedtype.Error\" title=\"type serde::de::Deserializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __D: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;,</div></h4></section></summary><div class='docblock'>Deserialize this value from the given Serde deserializer. <a href=\"https://docs.rs/serde/1.0.210/serde/de/trait.Deserialize.html#tymethod.deserialize\">Read more</a></div></details></div></details>","Deserialize<'de>","espresso_types::v0::NetworkConfig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CNetworkConfigFile%3CK%3E%3E-for-NetworkConfig%3CK%3E\" class=\"impl\"><a href=\"#impl-From%3CNetworkConfigFile%3CK%3E%3E-for-NetworkConfig%3CK%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;K&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;NetworkConfigFile&lt;K&gt;&gt; for NetworkConfig&lt;K&gt;<div class=\"where\">where\n    K: SignatureKey,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.81.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(val: NetworkConfigFile&lt;K&gt;) -&gt; NetworkConfig&lt;K&gt;</h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<NetworkConfigFile<K>>","espresso_types::v0::NetworkConfig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-NetworkConfig%3CK%3E\" class=\"impl\"><a href=\"#impl-NetworkConfig%3CK%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;K&gt; NetworkConfig&lt;K&gt;<div class=\"where\">where\n    K: SignatureKey,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from_file_or_orchestrator\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">from_file_or_orchestrator</a>(\n    client: &amp;OrchestratorClient,\n    file: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.81.0/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>&gt;,\n    libp2p_advertise_address: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;Multiaddr&gt;,\n    libp2p_public_key: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;PeerId&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;(NetworkConfig&lt;K&gt;, NetworkConfigSource), <a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.87/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Asynchronously retrieves a <code>NetworkConfig</code> either from a file or from an orchestrator.</p>\n<p>This function takes an <code>OrchestratorClient</code>, an optional file path, and Libp2p-specific parameters.</p>\n<p>If a file path is provided, the function will first attempt to load the <code>NetworkConfig</code> from the file.\nIf the file does not exist or cannot be read, the function will fall back to retrieving the <code>NetworkConfig</code> from the orchestrator.\nIn this case, if the path to the file does not exist, it will be created.\nThe retrieved <code>NetworkConfig</code> is then saved back to the file for future use.</p>\n<p>If no file path is provided, the function will directly retrieve the <code>NetworkConfig</code> from the orchestrator.</p>\n<h5 id=\"errors\"><a class=\"doc-anchor\" href=\"#errors\">§</a>Errors</h5>\n<p>If we were unable to load the configuration.</p>\n<h5 id=\"arguments\"><a class=\"doc-anchor\" href=\"#arguments\">§</a>Arguments</h5>\n<ul>\n<li><code>client</code> - An <code>OrchestratorClient</code> used to retrieve the <code>NetworkConfig</code> from the orchestrator.</li>\n<li><code>identity</code> - A string representing the identity for which to retrieve the <code>NetworkConfig</code>.</li>\n<li><code>file</code> - An optional string representing the path to the file from which to load the <code>NetworkConfig</code>.</li>\n<li><code>libp2p_address</code> - An optional address specifying where other Libp2p nodes can reach us</li>\n<li><code>libp2p_public_key</code> - The public key in which other Libp2p nodes can reach us with</li>\n</ul>\n<h5 id=\"returns\"><a class=\"doc-anchor\" href=\"#returns\">§</a>Returns</h5>\n<p>This function returns a tuple containing a <code>NetworkConfig</code> and a <code>NetworkConfigSource</code>. The <code>NetworkConfigSource</code> indicates whether the <code>NetworkConfig</code> was loaded from a file or retrieved from the orchestrator.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.generate_init_validator_config\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">generate_init_validator_config</a>(\n    client: &amp;OrchestratorClient,\n    is_da: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.bool.html\">bool</a>,\n) -&gt; ValidatorConfig&lt;K&gt;</h4></section></summary><div class=\"docblock\"><p>Get a temporary node index for generating a validator config</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_complete_config\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">get_complete_config</a>(\n    client: &amp;OrchestratorClient,\n    my_own_validator_config: ValidatorConfig&lt;K&gt;,\n    libp2p_advertise_address: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;Multiaddr&gt;,\n    libp2p_public_key: <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;PeerId&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;(NetworkConfig&lt;K&gt;, NetworkConfigSource), <a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.87/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Asynchronously retrieves a <code>NetworkConfig</code> from an orchestrator.\nThe retrieved one includes correct <code>node_index</code> and peer’s public config.</p>\n<h5 id=\"errors-1\"><a class=\"doc-anchor\" href=\"#errors-1\">§</a>Errors</h5>\n<p>If we are unable to get the configuration from the orchestrator</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.from_file\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">from_file</a>(file: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.81.0/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;NetworkConfig&lt;K&gt;, NetworkConfigError&gt;</h4></section></summary><div class=\"docblock\"><p>Loads a <code>NetworkConfig</code> from a file.</p>\n<p>This function takes a file path as a string, reads the file, and then deserializes the contents into a <code>NetworkConfig</code>.</p>\n<h5 id=\"arguments-1\"><a class=\"doc-anchor\" href=\"#arguments-1\">§</a>Arguments</h5>\n<ul>\n<li><code>file</code> - A string representing the path to the file from which to load the <code>NetworkConfig</code>.</li>\n</ul>\n<h5 id=\"returns-1\"><a class=\"doc-anchor\" href=\"#returns-1\">§</a>Returns</h5>\n<p>This function returns a <code>Result</code> that contains a <code>NetworkConfig</code> if the file was successfully read and deserialized, or a <code>NetworkConfigError</code> if an error occurred.</p>\n<h5 id=\"errors-2\"><a class=\"doc-anchor\" href=\"#errors-2\">§</a>Errors</h5>\n<p>This function will return an error if the file cannot be read or if the contents cannot be deserialized into a <code>NetworkConfig</code>.</p>\n<h5 id=\"examples\"><a class=\"doc-anchor\" href=\"#examples\">§</a>Examples</h5>\n<div class=\"example-wrap ignore\"><a href=\"#\" class=\"tooltip\" title=\"This example is not tested\">ⓘ</a><pre class=\"rust rust-example-rendered\"><code><span class=\"comment\">// # use hotshot::traits::election::static_committee::StaticElectionConfig;\n</span><span class=\"kw\">let </span>file = <span class=\"string\">\"/path/to/my/config\"</span>.to_string();\n<span class=\"comment\">// NOTE: broken due to staticelectionconfig not being importable\n// cannot import staticelectionconfig from hotshot without creating circular dependency\n// making this work probably involves the `types` crate implementing a dummy\n// electionconfigtype just ot make this example work\n</span><span class=\"kw\">let </span>config = NetworkConfig::&lt;BLSPubKey, StaticElectionConfig&gt;::from_file(file).unwrap();</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.to_file\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">to_file</a>(&amp;self, file: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.81.0/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.unit.html\">()</a>, NetworkConfigError&gt;</h4></section></summary><div class=\"docblock\"><p>Serializes the <code>NetworkConfig</code> and writes it to a file.</p>\n<p>This function takes a file path as a string, serializes the <code>NetworkConfig</code> into JSON format using <code>serde_json</code> and then writes the serialized data to the file.</p>\n<h5 id=\"arguments-2\"><a class=\"doc-anchor\" href=\"#arguments-2\">§</a>Arguments</h5>\n<ul>\n<li><code>file</code> - A string representing the path to the file where the <code>NetworkConfig</code> should be saved.</li>\n</ul>\n<h5 id=\"returns-2\"><a class=\"doc-anchor\" href=\"#returns-2\">§</a>Returns</h5>\n<p>This function returns a <code>Result</code> that contains <code>()</code> if the <code>NetworkConfig</code> was successfully serialized and written to the file, or a <code>NetworkConfigError</code> if an error occurred.</p>\n<h5 id=\"errors-3\"><a class=\"doc-anchor\" href=\"#errors-3\">§</a>Errors</h5>\n<p>This function will return an error if the <code>NetworkConfig</code> cannot be serialized or if the file cannot be written.</p>\n<h5 id=\"examples-1\"><a class=\"doc-anchor\" href=\"#examples-1\">§</a>Examples</h5>\n<div class=\"example-wrap ignore\"><a href=\"#\" class=\"tooltip\" title=\"This example is not tested\">ⓘ</a><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">let </span>file = <span class=\"string\">\"/path/to/my/config\"</span>.to_string();\n<span class=\"kw\">let </span>config = NetworkConfig::from_file(file);\nconfig.to_file(file).unwrap();</code></pre></div>\n</div></details></div></details>",0,"espresso_types::v0::NetworkConfig"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-NetworkConfig%3CKEY%3E\" class=\"impl\"><a href=\"#impl-Serialize-for-NetworkConfig%3CKEY%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;KEY&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for NetworkConfig&lt;KEY&gt;<div class=\"where\">where\n    KEY: SignatureKey + <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;__S&gt;(\n    &amp;self,\n    __serializer: __S,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.81.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, &lt;__S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __S: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</div></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"https://docs.rs/serde/1.0.210/serde/ser/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","espresso_types::v0::NetworkConfig"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()