FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/node-metrics /bin/node-metrics
RUN chmod +x /bin/node-metrics

# Run a web server on this port by default. Port can be overridden by the container orchestrator.
ENV ESPRESSO_NODE_VALIDATOR_PORT=80

CMD [ "/bin/node-metrics"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_NODE_VALIDATOR_PORT}/healthcheck  || exit 1
EXPOSE ${ESPRESSO_NODE_VALIDATOR_PORT}
