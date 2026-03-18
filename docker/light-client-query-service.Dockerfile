FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/light-client-query-service /bin/light-client-query-service-postgres
RUN chmod +x /bin/light-client-query-service-postgres

# Install genesis files for all supported configurations. The desired configuration can be chosen by
# setting `LIGHT_CLIENT_GENESIS`.
COPY light-client-query-service/genesis /genesis

# Allow injecting a genesis file with aws secretsmanager
# Set `LIGHT_CLIENT_GENESIS_SECRET`
COPY docker/scripts/light-client-awssecretsmanager.sh /bin/light-client-awssecretsmanager.sh

# Set a path to save the light client state on startup.
#
# Upon restart, the state will be loaded from this file and the node will be able to resume
# progress. The user should connect this path to a Docker volume to ensure persistence of the state
# beyond the lifetime of the Docker container itself, and avoid resyncing from genesis on restart.
ENV LIGHT_CLIENT_DB_PATH=/store/light-client

CMD ["/bin/light-client-query-service"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${QUERY_SERVICE_PORT}/healthcheck  || exit 1
EXPOSE ${QUERY_SERVICE_PORT}
