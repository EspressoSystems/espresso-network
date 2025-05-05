FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    nodejs  \
    npm \
    && npm install -g ts-node typescript \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && . $HOME/.cargo/env \
    &&  rm -rf /var/lib/apt/lists/* 

ENV PATH="/root/.cargo/bin:${PATH}"
    
ENTRYPOINT ["tini", "--"]

WORKDIR /app

COPY target/$TARGETARCH/release/deploy /bin/deploy
RUN chmod +x /bin/deploy

COPY package.json package-lock.json yarn.lock ./
RUN npm install

COPY contracts/script/multisigTransactionProposals/safeSDK /app/contracts/script/multisigTransactionProposals/safeSDK

CMD [ "/bin/deploy"]