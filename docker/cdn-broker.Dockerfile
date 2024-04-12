FROM ubuntu:jammy

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*

COPY target/$TARGETARCH/release/cdn-broker /bin/cdn-broker
RUN chmod +x /bin/cdn-broker

ENV RUST_LOG="info"

HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_CDN_SERVER_METRICS_PORT} || exit 1
ENTRYPOINT ["cdn-broker"]