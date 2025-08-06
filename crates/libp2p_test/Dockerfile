FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH
ARG CONFIG

COPY target/$TARGETARCH/release/libp2p_test /bin/libp2p-test
RUN chmod +x /bin/libp2p-test

ENV RUST_LOG=trace

CMD ["sh", "-c", "while [ ! -f /app_config/libp2p_test.toml ]; do echo 'Waiting for config...'; sleep 2; done && exec /bin/libp2p-test"]