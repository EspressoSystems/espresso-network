(function() {
    var type_impls = Object.fromEntries([["hotshot_query_service",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-DataSourceLifeCycle-for-FetchingDataSource%3CMockTypes,+FileSystemStorage%3CMockTypes%3E,+P%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#254-278\">Source</a><a href=\"#impl-DataSourceLifeCycle-for-FetchingDataSource%3CMockTypes,+FileSystemStorage%3CMockTypes%3E,+P%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;P: <a class=\"trait\" href=\"hotshot_query_service/data_source/fetching/trait.AvailabilityProvider.html\" title=\"trait hotshot_query_service::data_source::fetching::AvailabilityProvider\">AvailabilityProvider</a>&lt;<a class=\"struct\" href=\"hotshot_query_service/testing/mocks/struct.MockTypes.html\" title=\"struct hotshot_query_service::testing::mocks::MockTypes\">MockTypes</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html\" title=\"trait hotshot_query_service::testing::consensus::DataSourceLifeCycle\">DataSourceLifeCycle</a> for <a class=\"type\" href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a>&lt;<a class=\"struct\" href=\"hotshot_query_service/testing/mocks/struct.MockTypes.html\" title=\"struct hotshot_query_service::testing::mocks::MockTypes\">MockTypes</a>, P&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Storage\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#257\">Source</a><a href=\"#associatedtype.Storage\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#associatedtype.Storage\" class=\"associatedtype\">Storage</a> = <a class=\"struct\" href=\"https://docs.rs/tempfile/latest/tempfile/dir/struct.TempDir.html\" title=\"struct tempfile::dir::TempDir\">TempDir</a></h4></section></summary><div class='docblock'>Backing storage for the data source. <a href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#associatedtype.Storage\">Read more</a></div></details><section id=\"method.create\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#259-261\">Source</a><a href=\"#method.create\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#tymethod.create\" class=\"fn\">create</a>&lt;'async_trait&gt;(\n    node_id: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.usize.html\">usize</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = Self::<a class=\"associatedtype\" href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#associatedtype.Storage\" title=\"type hotshot_query_service::testing::consensus::DataSourceLifeCycle::Storage\">Storage</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    Self: 'async_trait,</div></h4></section><section id=\"method.connect\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#263-267\">Source</a><a href=\"#method.connect\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#tymethod.connect\" class=\"fn\">connect</a>&lt;'life0, 'async_trait&gt;(\n    storage: &amp;'life0 Self::<a class=\"associatedtype\" href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#associatedtype.Storage\" title=\"type hotshot_query_service::testing::consensus::DataSourceLifeCycle::Storage\">Storage</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = Self&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    Self: 'async_trait,\n    'life0: 'async_trait,</div></h4></section><section id=\"method.reset\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#269-273\">Source</a><a href=\"#method.reset\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#tymethod.reset\" class=\"fn\">reset</a>&lt;'life0, 'async_trait&gt;(\n    storage: &amp;'life0 Self::<a class=\"associatedtype\" href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#associatedtype.Storage\" title=\"type hotshot_query_service::testing::consensus::DataSourceLifeCycle::Storage\">Storage</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = Self&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    Self: 'async_trait,\n    'life0: 'async_trait,</div></h4></section><section id=\"method.handle_event\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#275-277\">Source</a><a href=\"#method.handle_event\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#tymethod.handle_event\" class=\"fn\">handle_event</a>&lt;'life0, 'life1, 'async_trait&gt;(\n    &amp;'life0 self,\n    event: &amp;'life1 <a class=\"struct\" href=\"hotshot_types/event/struct.Event.html\" title=\"struct hotshot_types::event::Event\">Event</a>&lt;<a class=\"struct\" href=\"hotshot_query_service/testing/mocks/struct.MockTypes.html\" title=\"struct hotshot_query_service::testing::mocks::MockTypes\">MockTypes</a>&gt;,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    Self: 'async_trait,\n    'life0: 'async_trait,\n    'life1: 'async_trait,</div></h4></section><section id=\"method.leaf_only_ds\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/consensus.rs.html#344-346\">Source</a><a href=\"#method.leaf_only_ds\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#method.leaf_only_ds\" class=\"fn\">leaf_only_ds</a>&lt;'life0, 'async_trait&gt;(\n    _storage: &amp;'life0 Self::<a class=\"associatedtype\" href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#associatedtype.Storage\" title=\"type hotshot_query_service::testing::consensus::DataSourceLifeCycle::Storage\">Storage</a>,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = Self&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    Self: 'async_trait,\n    'life0: 'async_trait,</div></h4></section><details class=\"toggle method-toggle\" open><summary><section id=\"method.setup\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/testing/consensus.rs.html#349\">Source</a><a href=\"#method.setup\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"hotshot_query_service/testing/consensus/trait.DataSourceLifeCycle.html#method.setup\" class=\"fn\">setup</a>&lt;'life0, 'async_trait, V&gt;(\n    _network: &amp;'life0 mut <a class=\"struct\" href=\"hotshot_query_service/testing/consensus/struct.MockNetwork.html\" title=\"struct hotshot_query_service::testing::consensus::MockNetwork\">MockNetwork</a>&lt;Self, V&gt;,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.85.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'async_trait&gt;&gt;<div class=\"where\">where\n    V: 'async_trait + <a class=\"trait\" href=\"hotshot_types/traits/node_implementation/trait.Versions.html\" title=\"trait hotshot_types::traits::node_implementation::Versions\">Versions</a>,\n    Self: 'async_trait,\n    'life0: 'async_trait,</div></h4></section></summary><div class='docblock'>Setup runs after setting up the network but before starting a test.</div></details></div></details>","DataSourceLifeCycle","hotshot_query_service::testing::consensus::MockDataSource"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-FetchingDataSource%3CTypes,+FileSystemStorage%3CTypes%3E,+P%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#161-239\">Source</a><a href=\"#impl-FetchingDataSource%3CTypes,+FileSystemStorage%3CTypes%3E,+P%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;Types: <a class=\"trait\" href=\"hotshot_types/traits/node_implementation/trait.NodeType.html\" title=\"trait hotshot_types::traits::node_implementation::NodeType\">NodeType</a>, P&gt; <a class=\"type\" href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a>&lt;Types, P&gt;<div class=\"where\">where\n    <a class=\"type\" href=\"hotshot_query_service/type.Payload.html\" title=\"type hotshot_query_service::Payload\">Payload</a>&lt;Types&gt;: <a class=\"trait\" href=\"hotshot_query_service/availability/trait.QueryablePayload.html\" title=\"trait hotshot_query_service::availability::QueryablePayload\">QueryablePayload</a>&lt;Types&gt;,\n    <a class=\"type\" href=\"hotshot_query_service/type.Header.html\" title=\"type hotshot_query_service::Header\">Header</a>&lt;Types&gt;: <a class=\"trait\" href=\"hotshot_query_service/availability/trait.QueryableHeader.html\" title=\"trait hotshot_query_service::availability::QueryableHeader\">QueryableHeader</a>&lt;Types&gt;,\n    P: <a class=\"trait\" href=\"hotshot_query_service/data_source/fetching/trait.AvailabilityProvider.html\" title=\"trait hotshot_query_service::data_source::fetching::AvailabilityProvider\">AvailabilityProvider</a>&lt;Types&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.create\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#172-176\">Source</a><h4 class=\"code-header\">pub async fn <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html#tymethod.create\" class=\"fn\">create</a>(path: &amp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/std/path/struct.Path.html\" title=\"struct std::path::Path\">Path</a>, provider: P) -&gt; <a class=\"type\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/type.Result.html\" title=\"type anyhow::Result\">Result</a>&lt;Self&gt;</h4></section></summary><div class=\"docblock\"><p>Create a new <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> with storage at <code>path</code>.</p>\n<p>If there is already data at <code>path</code>, it will be archived.</p>\n<p>The <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> will manage its own persistence synchronization.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.open\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#183-187\">Source</a><h4 class=\"code-header\">pub async fn <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html#tymethod.open\" class=\"fn\">open</a>(path: &amp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.85.1/std/path/struct.Path.html\" title=\"struct std::path::Path\">Path</a>, provider: P) -&gt; <a class=\"type\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/type.Result.html\" title=\"type anyhow::Result\">Result</a>&lt;Self&gt;</h4></section></summary><div class=\"docblock\"><p>Open an existing <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> from storage at <code>path</code>.</p>\n<p>If there is no data at <code>path</code>, a new store will be created.</p>\n<p>The <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> will manage its own persistence synchronization.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.create_with_store\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#197-207\">Source</a><h4 class=\"code-header\">pub async fn <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html#tymethod.create_with_store\" class=\"fn\">create_with_store</a>(\n    loader: &amp;mut AtomicStoreLoader,\n    provider: P,\n) -&gt; <a class=\"type\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/type.Result.html\" title=\"type anyhow::Result\">Result</a>&lt;Self&gt;</h4></section></summary><div class=\"docblock\"><p>Create a new <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> using a persistent storage loader.</p>\n<p>If there is existing data corresponding to the <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> data structures, it\nwill be archived.</p>\n<p>The <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> will register its persistent data structures with <code>loader</code>. The\ncaller is responsible for creating an <a href=\"atomic_store::AtomicStore\">AtomicStore</a> from <code>loader</code>\nand managing synchronization of the store.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.open_with_store\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#217-224\">Source</a><h4 class=\"code-header\">pub async fn <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html#tymethod.open_with_store\" class=\"fn\">open_with_store</a>(\n    loader: &amp;mut AtomicStoreLoader,\n    provider: P,\n) -&gt; <a class=\"type\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/type.Result.html\" title=\"type anyhow::Result\">Result</a>&lt;Self&gt;</h4></section></summary><div class=\"docblock\"><p>Open an existing <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> using a persistent storage loader.</p>\n<p>If there is no existing data corresponding to the <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> data structures, a\nnew store will be created.</p>\n<p>The <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> will register its persistent data structures with <code>loader</code>. The\ncaller is responsible for creating an <a href=\"atomic_store::AtomicStore\">AtomicStore</a> from <code>loader</code>\nand managing synchronization of the store.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.skip_version\" class=\"method\"><a class=\"src rightside\" href=\"src/hotshot_query_service/data_source/fs.rs.html#235-238\">Source</a><h4 class=\"code-header\">pub async fn <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html#tymethod.skip_version\" class=\"fn\">skip_version</a>(&amp;self) -&gt; <a class=\"type\" href=\"https://docs.rs/anyhow/1.0.97/anyhow/type.Result.html\" title=\"type anyhow::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.85.1/std/primitive.unit.html\">()</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Advance the version of the persistent store without committing changes to persistent state.</p>\n<p>This function is useful when the <a href=\"atomic_store::AtomicStore\">AtomicStore</a> synchronizing\nstorage for this <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> is being managed by the caller. The caller may want\nto persist some changes to other modules whose state is managed by the same\n<a href=\"atomic_store::AtomicStore\">AtomicStore</a>. In order to call\n<a href=\"atomic_store::AtomicStore::commit_version\">AtomicStore::commit_version</a>, the version of\nthis <a href=\"hotshot_query_service/data_source/fs/type.FileSystemDataSource.html\" title=\"type hotshot_query_service::data_source::fs::FileSystemDataSource\">FileSystemDataSource</a> must be advanced, either by <a href=\"hotshot_query_service/data_source/trait.Transaction.html#tymethod.commit\" title=\"method hotshot_query_service::data_source::Transaction::commit\">commit</a>\nor, if there are no outstanding changes, <a href=\"hotshot_query_service/data_source/fetching/struct.FetchingDataSource.html#method.skip_version\" title=\"method hotshot_query_service::data_source::fetching::FetchingDataSource::skip_version\">skip_version</a>.</p>\n</div></details></div></details>",0,"hotshot_query_service::testing::consensus::MockDataSource"]]],["sequencer",[]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[21174,17]}