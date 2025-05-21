FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/staking-cli /bin/staking-cli
RUN chmod +x /bin/staking-cli

CMD [ "staking-cli" ]
