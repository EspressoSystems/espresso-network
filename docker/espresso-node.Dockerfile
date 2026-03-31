FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/espresso-node /bin/espresso-node-postgres
RUN chmod +x /bin/espresso-node-postgres

COPY target/$TARGETARCH/release/espresso-node-sqlite /bin/espresso-node-sqlite
RUN chmod +x /bin/espresso-node-sqlite

COPY target/$TARGETARCH/release/utils /bin/utils
RUN chmod +x /bin/utils

COPY target/$TARGETARCH/release/reset-storage /bin/reset-storage
RUN chmod +x /bin/reset-storage

COPY target/$TARGETARCH/release/keygen /bin/keygen
RUN chmod +x /bin/keygen

COPY target/$TARGETARCH/release/pub-key /bin/pub-key
RUN chmod +x /bin/pub-key

# Install genesis files for all supported configurations. The desired configuration can be chosen by
# setting `ESPRESSO_SEQUENCER_GENESIS_FILE`.
COPY data/genesis /genesis

# Allow injecting a genesis file with aws secretsmanager
# Set `ESPRESSO_SEQUENCER_GENESIS_SECRET`
COPY docker/scripts/espresso-node-awssecretsmanager.sh /bin/espresso-node-awssecretsmanager.sh
RUN chmod +x /bin/espresso-node-awssecretsmanager.sh

# Copy entrypoint script
COPY scripts/espresso-node-entrypoint /bin/espresso-node
RUN chmod +x /bin/espresso-node

# Set a path to save the consensus config on startup.
#
# Upon restart, the config will be loaded from this file and the node will be able to resume
# progress. The user should connect this path to a Docker volume to ensure persistence of the
# configuration beyond the lifetime of the Docker container itself.
ENV ESPRESSO_SEQUENCER_STORAGE_PATH=/store/sequencer

# Set an L1 safety margin by default. This enables fast startup on chains where the L1 genesis block
# is very old.
ENV ESPRESSO_SEQUENCER_L1_FINALIZED_SAFETY_MARGIN=100

CMD ["/bin/espresso-node", "--", "http"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_SEQUENCER_API_PORT}/healthcheck  || exit 1
EXPOSE ${ESPRESSO_SEQUENCER_API_PORT}
