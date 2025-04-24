FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

ENV ESPRESSO_BUILDER_SERVER_PORT=31003

# Install genesis files for all supported configurations. The desired configuration can be chosen by
# setting `ESPRESSO_BUILDER_GENESIS_FILE`.
COPY data/genesis /genesis
COPY target/$TARGETARCH/release/marketplace-builder /bin/marketplace-builder
RUN chmod +x /bin/marketplace-builder

CMD [ "/bin/marketplace-builder"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_BUILDER_SERVER_PORT}/healthcheck || exit 1
EXPOSE ${ESPRESSO_BUILDER_SERVER_PORT}
