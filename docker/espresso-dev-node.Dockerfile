FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/espresso-dev-node /bin/espresso-dev-node
RUN chmod +x /bin/espresso-dev-node

# Download the anvil binary
RUN curl -L https://github.com/foundry-rs/foundry/releases/download/nightly/foundry_nightly_linux_${TARGETARCH}.tar.gz --output -| tar -xzvf - -C /bin/ anvil

# When running as a Docker service, we always want a healthcheck endpoint, so set a default for the
# port that the HTTP server will run on. This can be overridden in any given deployment environment.
ENV ESPRESSO_SEQUENCER_API_PORT=8770
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_SEQUENCER_API_PORT}/status/block-height || exit 1

# A storage directory is required to run the node. Set one inside the container by default. For
# persistence between runs, the user can optionally set up a volume mounted at this path.
ENV ESPRESSO_SEQUENCER_STORAGE_PATH=/data/espresso

EXPOSE 8770
EXPOSE 8771
EXPOSE 8772

CMD [ "/bin/espresso-dev-node" ]
