FROM ghcr.io/espressosystems/ubuntu-base:ubuntu-26

ARG TARGETARCH

COPY target/$TARGETARCH/release/cdn-whitelist /bin/cdn-whitelist
RUN chmod +x /bin/cdn-whitelist

ENV RUST_LOG="info"

CMD ["cdn-whitelist"]
