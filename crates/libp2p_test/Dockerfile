FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH
ARG CONFIG

COPY target/$TARGETARCH/release/libp2p_test /bin/libp2p-test
RUN chmod +x /bin/libp2p-test

ENV RUST_LOG=trace

CMD ["/bin/libp2p-test"]
