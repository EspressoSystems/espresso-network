//! Minimal HTTP test servers built on `warp`.

use tokio::net::TcpListener;
use url::Url;
use warp::{Filter, http::StatusCode};

/// Serve `filter` on an ephemeral port, returning its base URL (`http://127.0.0.1:<port>/`). The
/// server runs until the process exits.
pub async fn serve_on_random_port(
    filter: impl Filter<Extract = impl warp::Reply> + Clone + Send + Sync + 'static,
) -> Url {
    let listener = TcpListener::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0u16)))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(warp::serve(filter).incoming(listener).run());
    format!("http://127.0.0.1:{port}/").parse().unwrap()
}

/// Serve every request with the same status, content type, and body.
///
/// `content_type` and `body` are `&'static str` because they are captured by the `'static`
/// filter closure passed to [`serve_on_random_port`].
pub async fn serve_fixed(
    status: StatusCode,
    content_type: &'static str,
    body: &'static str,
) -> Url {
    let route = warp::any().map(move || {
        warp::reply::with_header(
            warp::reply::with_status(body, status),
            "content-type",
            content_type,
        )
    });
    serve_on_random_port(route).await
}
