FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/orchestrator /bin/orchestrator
RUN chmod +x /bin/orchestrator

ENV ESPRESSO_WEB_SERVER_PORT=44000

# Set up view timing for optimal performance in high volume conditions. We set a fairly long minimum
# propose time to wait to build up a large block before proposing.
ENV ESPRESSO_ORCHESTRATOR_MIN_PROPOSE_TIME=10s
# We set the minimum block size to 1, limiting the number of empty blocks but allowing us to propose
# a block immediately after the minimum propose time, keeping latency low in low-volume cases.
ENV ESPRESSO_ORCHESTRATOR_MIN_TRANSACTIONS=1
# Since min transactions is 1, max propose time only controls the frequency of empty blocks. We set
# this not too much higher than min propose time, for reasonable latency in low volume conditions,
# when empty blocks are required to push non-empty blocks through the pipeline.
ENV ESPRESSO_ORCHESTRATOR_MAX_PROPOSE_TIME=30s
# The view timeout is larger, since it should only matter when something goes wrong (i.e. leader
# failure).
ENV ESPRESSO_ORCHESTRATOR_NEXT_VIEW_TIMEOUT=5m

CMD [ "/bin/orchestrator"]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_ORCHESTRATOR_PORT}/healthcheck || exit 1

EXPOSE ${ESPRESSO_ORCHESTRATOR_PORT}
