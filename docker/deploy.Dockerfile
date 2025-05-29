FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

# Install system dependencies and Node.js via NodeSource
RUN apt-get update && \
    apt-get install -y curl gnupg libcurl4 libusb-1.0-0 tini && \
    curl -fsSL https://deb.nodesource.com/setup_18.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy project metadata and install JS deps
COPY package.json yarn.lock ./

RUN npm install -g yarn && \
    yarn global add typescript ts-node && \
    yarn && \
    rm -rf /usr/local/share/.cache ~/.cache node_modules/.cache

# Copy binaries and scripts
COPY target/$TARGETARCH/release/deploy /bin/deploy
COPY scripts/multisig-upgrade-entrypoint /bin/multisig-upgrade-entrypoint
COPY contracts/script/multisigTransactionProposals/safeSDK ./contracts/script/multisigTransactionProposals/safeSDK/

RUN chmod +x /bin/deploy /bin/multisig-upgrade-entrypoint

# Setup runtime
ENTRYPOINT ["tini", "--"]
ENV MULTISIG_UPGRADE_ENTRYPOINT_PATH=/bin/multisig-upgrade-entrypoint
CMD [ "/bin/deploy" ]
