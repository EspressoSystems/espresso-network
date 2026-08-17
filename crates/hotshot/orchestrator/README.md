# Orchestrator

This crate implements an orchestrator that coordinates starting the network with a particular configuration. It is
useful for testing and benchmarking. The HTTP server is built with [axum](https://github.com/tokio-rs/axum).

To run the orchestrator: `just example orchestrator http://0.0.0.0:3333 ./crates/orchestrator/run-config.toml`
