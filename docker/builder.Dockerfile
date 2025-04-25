FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

# Install genesis files for all supported configurations. The desired configuration can be chosen by
# setting `ESPRESSO_BUILDER_GENESIS_FILE`.
COPY data/genesis /genesis

# Download an SRS file to avoid download at runtime
ENV AZTEC_SRS_PATH=/kzg10-aztec20-srs-1048584.bin
RUN curl -LO https://github.com/EspressoSystems/ark-srs/releases/download/v0.2.0/$AZTEC_SRS_PATH

COPY target/$TARGETARCH/release/permissionless-builder /bin/permissionless-builder
RUN chmod +x /bin/permissionless-builder

HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_BUILDER_SERVER_PORT}/healthcheck || exit 1

CMD [ "/bin/permissionless-builder"]
