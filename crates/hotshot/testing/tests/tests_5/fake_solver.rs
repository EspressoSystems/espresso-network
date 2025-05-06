use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResult, node_types::TestTypes,
};
use hotshot_fakeapi::fake_solver::FakeSolverState;
use hotshot_testing::helpers::key_pair_for_id;
use hotshot_types::traits::{node_implementation::NodeType, signature_key::SignatureKey};
use tracing::instrument;
use url::Url;

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn test_fake_solver_fetch_non_permissioned_no_error() {
    use tokio::spawn;

    hotshot::helpers::initialize_logging();

    let solver_state = FakeSolverState::new(
        None, /* 0% error rate */
        vec![
            "http://localhost:1111".parse().unwrap(),
            "http://localhost:1112".parse().unwrap(),
            "http://localhost:1113".parse().unwrap(),
        ],
    );

    // Fire up the solver.
    let solver_url: Url = format!(
        "http://localhost:{}",
        portpicker::pick_unused_port().unwrap()
    )
    .parse()
    .unwrap();
    let url = solver_url.clone();
    let solver_handle = spawn(async move {
        solver_state.run::<TestTypes>(url).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Then, hit the API
    let resp = client
        .get(solver_url.join("v0/api/auction_results/1").unwrap())
        .send()
        .await
        .unwrap()
        .json::<TestAuctionResult>()
        .await
        .unwrap();

    solver_handle.abort();

    assert_eq!(resp.urls[0], Url::parse("http://localhost:1111/").unwrap());
    assert_eq!(resp.urls[1], Url::parse("http://localhost:1112/").unwrap());
    assert_eq!(resp.urls[2], Url::parse("http://localhost:1113/").unwrap());
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn test_fake_solver_fetch_non_permissioned_with_errors() {
    use tokio::spawn;

    hotshot::helpers::initialize_logging();

    let solver_state =
        FakeSolverState::new(Some(0.5), vec!["http://localhost:1111".parse().unwrap()]);

    // Fire up the solver.
    let solver_url: Url = format!(
        "http://localhost:{}",
        portpicker::pick_unused_port().unwrap()
    )
    .parse()
    .unwrap();
    let url = solver_url.clone();
    let solver_handle = spawn(async move {
        solver_state.run::<TestTypes>(url).await.unwrap();
    });

    let client = reqwest::Client::new();
    let mut payloads = Vec::new();

    for i in 0..10 {
        // Then, hit the API
        let resp = client
            .get(
                solver_url
                    .join(&format!("v0/api/auction_results/{i}"))
                    .unwrap(),
            )
            .send()
            .await;

        if let Err(ref e) = resp {
            // We want to die if we don't get a 500, because that's an unexpected error.
            assert!(
                e.is_status(),
                "Got unexpected error response; error = {e:?}"
            );

            // Otherwise, make sure it's a 500 error
            let status = e.status().unwrap();

            // if it is, we're good to go, and we expect this.
            assert_eq!(
                status,
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                "Got unexpected error code; code = {status:?}"
            );
        } else {
            let resp = resp.unwrap();

            if resp.status() != reqwest::StatusCode::OK {
                assert_eq!(
                    resp.status(),
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                    "Got unexpected error code; code = {:?}",
                    resp.status(),
                );

                // Early return if it's an okay status code
                return;
            }

            let payload = resp.json::<TestAuctionResult>().await.unwrap();
            payloads.push(payload);
        }
    }

    solver_handle.abort();

    // Assert over the payloads with a 50% error rate.
    for payload in payloads {
        assert_eq!(
            payload.urls[0],
            Url::parse("http://localhost:1111/").unwrap()
        );
    }
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn test_fake_solver_fetch_permissioned_no_error() {
    use tokio::spawn;

    hotshot::helpers::initialize_logging();

    let solver_state = FakeSolverState::new(
        None, /* 0% error rate */
        vec![
            "http://localhost:1111".parse().unwrap(),
            "http://localhost:1112".parse().unwrap(),
            "http://localhost:1113".parse().unwrap(),
        ],
    );

    // We need a private key
    let (private_key, _) = key_pair_for_id::<TestTypes>(0);

    // Fire up the solver.
    let solver_url: Url = format!(
        "http://localhost:{}",
        portpicker::pick_unused_port().unwrap()
    )
    .parse()
    .unwrap();
    let url = solver_url.clone();
    let solver_handle = spawn(async move {
        solver_state.run::<TestTypes>(url).await.unwrap();
    });

    let client = reqwest::Client::new();
    let encoded_signature: tagged_base64::TaggedBase64 =
        <TestTypes as NodeType>::SignatureKey::sign(&private_key, &[0; 32])
            .unwrap()
            .into();

    // Then, hit the API
    let resp = client
        .get(
            solver_url
                .join(&format!("v0/api/auction_results/1/{encoded_signature}"))
                .unwrap(),
        )
        .send()
        .await
        .unwrap()
        .json::<TestAuctionResult>()
        .await
        .unwrap();

    solver_handle.abort();

    assert_eq!(resp.urls[0], Url::parse("http://localhost:1111/").unwrap());
    assert_eq!(resp.urls[1], Url::parse("http://localhost:1112/").unwrap());
    assert_eq!(resp.urls[2], Url::parse("http://localhost:1113/").unwrap());
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn test_fake_solver_fetch_permissioned_with_errors() {
    use tokio::spawn;

    hotshot::helpers::initialize_logging();

    let solver_state =
        FakeSolverState::new(Some(0.5), vec!["http://localhost:1111".parse().unwrap()]);

    // We need a private key
    let (private_key, _) = key_pair_for_id::<TestTypes>(0);

    // Fire up the solver.
    let solver_url: Url = format!(
        "http://localhost:{}",
        portpicker::pick_unused_port().unwrap()
    )
    .parse()
    .unwrap();
    let url = solver_url.clone();
    let solver_handle = spawn(async move {
        solver_state.run::<TestTypes>(url).await.unwrap();
    });

    let client = reqwest::Client::new();
    let mut payloads = Vec::new();
    let encoded_signature: tagged_base64::TaggedBase64 =
        <TestTypes as NodeType>::SignatureKey::sign(&private_key, &[0; 32])
            .unwrap()
            .into();

    for i in 0..10 {
        // Then, hit the API
        let resp = client
            .get(
                solver_url
                    .join(&format!("v0/api/auction_results/{i}/{encoded_signature}"))
                    .unwrap(),
            )
            .send()
            .await;

        if let Err(ref e) = resp {
            // We want to die if we don't get a 500, because that's an unexpected error.
            assert!(
                e.is_status(),
                "Got unexpected error response; error = {e:?}"
            );

            // Otherwise, make sure it's a 500 error
            let status = e.status().unwrap();

            // if it is, we're good to go, and we expect this.
            assert_eq!(
                status,
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                "Got unexpected error code; code = {status:?}"
            );
        } else {
            let resp = resp.unwrap();

            if resp.status() != reqwest::StatusCode::OK {
                assert_eq!(
                    resp.status(),
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                    "Got unexpected error code; code = {:?}",
                    resp.status(),
                );

                // Early return if it's an okay status code
                return;
            }

            let payload = resp.json::<TestAuctionResult>().await.unwrap();
            payloads.push(payload);
        }
    }

    solver_handle.abort();

    // Assert over the payloads with a 50% error rate.
    for payload in payloads {
        assert_eq!(
            payload.urls[0],
            Url::parse("http://localhost:1111/").unwrap()
        );
    }
}
