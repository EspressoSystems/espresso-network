FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/espresso-dev-node /bin/espresso-dev-node
RUN chmod +x /bin/espresso-dev-node

# Download and verify the anvil binary using verified GitHub release download
COPY scripts/download-github-release.sh /tmp/download-github-release.sh
RUN chmod +x /tmp/download-github-release.sh && \
    /tmp/download-github-release.sh \
      --repo foundry-rs/foundry \
      --tag nightly \
      --asset "foundry_nightly_linux_${TARGETARCH}.tar.gz" \
      --extract-to /bin \
      --extract-file anvil && \
    rm /tmp/download-github-release.sh

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
