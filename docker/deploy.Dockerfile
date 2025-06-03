FROM ghcr.io/espressosystems/nodejs-base:main

ARG TARGETARCH

WORKDIR /app
COPY package.json yarn.lock ./

RUN yarn && rm -rf /usr/local/share/.cache

COPY target/$TARGETARCH/release/deploy /bin/deploy
COPY scripts/multisig-upgrade-entrypoint /bin/multisig-upgrade-entrypoint
COPY contracts/script/multisigTransactionProposals/safeSDK ./contracts/script/multisigTransactionProposals/safeSDK/

CMD ["/bin/deploy"]
