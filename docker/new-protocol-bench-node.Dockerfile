FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/new-protocol-bench-node /bin/new-protocol-bench-node
RUN chmod +x /bin/new-protocol-bench-node

ENTRYPOINT ["/bin/new-protocol-bench-node"]
