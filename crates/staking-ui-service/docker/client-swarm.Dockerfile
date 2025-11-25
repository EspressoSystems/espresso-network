FROM ghcr.io/espressosystems/ubuntu-base:main

ARG CARGO_TARGET_DIR=target

COPY $CARGO_TARGET_DIR/release/client-swarm /bin/client-swarm
RUN chmod +x /bin/client-swarm

# HTTP server port.
ENV STAKING_CLIENT_SWARM_PORT=8080

CMD ["/bin/staking-ui-service"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${STAKING_CLIENT_SWARM_PORT}/healthcheck  || exit 1
EXPOSE ${STAKING_CLIENT_SWARM_PORT}
