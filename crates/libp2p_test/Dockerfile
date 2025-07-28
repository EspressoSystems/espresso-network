FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH
ARG CONFIG

COPY target/$TARGETARCH/release/libp2p_test /bin/libp2p-test
RUN chmod +x /bin/libp2p-test

COPY crates/libp2p_test/config/$CONFIG/libp2p_test.toml libp2p_test.toml

ENV RUST_LOG=trace

CMD ["/bin/libp2p-test"]
