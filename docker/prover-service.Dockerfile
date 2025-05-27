FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH


# copy the binaries
COPY target/$TARGETARCH/release/state-prover /usr/local/bin/state-prover
RUN chmod +x /usr/local/bin/state-prover

# When running as a Docker service, we always want a healthcheck endpoint, so set a default for the
# port that the HTTP server will run on. This can be overridden in any given deployment environment.
ENV ESPRESSO_PROVER_SERVICE_PORT=80
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_PROVER_SERVICE_PORT}/healthcheck || exit 1

CMD [ "state-prover", "-d" ]
