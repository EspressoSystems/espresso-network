(function() {
    var type_impls = Object.fromEntries([["hotshot_query_service",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Execute%3C'q,+DB%3E-for-Query%3C'q,+DB,+A%3E\" class=\"impl\"><a href=\"#impl-Execute%3C'q,+DB%3E-for-Query%3C'q,+DB,+A%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'q, DB, A&gt; Execute&lt;'q, DB&gt; for Query&lt;'q, DB, A&gt;<div class=\"where\">where\n    DB: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>,\n    A: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + IntoArguments&lt;'q, DB&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.sql\" class=\"method trait-impl\"><a href=\"#method.sql\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">sql</a>(&amp;self) -&gt; &amp;'q <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.str.html\">str</a></h4></section></summary><div class='docblock'>Gets the SQL that will be executed.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.statement\" class=\"method trait-impl\"><a href=\"#method.statement\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">statement</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&amp;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Statement\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Statement\">Statement</a>&lt;'q&gt;&gt;</h4></section></summary><div class='docblock'>Gets the previously cached statement, if available.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.take_arguments\" class=\"method trait-impl\"><a href=\"#method.take_arguments\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">take_arguments</a>(\n    &amp;mut self,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Arguments\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Arguments\">Arguments</a>&lt;'q&gt;&gt;, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/error/trait.Error.html\" title=\"trait core::error::Error\">Error</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a>&gt;&gt;</h4></section></summary><div class='docblock'>Returns the arguments to be bound against the query string. <a>Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.persistent\" class=\"method trait-impl\"><a href=\"#method.persistent\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a class=\"fn\">persistent</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Returns <code>true</code> if the statement should be cached.</div></details></div></details>","Execute<'q, DB>","hotshot_query_service::data_source::storage::sql::transaction::Query"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Query%3C'q,+DB,+%3CDB+as+Database%3E::Arguments%3C'q%3E%3E\" class=\"impl\"><a href=\"#impl-Query%3C'q,+DB,+%3CDB+as+Database%3E::Arguments%3C'q%3E%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'q, DB&gt; Query&lt;'q, DB, &lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Arguments\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Arguments\">Arguments</a>&lt;'q&gt;&gt;<div class=\"where\">where\n    DB: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.bind\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">bind</a>&lt;T&gt;(self, value: T) -&gt; Query&lt;'q, DB, &lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Arguments\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Arguments\">Arguments</a>&lt;'q&gt;&gt;<div class=\"where\">where\n    T: 'q + Encode&lt;'q, DB&gt; + Type&lt;DB&gt;,</div></h4></section></summary><div class=\"docblock\"><p>Bind a value for use with this SQL query.</p>\n<p>If the number of times this is called does not match the number of bind parameters that\nappear in the query (<code>?</code> for most SQL flavors, <code>$1 .. $N</code> for Postgres) then an error\nwill be returned when this query is executed.</p>\n<p>There is no validation that the value is of the type expected by the query. Most SQL\nflavors will perform type coercion (Postgres will return a database error).</p>\n<p>If encoding the value fails, the error is stored and later surfaced when executing the query.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_bind\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_bind</a>&lt;T&gt;(\n    &amp;mut self,\n    value: T,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/error/trait.Error.html\" title=\"trait core::error::Error\">Error</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a>&gt;&gt;<div class=\"where\">where\n    T: 'q + Encode&lt;'q, DB&gt; + Type&lt;DB&gt;,</div></h4></section></summary><div class=\"docblock\"><p>Like [<code>Query::try_bind</code>] but immediately returns an error if encoding the value failed.</p>\n</div></details></div></details>",0,"hotshot_query_service::data_source::storage::sql::transaction::Query"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Query%3C'q,+DB,+A%3E\" class=\"impl\"><a href=\"#impl-Query%3C'q,+DB,+A%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'q, DB, A&gt; Query&lt;'q, DB, A&gt;<div class=\"where\">where\n    A: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'q + IntoArguments&lt;'q, DB&gt;,\n    DB: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.map\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">map</a>&lt;F, O&gt;(\n    self,\n    f: F,\n) -&gt; Map&lt;'q, DB, impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/function/trait.FnMut.html\" title=\"trait core::ops::function::FnMut\">FnMut</a>(&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Row\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Row\">Row</a>) + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>, A&gt;<div class=\"where\">where\n    F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/function/trait.FnMut.html\" title=\"trait core::ops::function::FnMut\">FnMut</a>(&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Row\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Row\">Row</a>) -&gt; O + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,\n    O: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</div></h4></section></summary><div class=\"docblock\"><p>Map each row in the result to another type.</p>\n<p>See <a href=\"Query::try_map\"><code>try_map</code></a> for a fallible version of this method.</p>\n<p>The <a href=\"super::query_as::query_as\"><code>query_as</code></a> method will construct a mapped query using\na <a href=\"super::from_row::FromRow\"><code>FromRow</code></a> implementation.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.try_map\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">try_map</a>&lt;F, O&gt;(self, f: F) -&gt; Map&lt;'q, DB, F, A&gt;<div class=\"where\">where\n    F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/function/trait.FnMut.html\" title=\"trait core::ops::function::FnMut\">FnMut</a>(&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Row\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Row\">Row</a>) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;O, Error&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,\n    O: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>,</div></h4></section></summary><div class=\"docblock\"><p>Map each row in the result to another type.</p>\n<p>The <a href=\"super::query_as::query_as\"><code>query_as</code></a> method will construct a mapped query using\na <a href=\"super::from_row::FromRow\"><code>FromRow</code></a> implementation.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.execute\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">execute</a>&lt;'e, 'c, E&gt;(\n    self,\n    executor: E,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.QueryResult\" title=\"type hotshot_query_service::data_source::storage::sql::Database::QueryResult\">QueryResult</a>, Error&gt;<div class=\"where\">where\n    'c: 'e,\n    'q: 'e,\n    A: 'e,\n    E: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Executor.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Executor\">Executor</a>&lt;'c, Database = DB&gt;,</div></h4></section></summary><div class=\"docblock\"><p>Execute the query and return the total number of rows affected.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.execute_many\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">execute_many</a>&lt;'e, 'c, E&gt;(\n    self,\n    executor: E,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn Stream&lt;Item = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.QueryResult\" title=\"type hotshot_query_service::data_source::storage::sql::Database::QueryResult\">QueryResult</a>, Error&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'e&gt;&gt;<div class=\"where\">where\n    'c: 'e,\n    'q: 'e,\n    A: 'e,\n    E: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Executor.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Executor\">Executor</a>&lt;'c, Database = DB&gt;,</div></h4></section><span class=\"item-info\"><div class=\"stab deprecated\"><span class=\"emoji\">👎</span><span>Deprecated: Only the SQLite driver supports multiple statements in one prepared statement and that behavior is deprecated. Use <code>sqlx::raw_sql()</code> instead. See https://github.com/launchbadge/sqlx/issues/3108 for discussion.</span></div></span></summary><div class=\"docblock\"><p>Execute multiple queries and return the rows affected from each query, in a stream.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">fetch</a>&lt;'e, 'c, E&gt;(\n    self,\n    executor: E,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn Stream&lt;Item = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Row\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Row\">Row</a>, Error&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'e&gt;&gt;<div class=\"where\">where\n    'c: 'e,\n    'q: 'e,\n    A: 'e,\n    E: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Executor.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Executor\">Executor</a>&lt;'c, Database = DB&gt;,</div></h4></section></summary><div class=\"docblock\"><p>Execute the query and return the generated results as a stream.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch_many\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">fetch_many</a>&lt;'e, 'c, E&gt;(\n    self,\n    executor: E,\n) -&gt; <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/core/pin/struct.Pin.html\" title=\"struct core::pin::Pin\">Pin</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/alloc/boxed/struct.Box.html\" title=\"struct alloc::boxed::Box\">Box</a>&lt;dyn Stream&lt;Item = <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"enum\" href=\"https://docs.rs/either/1/either/enum.Either.html\" title=\"enum either::Either\">Either</a>&lt;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.QueryResult\" title=\"type hotshot_query_service::data_source::storage::sql::Database::QueryResult\">QueryResult</a>, &lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Row\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Row\">Row</a>&gt;, Error&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'e&gt;&gt;<div class=\"where\">where\n    'c: 'e,\n    'q: 'e,\n    A: 'e,\n    E: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Executor.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Executor\">Executor</a>&lt;'c, Database = DB&gt;,</div></h4></section><span class=\"item-info\"><div class=\"stab deprecated\"><span class=\"emoji\">👎</span><span>Deprecated: Only the SQLite driver supports multiple statements in one prepared statement and that behavior is deprecated. Use <code>sqlx::raw_sql()</code> instead. See https://github.com/launchbadge/sqlx/issues/3108 for discussion.</span></div></span></summary><div class=\"docblock\"><p>Execute multiple queries and return the generated results as a stream.</p>\n<p>For each query in the stream, any generated rows are returned first,\nthen the <code>QueryResult</code> with the number of rows affected.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch_all\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">fetch_all</a>&lt;'e, 'c, E&gt;(\n    self,\n    executor: E,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Row\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Row\">Row</a>&gt;, Error&gt;<div class=\"where\">where\n    'c: 'e,\n    'q: 'e,\n    A: 'e,\n    E: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Executor.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Executor\">Executor</a>&lt;'c, Database = DB&gt;,</div></h4></section></summary><div class=\"docblock\"><p>Execute the query and return all the resulting rows collected into a <a href=\"https://doc.rust-lang.org/1.84.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\"><code>Vec</code></a>.</p>\n<h6 id=\"note-beware-result-set-size\"><a class=\"doc-anchor\" href=\"#note-beware-result-set-size\">§</a>Note: beware result set size.</h6>\n<p>This will attempt to collect the full result set of the query into memory.</p>\n<p>To avoid exhausting available memory, ensure the result set has a known upper bound,\ne.g. using <code>LIMIT</code>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch_one\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">fetch_one</a>&lt;'e, 'c, E&gt;(\n    self,\n    executor: E,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Row\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Row\">Row</a>, Error&gt;<div class=\"where\">where\n    'c: 'e,\n    'q: 'e,\n    A: 'e,\n    E: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Executor.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Executor\">Executor</a>&lt;'c, Database = DB&gt;,</div></h4></section></summary><div class=\"docblock\"><p>Execute the query, returning the first row or [<code>Error::RowNotFound</code>] otherwise.</p>\n<h6 id=\"note-for-best-performance-ensure-the-query-returns-at-most-one-row\"><a class=\"doc-anchor\" href=\"#note-for-best-performance-ensure-the-query-returns-at-most-one-row\">§</a>Note: for best performance, ensure the query returns at most one row.</h6>\n<p>Depending on the driver implementation, if your query can return more than one row,\nit may lead to wasted CPU time and bandwidth on the database server.</p>\n<p>Even when the driver implementation takes this into account, ensuring the query returns at most one row\ncan result in a more optimal query plan.</p>\n<p>If your query has a <code>WHERE</code> clause filtering a unique column by a single value, you’re good.</p>\n<p>Otherwise, you might want to add <code>LIMIT 1</code> to your query.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.fetch_optional\" class=\"method\"><h4 class=\"code-header\">pub async fn <a class=\"fn\">fetch_optional</a>&lt;'e, 'c, E&gt;(\n    self,\n    executor: E,\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;DB as <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a>&gt;::<a class=\"associatedtype\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html#associatedtype.Row\" title=\"type hotshot_query_service::data_source::storage::sql::Database::Row\">Row</a>&gt;, Error&gt;<div class=\"where\">where\n    'c: 'e,\n    'q: 'e,\n    A: 'e,\n    E: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Executor.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Executor\">Executor</a>&lt;'c, Database = DB&gt;,</div></h4></section></summary><div class=\"docblock\"><p>Execute the query, returning the first row or <code>None</code> otherwise.</p>\n<h6 id=\"note-for-best-performance-ensure-the-query-returns-at-most-one-row-1\"><a class=\"doc-anchor\" href=\"#note-for-best-performance-ensure-the-query-returns-at-most-one-row-1\">§</a>Note: for best performance, ensure the query returns at most one row.</h6>\n<p>Depending on the driver implementation, if your query can return more than one row,\nit may lead to wasted CPU time and bandwidth on the database server.</p>\n<p>Even when the driver implementation takes this into account, ensuring the query returns at most one row\ncan result in a more optimal query plan.</p>\n<p>If your query has a <code>WHERE</code> clause filtering a unique column by a single value, you’re good.</p>\n<p>Otherwise, you might want to add <code>LIMIT 1</code> to your query.</p>\n</div></details></div></details>",0,"hotshot_query_service::data_source::storage::sql::transaction::Query"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Query%3C'q,+DB,+A%3E\" class=\"impl\"><a href=\"#impl-Query%3C'q,+DB,+A%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'q, DB, A&gt; Query&lt;'q, DB, A&gt;<div class=\"where\">where\n    DB: <a class=\"trait\" href=\"hotshot_query_service/data_source/storage/sql/trait.Database.html\" title=\"trait hotshot_query_service::data_source::storage::sql::Database\">Database</a> + HasStatementCache,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.persistent\" class=\"method\"><h4 class=\"code-header\">pub fn <a class=\"fn\">persistent</a>(self, value: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.bool.html\">bool</a>) -&gt; Query&lt;'q, DB, A&gt;</h4></section></summary><div class=\"docblock\"><p>If <code>true</code>, the statement will get prepared once and cached to the\nconnection’s statement cache.</p>\n<p>If queried once with the flag set to <code>true</code>, all subsequent queries\nmatching the one with the flag will use the cached statement until the\ncache is cleared.</p>\n<p>If <code>false</code>, the prepared statement will be closed after execution.</p>\n<p>Default: <code>true</code>.</p>\n</div></details></div></details>",0,"hotshot_query_service::data_source::storage::sql::transaction::Query"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[27754]}