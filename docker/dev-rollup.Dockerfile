FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/dev-rollup /bin/dev-rollup
RUN chmod +x /bin/dev-rollup

CMD [ "/bin/dev-rollup"]
