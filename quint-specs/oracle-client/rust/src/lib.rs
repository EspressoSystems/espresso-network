//! Minimal client for the Quint oracle daemon.
//!
//! Instrument your code by IMPORTING this crate and calling [`start_test`] /
//! [`log_action`] at the real transition sites — you never hand-write an HTTP
//! client. Both are NO-OPS unless the oracle set `QUINT_ORACLE_URL` in the
//! environment (the oracle exports it for the test command it spawns), so adding
//! these calls cannot affect normal builds or test runs.
//!
//! # Parallel and async tests
//!
//! The daemon keeps a SINGLE "current test" slot, so two tests reporting at the
//! same time would corrupt each other's trace. This client makes runs safe WITHOUT
//! any daemon change, via two complementary paths:
//!
//! * **Synchronous tests** (the transition runs on the test's own thread):
//!   [`start_test`] buffers the test's steps in a thread-local; when the guard
//!   drops the whole trace is flushed under a process-global lock, so the daemon
//!   receives each test's `POST /test` + `PUT /log…` as one uninterrupted group.
//!   These tests may run in **parallel** (`--test-threads=N`).
//! * **Async/actor tests** (the transition runs on a `tokio` worker thread, not the
//!   test thread): there is no thread-local buffer there, so [`log_action`] sends a
//!   live `PUT /log`. To make it land in the right trace, [`start_test`] registers
//!   the test with the daemon **immediately** (`POST /test/<name>` up front).
//!   Because the daemon has only one slot, such tests MUST run **serially**
//!   (`--test-threads=1`).
//!
//! **Hold the guard for the test's scope:**
//!
//! ```ignore
//! let _t = quint_oracle_client::start_test("test_value_response_skips_when_decided");
//! quint_oracle_client::log_action("onValueResponse", &[
//!     quint_oracle_client::Arg { name: "certValid", value: cert_ok.into(), domain: None },
//! ]);
//! // _t drops here → the buffered trace is flushed atomically.
//! ```
//!
//! Protocol: `POST /test/<name>` then `PUT /log {action, arguments, scopes?}` (see
//! `crates/ORACLE-SPEC.md`). Zero dependencies — raw HTTP/1.1 over std `TcpStream`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;

/// The concrete value of a logged action argument.
pub enum ArgValue {
    Bool(bool),
    Int(u64),
    Str(String),
}

impl ArgValue {
    fn to_json(&self) -> String {
        match self {
            ArgValue::Bool(b) => b.to_string(),
            ArgValue::Int(n) => n.to_string(),
            ArgValue::Str(s) => json_string(s),
        }
    }
}

impl From<bool> for ArgValue {
    fn from(b: bool) -> Self {
        ArgValue::Bool(b)
    }
}
impl From<u64> for ArgValue {
    fn from(n: u64) -> Self {
        ArgValue::Int(n)
    }
}
impl From<usize> for ArgValue {
    fn from(n: usize) -> Self {
        ArgValue::Int(n as u64)
    }
}
impl From<i64> for ArgValue {
    fn from(n: i64) -> Self {
        ArgValue::Int(n as u64)
    }
}
impl From<&str> for ArgValue {
    fn from(s: &str) -> Self {
        ArgValue::Str(s.to_string())
    }
}
impl From<String> for ArgValue {
    fn from(s: String) -> Self {
        ArgValue::Str(s)
    }
}

/// A single logged argument: a Quint action parameter name, its concrete value,
/// and (optionally) the Quint constant set the value is drawn from.
pub struct Arg {
    pub name: &'static str,
    pub value: ArgValue,
    pub domain: Option<&'static str>,
}

// The test currently running on THIS thread, with its buffered step bodies.
// `start_test` sets it, `log_action` appends to it, and the `TestGuard` flushes
// and clears it on drop. Thread-local so bounded-parallel tests never share a
// buffer.
thread_local! {
    static CURRENT: RefCell<Option<TestBuf>> = const { RefCell::new(None) };
}

/// Serializes the per-test FLUSH so the daemon (which keeps one `current_test`
/// slot) receives each test's `POST /test` + `PUT /log…` as one uninterrupted
/// group — even when the tests themselves ran concurrently. Held only for the
/// (fast, network-bound) flush, never during the test body, so test parallelism
/// is preserved.
static FLUSH: Mutex<()> = Mutex::new(());

struct TestBuf {
    name: String,
    /// Pre-rendered `/log` JSON bodies, in call order.
    steps: Vec<String>,
}

/// Per-NODE step buffers for a MULTI-NODE test (integration suites where N nodes
/// emit into one run). `start_test` resets it; [`log_action_for`] appends each
/// step to the calling node's bucket; on guard drop every node is flushed as its
/// OWN daemon trace `<test>::node<i>`. This turns an un-replayable N-node
/// interleaving into N single-node traces a single-node spec can replay.
///
/// `node` is an opaque identity (e.g. a validator address / peer id); the client
/// CANONICALIZES it to a small stable index here (`0,1,2,…` in first-seen order)
/// — raw addresses are random/unbounded and the oracle can't pin them, but the
/// index domain is finite. Stability is per-trace (all that replay needs).
///
/// Guarded by the process-global lock below; integration runs serially, so the
/// per-node map always belongs to the one active test.
struct MultiBuf {
    test: String,
    idx_of: HashMap<String, usize>,
    per_node: Vec<Vec<String>>,
}
static MULTI: Mutex<Option<MultiBuf>> = Mutex::new(None);

/// Begin a test. **Hold the returned guard for the test's scope** (e.g.
/// `let _t = quint_oracle_client::start_test("my_test");`). When it drops at the
/// end of the test, the buffered steps are flushed to the oracle atomically.
/// Returns an inert guard (no-op on drop) when `QUINT_ORACLE_URL` is unset.
///
/// Two delivery paths, depending on where the instrumented code runs:
///   * **Same thread as the test** (synchronous tests): each `log_action` buffers
///     into the thread-local below and the whole group is flushed atomically on
///     drop — so bounded-PARALLEL synchronous tests never corrupt each other.
///   * **A different thread** (async/actor tests: the transition runs on a
///     `tokio` worker, not the test thread): `log_action` can't see this thread's
///     buffer, so it falls back to a live `PUT /log`. For that to land in the
///     right trace the daemon's single `current_test` slot must already be ours —
///     so we **`POST /test/<name>` immediately here**, before any worker logs
///     arrive. INVARIANT: async tests whose transitions run off-thread must be run
///     SERIALLY (`--test-threads=1`); otherwise concurrent immediate-POSTs would
///     thrash the daemon's one slot. (Parallel synchronous tests are fine — they
///     buffer and never live-send.)
#[must_use = "hold the guard for the test's scope; dropping it flushes the trace"]
pub fn start_test(name: &str) -> TestGuard {
    let Ok(url) = env::var("QUINT_ORACLE_URL") else {
        return TestGuard { active: false };
    };
    CURRENT.with(|c| {
        *c.borrow_mut() = Some(TestBuf {
            name: name.to_string(),
            steps: Vec::new(),
        });
    });
    // Reset the per-node buffers for this test (multi-node path via log_action_for).
    {
        let mut g = MULTI.lock().unwrap_or_else(|e| e.into_inner());
        *g = Some(MultiBuf {
            test: name.to_string(),
            idx_of: HashMap::new(),
            per_node: Vec::new(),
        });
    }
    // Register the test up front so off-thread (worker) log_actions during the
    // test land in this test's slot, not None (dropped) or a previous test's.
    let _lock = FLUSH.lock().unwrap_or_else(|e| e.into_inner());
    let _ = send(&url, "POST", &format!("/test/{}", encode_path(name)), None);
    TestGuard { active: true }
}

/// Report that a spec transition completed successfully. On the test's own thread
/// it buffers against the current test (flushed atomically on guard drop). On any
/// OTHER thread — e.g. a `tokio` worker running an async transition — there is no
/// thread-local buffer, so it falls back to a live `PUT /log`; that lands in the
/// test `start_test` already registered with the daemon (see its INVARIANT: such
/// tests must run serially). No-op when `QUINT_ORACLE_URL` is unset.
pub fn log_action(action: &str, args: &[Arg]) {
    log_action_in_scopes(&[], action, args);
}

fn log_action_in_scopes(scopes: &[&str], action: &str, args: &[Arg]) {
    let Ok(url) = env::var("QUINT_ORACLE_URL") else {
        return;
    };
    let body = render_log_body(scopes, action, args);
    let buffered = CURRENT.with(|c| match c.borrow_mut().as_mut() {
        Some(buf) => {
            buf.steps.push(body.clone());
            true
        }
        None => false,
    });
    if !buffered {
        // No active guard: best-effort live send (un-grouped; correct only serially).
        let _ = send(&url, "PUT", "/log", Some(&body));
    }
}

/// Like [`log_action`], but attributes the step to a specific NODE for a MULTI-NODE
/// test. `node` is an opaque identity (validator address / peer id); the client
/// canonicalizes it to a small stable index and buckets the step per node. On guard
/// drop each node is flushed as its own `<test>::node<i>` trace, so a single-node
/// spec can replay each node's slice instead of one un-replayable interleaving. The
/// SPEC stays node-unaware — `node` is a routing key, never a spec argument. No-op
/// when `QUINT_ORACLE_URL` is unset or no test is active.
pub fn log_action_for(node: &str, action: &str, args: &[Arg]) {
    log_action_for_scopes(&[], node, action, args);
}

fn log_action_for_scopes(scopes: &[&str], node: &str, action: &str, args: &[Arg]) {
    if env::var("QUINT_ORACLE_URL").is_err() {
        return;
    }
    let body = render_log_body(scopes, action, args);
    // On the test's OWN thread (synchronous / unit tests) the thread-local buffer is
    // set — use it. It is per-thread, so those tests stay PARALLEL-safe and the node
    // id is irrelevant (one node per test). This is the path a shared emit site takes
    // when driven directly by a unit test.
    let buffered = CURRENT.with(|c| match c.borrow_mut().as_mut() {
        Some(buf) => {
            buf.steps.push(body.clone());
            true
        }
        None => false,
    });
    if buffered {
        return;
    }
    // Off-thread (async/actor integration: the transition runs on a worker, not the
    // test thread) → bucket per node in the global MULTI. Safe because such suites run
    // SERIALLY, so MULTI belongs to the one active test.
    let mut g = MULTI.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(m) = g.as_mut() {
        // Canonicalize the opaque node id to a small index (first-seen order).
        let idx = match m.idx_of.get(node) {
            Some(&i) => i,
            None => {
                let i = m.per_node.len();
                m.per_node.push(Vec::new());
                m.idx_of.insert(node.to_string(), i);
                i
            }
        };
        m.per_node[idx].push(body);
    }
}

/// A logger whose events explicitly target one or more Quint Studio component
/// keys. The oracle only replays them when its config's `eventScope` matches.
pub struct ScopedLogger<'a> {
    scopes: &'a [&'a str],
}

/// Create a component-scoped logger. Use multiple scopes only for a transition
/// that genuinely belongs to every named component abstraction.
pub fn scope<'a>(scopes: &'a [&'a str]) -> ScopedLogger<'a> {
    ScopedLogger { scopes }
}

impl ScopedLogger<'_> {
    pub fn log_action(&self, action: &str, args: &[Arg]) {
        log_action_in_scopes(self.scopes, action, args);
    }

    pub fn log_action_for(&self, node: &str, action: &str, args: &[Arg]) {
        log_action_for_scopes(self.scopes, node, action, args);
    }
}

/// Guard returned by [`start_test`]. Flushes the buffered trace on drop.
#[must_use = "hold the guard for the test's scope; dropping it flushes the trace"]
pub struct TestGuard {
    active: bool,
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let Ok(url) = env::var("QUINT_ORACLE_URL") else {
            return;
        };
        // take() both buffers (releasing their locks) before any network I/O.
        let single = CURRENT.with(|c| c.borrow_mut().take());
        let multi = {
            let mut g = MULTI.lock().unwrap_or_else(|e| e.into_inner());
            g.take()
        };
        // Flush under the global lock so the daemon's single current_test slot stays
        // ours for each group. Recover from a poisoned lock — instrumentation must
        // never panic a test.
        let _lock = FLUSH.lock().unwrap_or_else(|e| e.into_inner());
        // Multi-node: one daemon trace per node (`<test>::node<i>`), each a clean
        // single-node sequence.
        if let Some(m) = multi {
            for (i, steps) in m.per_node.iter().enumerate() {
                if steps.is_empty() {
                    continue;
                }
                let path = format!("/test/{}::node{}", encode_path(&m.test), i);
                let _ = send(&url, "POST", &path, None);
                for body in steps {
                    let _ = send(&url, "PUT", "/log", Some(body));
                }
            }
        }
        // Single-node: the bare test name (skip if it never buffered a step — e.g. a
        // pure multi-node test, whose steps went to the per-node buckets above).
        if let Some(buf) = single {
            if !buf.steps.is_empty() {
                let path = format!("/test/{}", encode_path(&buf.name));
                let _ = send(&url, "POST", &path, None);
                for body in &buf.steps {
                    let _ = send(&url, "PUT", "/log", Some(body));
                }
            }
        }
    }
}

/// Render one `/log` request body: `{"action":..,"arguments":[{name,value,domain?}..]}`.
fn render_log_body(scopes: &[&str], action: &str, args: &[Arg]) -> String {
    let mut body = String::with_capacity(64);
    body.push_str("{\"action\":");
    body.push_str(&json_string(action));
    body.push_str(",\"arguments\":[");
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str("{\"name\":");
        body.push_str(&json_string(a.name));
        body.push_str(",\"value\":");
        body.push_str(&a.value.to_json());
        if let Some(d) = a.domain {
            body.push_str(",\"domain\":");
            body.push_str(&json_string(d));
        }
        body.push('}');
    }
    body.push(']');
    if !scopes.is_empty() {
        body.push_str(",\"scopes\":[");
        for (i, scope) in scopes.iter().enumerate() {
            if i > 0 {
                body.push(',');
            }
            body.push_str(&json_string(scope));
        }
        body.push(']');
    }
    body.push('}');
    body
}

/// JSON-encode a string (quotes + escapes the minimal set).
fn json_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Percent-encode anything that is not a safe URL path character. Test names are
/// ASCII identifiers in practice, so this is mostly a pass-through.
fn encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
            out.push(c);
        } else {
            for b in c.to_string().as_bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// Write one HTTP/1.1 request and drain the response. Best-effort: any error is
/// swallowed by the callers so instrumentation never affects test outcomes.
fn send(url: &str, method: &str, path: &str, body: Option<&str>) -> Result<(), String> {
    // The daemon URL is `http://host:port` (no path). Split host/port.
    let authority = url
        .strip_prefix("http://")
        .ok_or("QUINT_ORACLE_URL must start with http://")?
        .trim_end_matches('/');
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|e| e.to_string())?),
        None => (authority, 80),
    };

    let mut stream = TcpStream::connect((host, port)).map_err(|e| e.to_string())?;
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

    let body = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\r\n\
         {body}",
        len = body.len(),
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;

    // Drain the response so the daemon finishes processing this request before
    // the connection closes. We don't parse it — the report is built at the end.
    let mut sink = Vec::new();
    let _ = stream.read_to_end(&mut sink);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Instant;

    type Reqs = Arc<Mutex<Vec<(String, String, String)>>>; // (method, path, body)

    /// A throwaway HTTP/1.1 server that records each request and replies 204.
    /// Returns the `http://host:port` URL and the shared request log.
    fn spawn_mock_daemon() -> (String, Reqs) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let reqs: Reqs = Arc::new(Mutex::new(Vec::new()));
        let sink = reqs.clone();
        thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(mut stream) = conn else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                let mut it = line.split_whitespace();
                let method = it.next().unwrap_or("").to_string();
                let path = it.next().unwrap_or("").to_string();
                let mut len = 0usize;
                loop {
                    let mut h = String::new();
                    if reader.read_line(&mut h).is_err() || h == "\r\n" || h.is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0u8; len];
                if len > 0 {
                    let _ = reader.read_exact(&mut body);
                }
                sink.lock().unwrap().push((
                    method,
                    path,
                    String::from_utf8_lossy(&body).into_owned(),
                ));
                let _ = stream.write_all(
                    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                let _ = stream.flush();
            }
        });
        (url, reqs)
    }

    fn eventually(reqs: &Reqs, pred: impl Fn(&(String, String, String)) -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if reqs.lock().unwrap().iter().any(&pred) {
                return true;
            }
            thread::sleep(Duration::from_millis(10));
        }
        false
    }

    // One test (the env var is process-global; cargo parallelizes tests, so keep it
    // in a single sequential function). Covers both delivery paths plus the no-op.
    #[test]
    fn delivery_paths() {
        // (a) No env var → complete no-op (normal builds/tests are unaffected).
        // SAFETY: single-threaded; no other thread reads the var here.
        unsafe { env::remove_var("QUINT_ORACLE_URL") };
        let g = start_test("inert");
        log_action("x", &[]);
        drop(g); // must not panic or connect anywhere

        // (b) With the daemon set, verify the two paths that make async coverage work.
        let (url, reqs) = spawn_mock_daemon();
        // SAFETY: single-threaded setup before any worker thread reads the var.
        unsafe { env::set_var("QUINT_ORACLE_URL", &url) };

        let guard = start_test("t_async");
        // 1. start_test registers the test with the daemon IMMEDIATELY (POST /test),
        //    so a worker-thread log_action arriving mid-test lands in the right slot.
        assert!(
            eventually(&reqs, |(m, p, _)| m == "POST" && p == "/test/t_async"),
            "start_test must POST /test/<name> immediately, before the guard drops"
        );

        // 2. a log_action on a DIFFERENT thread (no thread-local guard there — the
        //    async/actor case) is still SENT live (PUT /log), not silently dropped.
        thread::spawn(|| {
            log_action(
                "onValueResponse",
                &[Arg {
                    name: "certValid",
                    value: true.into(),
                    domain: None,
                }],
            );
        })
        .join()
        .unwrap();
        assert!(
            eventually(&reqs, |(m, p, b)| m == "PUT"
                && p == "/log"
                && b.contains("onValueResponse")),
            "an off-thread log_action must live-send PUT /log (not be dropped)"
        );

        // 3. Per-node routing: log_action_for from WORKER threads (no thread-local
        //    buffer — the async/actor case) buckets by node identity; on drop each
        //    node flushes as its OWN `<test>::node<i>` trace (canonicalized index,
        //    first-seen order). addrA twice → still node0; addrB → node1.
        for (node, action) in [
            ("addrA", "onValueResponse"),
            ("addrB", "storeProposedValue"),
            ("addrA", "storeProposedValue"),
        ] {
            thread::spawn(move || log_action_for(node, action, &[]))
                .join()
                .unwrap();
        }

        drop(guard);
        assert!(
            eventually(&reqs, |(m, p, _)| m == "POST"
                && p == "/test/t_async::node0"),
            "node addrA must flush as <test>::node0"
        );
        assert!(
            eventually(&reqs, |(m, p, _)| m == "POST"
                && p == "/test/t_async::node1"),
            "node addrB must flush as <test>::node1"
        );
        // SAFETY: no other thread is reading the var at this point.
        unsafe { env::remove_var("QUINT_ORACLE_URL") };
    }

    #[test]
    fn scoped_log_body_carries_every_target_component() {
        // The documented ergonomic form must keep the temporary scope array
        // alive for the logger's local lifetime.
        let logger = scope(&["bank", "ledger"]);
        logger.log_action("no-op-without-oracle", &[]);

        let body = render_log_body(
            &["bank", "ledger"],
            "deposit",
            &[Arg {
                name: "amount",
                value: 4usize.into(),
                domain: Some("amounts"),
            }],
        );
        assert_eq!(
            body,
            "{\"action\":\"deposit\",\"arguments\":[{\"name\":\"amount\",\"value\":4,\"domain\":\"amounts\"}],\"scopes\":[\"bank\",\"ledger\"]}"
        );
    }
}
