FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/nasty-client /bin/nasty-client
RUN chmod +x /bin/nasty-client

# Run a web server on this port by default. Port can be overridden by the container orchestrator.
ENV ESPRESSO_NASTY_CLIENT_PORT=80

CMD [ "/bin/nasty-client"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_NASTY_CLIENT_PORT}/healthcheck  || exit 1
EXPOSE ${ESPRESSO_NASTY_CLIENT_PORT}
