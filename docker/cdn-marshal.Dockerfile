FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/cdn-marshal /bin/cdn-marshal
RUN chmod +x /bin/cdn-marshal

ENV RUST_LOG="info"

HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_CDN_SERVER_METRICS_PORT}/metrics || exit 1
CMD ["cdn-marshal"]
