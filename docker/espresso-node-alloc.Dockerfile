# syntax=docker/dockerfile:1
# Like espresso-node-legacy.Dockerfile: start from the published image and swap in two binaries (here built with an alternative global allocator).
ARG BASE_IMAGE=ghcr.io/espressosystems/espresso-network/espresso-node:main
FROM ${BASE_IMAGE}
COPY target/release/espresso-node /bin/espresso-node-postgres
COPY target/release/espresso-node-sqlite /bin/espresso-node-sqlite
