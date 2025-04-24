FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/marketplace-solver /bin/marketplace-solver
RUN chmod +x /bin/marketplace-solver
 
ENV ESPRESSO_MARKETPLACE_SOLVER_API_PORT=25000

CMD ["/bin/marketplace-solver"]


HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_MARKETPLACE_SOLVER_API_PORT}/healthcheck  || exit 1
EXPOSE ${ESPRESSO_MARKETPLACE_SOLVER_API_PORT}