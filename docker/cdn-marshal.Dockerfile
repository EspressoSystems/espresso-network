FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/cdn-marshal /bin/cdn-marshal
RUN chmod +x /bin/cdn-marshal

ENV RUST_LOG="info"

HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_CDN_SERVER_METRICS_PORT}/metrics || exit 1
CMD ["cdn-marshal"]
