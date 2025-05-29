FROM ghcr.io/espressosystems/ubuntu-base:main AS builder

ARG TARGETARCH

RUN apt-get update \
    && apt-get install -y curl libcurl4 nodejs npm libusb-1.0.0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY package.json yarn.lock ./

RUN npm install -g yarn \
  && yarn global add typescript ts-node \
  && yarn \
  && rm -rf /usr/local/share/.cache
  
COPY target/$TARGETARCH/release/deploy /bin/deploy
COPY scripts/multisig-upgrade-entrypoint /bin/multisig-upgrade-entrypoint
COPY contracts/script/multisigTransactionProposals/safeSDK ./contracts/script/multisigTransactionProposals/safeSDK/

RUN chmod +x /bin/deploy /bin/multisig-upgrade-entrypoint


# Runner image: deploy binary, ts-node, multisig upgrade script
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y --fix-missing tini libcurl4 nodejs npm libusb-1.0-0 && \
    npm install -g yarn ts-node && \
    rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY --from=builder /usr/local/bin /usr/local/bin
COPY --from=builder /usr/local/share /usr/local/share
COPY --from=builder /app/ /app/
COPY --from=builder /bin/deploy /bin/deploy
COPY --from=builder /bin/multisig-upgrade-entrypoint /bin/multisig-upgrade-entrypoint
ENV MULTISIG_UPGRADE_ENTRYPOINT_PATH=/bin/multisig-upgrade-entrypoint

CMD [ "/bin/deploy"]