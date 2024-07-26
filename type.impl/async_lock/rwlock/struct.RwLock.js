(function() {var type_impls = {
"sequencer":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-RwLock%3CT%3E\" class=\"impl\"><a href=\"#impl-Debug-for-RwLock%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for RwLock&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.80.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.80.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.80.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","sequencer::state_signature::relay_server::State"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-RwLock%3CT%3E\" class=\"impl\"><a href=\"#impl-Default-for-RwLock%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for RwLock&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.80.0/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; RwLock&lt;T&gt;</h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/1.80.0/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","sequencer::state_signature::relay_server::State"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-From%3CT%3E-for-RwLock%3CT%3E\" class=\"impl\"><a href=\"#impl-From%3CT%3E-for-RwLock%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;T&gt; for RwLock&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.from\" class=\"method trait-impl\"><a href=\"#method.from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.80.0/core/convert/trait.From.html#tymethod.from\" class=\"fn\">from</a>(val: T) -&gt; RwLock&lt;T&gt;</h4></section></summary><div class='docblock'>Converts to this type from the input type.</div></details></div></details>","From<T>","sequencer::state_signature::relay_server::State"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-ReadState-for-RwLock%3CState%3E\" class=\"impl\"><a href=\"#impl-ReadState-for-RwLock%3CState%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;State&gt; ReadState for RwLock&lt;State&gt;<div class=\"where\">where\n    State: 'static + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.State\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.State\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a class=\"associatedtype\">State</a> = State</h4></section></summary><div class='docblock'>The type of state which this type allows a caller to read.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.read\" class=\"method trait-impl\"><a href=\"#method.read\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">read</a>&lt;'life0, 'async_trait, T&gt;(\n    &amp;'life0 self,\n    op: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + for&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/ops/function/trait.FnOnce.html\" title=\"trait core::ops::function::FnOnce\">FnOnce</a>(&amp;'a &lt;RwLock&lt;State&gt; as ReadState&gt;::State) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = T&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'a&gt;&gt; + 'async_trait,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = T&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    T: 'async_trait,\n    RwLock&lt;State&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Do an operation with immutable access to the state. <a>Read more</a></div></details></div></details>","ReadState","sequencer::state_signature::relay_server::State"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-RwLock%3CT%3E\" class=\"impl\"><a href=\"#impl-RwLock%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; RwLock&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><h4 class=\"code-header\">pub const fn <a class=\"fn\">new</a>(t: T) -&gt; RwLock&lt;T&gt;</h4></section></summary><div class=\"docblock\"><p>Creates a new reader-writer lock.</p>\n<h5 id=\"examples\"><a class=\"doc-anchor\" href=\"#examples\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = RwLock::new(<span class=\"number\">0</span>);</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.into_inner\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">into_inner</a>(self) -&gt; T</h4></section></summary><div class=\"docblock\"><p>Unwraps the lock and returns the inner value.</p>\n<h5 id=\"examples-1\"><a class=\"doc-anchor\" href=\"#examples-1\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = RwLock::new(<span class=\"number\">5</span>);\n<span class=\"macro\">assert_eq!</span>(lock.into_inner(), <span class=\"number\">5</span>);</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_read_arc\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_read_arc</a>(self: &amp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;T&gt;&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;RwLockReadGuardArc&lt;T&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Attempts to acquire an an owned, reference-counted read lock.</p>\n<p>If a read lock could not be acquired at this time, then <a href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html#variant.None\" title=\"variant core::option::Option::None\"><code>None</code></a> is returned. Otherwise, a\nguard is returned that releases the lock when dropped.</p>\n<h5 id=\"examples-2\"><a class=\"doc-anchor\" href=\"#examples-2\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>std::sync::Arc;\n<span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = Arc::new(RwLock::new(<span class=\"number\">1</span>));\n\n<span class=\"kw\">let </span>reader = lock.read_arc().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>reader, <span class=\"number\">1</span>);\n\n<span class=\"macro\">assert!</span>(lock.try_read_arc().is_some());</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.read_arc\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">read_arc</a>&lt;'a&gt;(self: &amp;'a <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;T&gt;&gt;) -&gt; ReadArc&lt;'a, T&gt;</h4></section></summary><div class=\"docblock\"><p>Acquires an owned, reference-counted read lock.</p>\n<p>Returns a guard that releases the lock when dropped.</p>\n<p>Note that attempts to acquire a read lock will block if there are also concurrent attempts\nto acquire a write lock.</p>\n<h5 id=\"examples-3\"><a class=\"doc-anchor\" href=\"#examples-3\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>std::sync::Arc;\n<span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = Arc::new(RwLock::new(<span class=\"number\">1</span>));\n\n<span class=\"kw\">let </span>reader = lock.read_arc().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>reader, <span class=\"number\">1</span>);\n\n<span class=\"macro\">assert!</span>(lock.try_read_arc().is_some());</code></pre></div>\n</div></details></div></details>",0,"sequencer::state_signature::relay_server::State"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-RwLock%3CT%3E\" class=\"impl\"><a href=\"#impl-RwLock%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; RwLock&lt;T&gt;<div class=\"where\">where\n    T: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_read\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_read</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;RwLockReadGuard&lt;'_, T&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Attempts to acquire a read lock.</p>\n<p>If a read lock could not be acquired at this time, then <a href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html#variant.None\" title=\"variant core::option::Option::None\"><code>None</code></a> is returned. Otherwise, a\nguard is returned that releases the lock when dropped.</p>\n<h5 id=\"examples\"><a class=\"doc-anchor\" href=\"#examples\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = RwLock::new(<span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span>reader = lock.read().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>reader, <span class=\"number\">1</span>);\n\n<span class=\"macro\">assert!</span>(lock.try_read().is_some());</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.read\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">read</a>(&amp;self) -&gt; Read&lt;'_, T&gt;</h4></section></summary><div class=\"docblock\"><p>Acquires a read lock.</p>\n<p>Returns a guard that releases the lock when dropped.</p>\n<p>Note that attempts to acquire a read lock will block if there are also concurrent attempts\nto acquire a write lock.</p>\n<h5 id=\"examples-1\"><a class=\"doc-anchor\" href=\"#examples-1\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = RwLock::new(<span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span>reader = lock.read().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>reader, <span class=\"number\">1</span>);\n\n<span class=\"macro\">assert!</span>(lock.try_read().is_some());</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_upgradable_read\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_upgradable_read</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;RwLockUpgradableReadGuard&lt;'_, T&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Attempts to acquire a read lock with the possiblity to upgrade to a write lock.</p>\n<p>If a read lock could not be acquired at this time, then <a href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html#variant.None\" title=\"variant core::option::Option::None\"><code>None</code></a> is returned. Otherwise, a\nguard is returned that releases the lock when dropped.</p>\n<p>Upgradable read lock reserves the right to be upgraded to a write lock, which means there\ncan be at most one upgradable read lock at a time.</p>\n<h5 id=\"examples-2\"><a class=\"doc-anchor\" href=\"#examples-2\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::{RwLock, RwLockUpgradableReadGuard};\n\n<span class=\"kw\">let </span>lock = RwLock::new(<span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span>reader = lock.upgradable_read().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>reader, <span class=\"number\">1</span>);\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>lock.try_read().unwrap(), <span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span><span class=\"kw-2\">mut </span>writer = RwLockUpgradableReadGuard::upgrade(reader).<span class=\"kw\">await</span>;\n<span class=\"kw-2\">*</span>writer = <span class=\"number\">2</span>;</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.upgradable_read\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">upgradable_read</a>(&amp;self) -&gt; UpgradableRead&lt;'_, T&gt;</h4></section></summary><div class=\"docblock\"><p>Acquires a read lock with the possiblity to upgrade to a write lock.</p>\n<p>Returns a guard that releases the lock when dropped.</p>\n<p>Upgradable read lock reserves the right to be upgraded to a write lock, which means there\ncan be at most one upgradable read lock at a time.</p>\n<p>Note that attempts to acquire an upgradable read lock will block if there are concurrent\nattempts to acquire another upgradable read lock or a write lock.</p>\n<h5 id=\"examples-3\"><a class=\"doc-anchor\" href=\"#examples-3\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::{RwLock, RwLockUpgradableReadGuard};\n\n<span class=\"kw\">let </span>lock = RwLock::new(<span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span>reader = lock.upgradable_read().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>reader, <span class=\"number\">1</span>);\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>lock.try_read().unwrap(), <span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span><span class=\"kw-2\">mut </span>writer = RwLockUpgradableReadGuard::upgrade(reader).<span class=\"kw\">await</span>;\n<span class=\"kw-2\">*</span>writer = <span class=\"number\">2</span>;</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_upgradable_read_arc\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_upgradable_read_arc</a>(\n    self: &amp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;T&gt;&gt;,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;RwLockUpgradableReadGuardArc&lt;T&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Attempts to acquire an owned, reference-counted read lock with the possiblity to\nupgrade to a write lock.</p>\n<p>If a read lock could not be acquired at this time, then <a href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html#variant.None\" title=\"variant core::option::Option::None\"><code>None</code></a> is returned. Otherwise, a\nguard is returned that releases the lock when dropped.</p>\n<p>Upgradable read lock reserves the right to be upgraded to a write lock, which means there\ncan be at most one upgradable read lock at a time.</p>\n<h5 id=\"examples-4\"><a class=\"doc-anchor\" href=\"#examples-4\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>std::sync::Arc;\n<span class=\"kw\">use </span>async_lock::{RwLock, RwLockUpgradableReadGuardArc};\n\n<span class=\"kw\">let </span>lock = Arc::new(RwLock::new(<span class=\"number\">1</span>));\n\n<span class=\"kw\">let </span>reader = lock.upgradable_read_arc().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>reader, <span class=\"number\">1</span>);\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>lock.try_read_arc().unwrap(), <span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span><span class=\"kw-2\">mut </span>writer = RwLockUpgradableReadGuardArc::upgrade(reader).<span class=\"kw\">await</span>;\n<span class=\"kw-2\">*</span>writer = <span class=\"number\">2</span>;</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.upgradable_read_arc\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">upgradable_read_arc</a>&lt;'a&gt;(\n    self: &amp;'a <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;T&gt;&gt;,\n) -&gt; UpgradableReadArc&lt;'a, T&gt;</h4></section></summary><div class=\"docblock\"><p>Acquires an owned, reference-counted read lock with the possiblity\nto upgrade to a write lock.</p>\n<p>Returns a guard that releases the lock when dropped.</p>\n<p>Upgradable read lock reserves the right to be upgraded to a write lock, which means there\ncan be at most one upgradable read lock at a time.</p>\n<p>Note that attempts to acquire an upgradable read lock will block if there are concurrent\nattempts to acquire another upgradable read lock or a write lock.</p>\n<h5 id=\"examples-5\"><a class=\"doc-anchor\" href=\"#examples-5\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>std::sync::Arc;\n<span class=\"kw\">use </span>async_lock::{RwLock, RwLockUpgradableReadGuardArc};\n\n<span class=\"kw\">let </span>lock = Arc::new(RwLock::new(<span class=\"number\">1</span>));\n\n<span class=\"kw\">let </span>reader = lock.upgradable_read_arc().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>reader, <span class=\"number\">1</span>);\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>lock.try_read_arc().unwrap(), <span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span><span class=\"kw-2\">mut </span>writer = RwLockUpgradableReadGuardArc::upgrade(reader).<span class=\"kw\">await</span>;\n<span class=\"kw-2\">*</span>writer = <span class=\"number\">2</span>;</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_write\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_write</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;RwLockWriteGuard&lt;'_, T&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Attempts to acquire a write lock.</p>\n<p>If a write lock could not be acquired at this time, then <a href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html#variant.None\" title=\"variant core::option::Option::None\"><code>None</code></a> is returned. Otherwise, a\nguard is returned that releases the lock when dropped.</p>\n<h5 id=\"examples-6\"><a class=\"doc-anchor\" href=\"#examples-6\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = RwLock::new(<span class=\"number\">1</span>);\n\n<span class=\"macro\">assert!</span>(lock.try_write().is_some());\n<span class=\"kw\">let </span>reader = lock.read().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert!</span>(lock.try_write().is_none());</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.write\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">write</a>(&amp;self) -&gt; Write&lt;'_, T&gt;</h4></section></summary><div class=\"docblock\"><p>Acquires a write lock.</p>\n<p>Returns a guard that releases the lock when dropped.</p>\n<h5 id=\"examples-7\"><a class=\"doc-anchor\" href=\"#examples-7\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = RwLock::new(<span class=\"number\">1</span>);\n\n<span class=\"kw\">let </span>writer = lock.write().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert!</span>(lock.try_read().is_none());</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_write_arc\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_write_arc</a>(self: &amp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;T&gt;&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;RwLockWriteGuardArc&lt;T&gt;&gt;</h4></section></summary><div class=\"docblock\"><p>Attempts to acquire an owned, reference-counted write lock.</p>\n<p>If a write lock could not be acquired at this time, then <a href=\"https://doc.rust-lang.org/1.80.0/core/option/enum.Option.html#variant.None\" title=\"variant core::option::Option::None\"><code>None</code></a> is returned. Otherwise, a\nguard is returned that releases the lock when dropped.</p>\n<h5 id=\"examples-8\"><a class=\"doc-anchor\" href=\"#examples-8\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>std::sync::Arc;\n<span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = Arc::new(RwLock::new(<span class=\"number\">1</span>));\n\n<span class=\"macro\">assert!</span>(lock.try_write_arc().is_some());\n<span class=\"kw\">let </span>reader = lock.read_arc().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert!</span>(lock.try_write_arc().is_none());</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.write_arc\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">write_arc</a>&lt;'a&gt;(self: &amp;'a <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/sync/struct.Arc.html\" title=\"struct alloc::sync::Arc\">Arc</a>&lt;RwLock&lt;T&gt;&gt;) -&gt; WriteArc&lt;'a, T&gt;</h4></section></summary><div class=\"docblock\"><p>Acquires an owned, reference-counted write lock.</p>\n<p>Returns a guard that releases the lock when dropped.</p>\n<h5 id=\"examples-9\"><a class=\"doc-anchor\" href=\"#examples-9\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>std::sync::Arc;\n<span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span>lock = Arc::new(RwLock::new(<span class=\"number\">1</span>));\n\n<span class=\"kw\">let </span>writer = lock.write_arc().<span class=\"kw\">await</span>;\n<span class=\"macro\">assert!</span>(lock.try_read_arc().is_none());</code></pre></div>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.get_mut\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">get_mut</a>(&amp;mut self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.80.0/std/primitive.reference.html\">&amp;mut T</a></h4></section></summary><div class=\"docblock\"><p>Returns a mutable reference to the inner value.</p>\n<p>Since this call borrows the lock mutably, no actual locking takes place. The mutable borrow\nstatically guarantees no locks exist.</p>\n<h5 id=\"examples-10\"><a class=\"doc-anchor\" href=\"#examples-10\">§</a>Examples</h5>\n<div class=\"example-wrap\"><pre class=\"rust rust-example-rendered\"><code><span class=\"kw\">use </span>async_lock::RwLock;\n\n<span class=\"kw\">let </span><span class=\"kw-2\">mut </span>lock = RwLock::new(<span class=\"number\">1</span>);\n\n<span class=\"kw-2\">*</span>lock.get_mut() = <span class=\"number\">2</span>;\n<span class=\"macro\">assert_eq!</span>(<span class=\"kw-2\">*</span>lock.read().<span class=\"kw\">await</span>, <span class=\"number\">2</span>);</code></pre></div>\n</div></details></div></details>",0,"sequencer::state_signature::relay_server::State"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-WriteState-for-RwLock%3CState%3E\" class=\"impl\"><a href=\"#impl-WriteState-for-RwLock%3CState%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;State&gt; WriteState for RwLock&lt;State&gt;<div class=\"where\">where\n    State: 'static + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.write\" class=\"method trait-impl\"><a href=\"#method.write\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">write</a>&lt;'life0, 'async_trait, T&gt;(\n    &amp;'life0 self,\n    op: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + for&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/ops/function/trait.FnOnce.html\" title=\"trait core::ops::function::FnOnce\">FnOnce</a>(&amp;'a mut &lt;RwLock&lt;State&gt; as ReadState&gt;::State) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = T&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'a&gt;&gt; + 'async_trait,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.80.0/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = T&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    'life0: 'async_trait,\n    T: 'async_trait,\n    RwLock&lt;State&gt;: 'async_trait,</div></h4></section></summary><div class='docblock'>Do an operation with mutable access to the state. <a>Read more</a></div></details></div></details>","WriteState","sequencer::state_signature::relay_server::State"],["<section id=\"impl-Send-for-RwLock%3CT%3E\" class=\"impl\"><a href=\"#impl-Send-for-RwLock%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> for RwLock&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section>","Send","sequencer::state_signature::relay_server::State"],["<section id=\"impl-Sync-for-RwLock%3CT%3E\" class=\"impl\"><a href=\"#impl-Sync-for-RwLock%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> for RwLock&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.80.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div></h3></section>","Sync","sequencer::state_signature::relay_server::State"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()