FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

# Copy all binaries and scripts in one step
COPY \
  target/${TARGETARCH}/release/sequencer \
  target/${TARGETARCH}/release/sequencer-sqlite \
  target/${TARGETARCH}/release/utils \
  target/${TARGETARCH}/release/reset-storage \
  target/${TARGETARCH}/release/keygen \
  target/${TARGETARCH}/release/pub-key \
  docker/scripts/sequencer-awssecretsmanager.sh \
  scripts/sequencer-entrypoint \
  /bin/

# Make all copied files executable
RUN chmod +x \
  /bin/sequencer \
  /bin/sequencer-sqlite \
  /bin/utils \
  /bin/reset-storage \
  /bin/keygen \
  /bin/pub-key \
  /bin/sequencer-awssecretsmanager.sh \
  /bin/sequencer-entrypoint && \
  mv /bin/sequencer-entrypoint /bin/sequencer

# Copy genesis files
COPY data/genesis /genesis

# 1. Set a path to save the consensus config on startup.
# Upon restart, the config will be loaded from this file and the node will be
# able to resume progress. The user should connect this path to a Docker volume
# to ensure persistence of the configuration beyond the lifetime of the Docker
# container itself.

# 2. Set an L1 safety margin by default.
# This enables fast startup on chains where the L1 genesis block is very old.
ENV ESPRESSO_SEQUENCER_STORAGE_PATH=/store/sequencer \
    ESPRESSO_SEQUENCER_L1_FINALIZED_SAFETY_MARGIN=100

CMD ["/bin/sequencer", "--", "http"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_SEQUENCER_API_PORT}/healthcheck || exit 1
EXPOSE ${ESPRESSO_SEQUENCER_API_PORT}
