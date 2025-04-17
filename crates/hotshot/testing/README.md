# Purpose

Infrastructure and integration tests for hotshot. Since a lot of our tests can take a while to run, they've been split into groups to allow for parallelization in CI.

# Usage

The overall control flow is:

```ignore
TestBuilder::default().build() ->  TestLauncher::launch()      -> TestRunner::execute()
|                                  |                              |
- easy override setup fn           - more explicit overrides      - executes the test
|                                    | for networking, storage,
- easy override correctness fn       | hooks/overrides etc
|
- easily add in hooks
|
- easily override launching
```

Easily overriding setup/correctness checks/hooks and launching is all done by anonymous functions. Fairly sane and configurable setup and correct check functions may be generated from the round builder. The intended workflow should look like:

```rust
use std::sync::Arc;
use futures::FutureExt;
use hotshot_example_types::test_types::StaticNodeImplType;
use hotshot_example_types::round::RoundHook;
use hotshot_example_types::test_types::StaticCommitteeTestTypes;
use hotshot_example_types::test_builder::TestBuilder;
use hotshot_example_types::test_builder::TestMetadata;

async {
    // specify general characteristics of the test in TestMetadata
    let metadata = TestMetadata {
        total_nodes: 10,
        start_nodes: 10,
        num_succeeds: 5,
        failure_threshold: 10,
        ..Default::default()
    };

    // construct the builder
    let mut test_builder = TestBuilder {
        metadata,
        /// we could build a check
        check: None,
        /// or a round setup if we want
        setup: None
    };

    // construct the launcher
    // this may be used to manually override any of the round functions
    let test_launcher = test_builder.build::<StaticCommitteeTestTypes, StaticNodeImplType>();

    /// now let's add in a custom hook to print some debugging information at the beginning
    /// of each view
    let hook =
        RoundHook(Arc::new(move |_runner, ctx| {
            async move {
                tracing::error!("Context for this view is {:#?}", ctx);
                Ok(())
            }
            .boxed_local()
        }));

    /// add the hook, launch the test, then run it.
    test_launcher.push_hook(hook).launch().run_test().await.unwrap();

};
```

See TODO for examples.
