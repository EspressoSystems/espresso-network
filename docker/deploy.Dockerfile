FROM ghcr.io/espressosystems/nodejs-base:main

ARG TARGETARCH

WORKDIR /app
COPY package.json yarn.lock ./

# TODO: if this works, install make in base image
RUN apt-get update \
    && apt-get install -y --no-install-recommends make \
    && rm -rf /var/lib/apt/lists/*

RUN yarn && rm -rf /usr/local/share/.cache

COPY target/$TARGETARCH/release/deploy /bin/deploy
RUN chmod +x /bin/deploy

COPY scripts/multisig-upgrade-entrypoint /bin/multisig-upgrade-entrypoint
RUN chmod +x /bin/multisig-upgrade-entrypoint
ENV MULTISIG_UPGRADE_ENTRYPOINT_PATH=/bin/multisig-upgrade-entrypoint

COPY contracts/script/multisigTransactionProposals/safeSDK ./contracts/script/multisigTransactionProposals/safeSDK/

CMD ["/bin/deploy"]
