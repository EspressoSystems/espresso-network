pub(crate) mod nonempty;

use std::time::Duration;

use crate::NetworkError;

/// A variant of `timeout` that merges the timeout error into network error.
pub(crate) async fn until<F, A, E>(t: Duration, fut: F) -> Result<A, NetworkError>
where
    F: Future<Output = Result<A, E>>,
    E: Into<NetworkError>,
{
    match tokio::time::timeout(t, fut).await {
        Ok(Ok(a)) => Ok(a),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(NetworkError::Timeout),
    }
}
