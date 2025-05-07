FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/state-relay-server /bin/state-relay-server
RUN chmod +x /bin/state-relay-server

ENV ESPRESSO_STATE_RELAY_SERVER_PORT=40004
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_STATE_RELAY_SERVER_PORT}/healthcheck  || exit 1

EXPOSE ${ESPRESSO_STATE_RELAY_SERVER_PORT}

CMD [ "/bin/state-relay-server"]
