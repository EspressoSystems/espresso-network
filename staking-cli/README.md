# Espresso staking CLI

A small CI to interact with the stake table contract.

    cargo run --bin staking-cli -p staking-cli -- --help

The CLI is currently intended for testing/demo purposes only.

To load the stake table for the demo, run the demo first

    just demo-native

and once that is started up

    RUST_LOG=info cargo run --bin staking-cli -p staking-cli -- stake-for-demo

it should show that validators are being registered

    2025-03-14T16:10:06.635922Z  INFO staking_cli::demo: Deploying validator 0 with commission 0.00 %
    2025-03-14T16:10:10.665231Z  INFO staking_cli::demo: Deploying validator 1 with commission 1.00 %
    2025-03-14T16:10:14.692189Z  INFO staking_cli::demo: Deploying validator 2 with commission 2.00 %
    2025-03-14T16:10:18.720833Z  INFO staking_cli::demo: Deploying validator 3 with commission 3.00 %
    2025-03-14T16:10:22.560015Z  INFO staking_cli::demo: Deploying validator 4 with commission 4.00 %
