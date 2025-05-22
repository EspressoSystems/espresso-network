FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

# Install genesis files for all supported configurations. The desired configuration can be chosen by
# setting `ESPRESSO_BUILDER_GENESIS_FILE`.
COPY data/genesis /genesis

COPY target/$TARGETARCH/release/permissionless-builder /bin/permissionless-builder
RUN chmod +x /bin/permissionless-builder

HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_BUILDER_SERVER_PORT}/healthcheck || exit 1

CMD [ "/bin/permissionless-builder"]
