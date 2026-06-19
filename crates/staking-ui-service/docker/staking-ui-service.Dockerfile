FROM ghcr.io/espressosystems/ubuntu-base:main

ARG CARGO_TARGET_DIR=target

COPY $CARGO_TARGET_DIR/release/staking-ui-service /bin/staking-ui-service
RUN chmod +x /bin/staking-ui-service

# SQLite storage path.
ENV ESPRESSO_STAKING_SERVICE_STORAGE=/store/staking-ui-service

# HTTP server port.
ENV ESPRESSO_STAKING_SERVICE_PORT=8080

CMD ["/bin/staking-ui-service"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_STAKING_SERVICE_PORT}/healthcheck  || exit 1
EXPOSE ${ESPRESSO_STAKING_SERVICE_PORT}
