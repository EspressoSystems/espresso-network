FROM ubuntu:jammy

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/cdn-whitelist /bin/cdn-whitelist
RUN chmod +x /bin/cdn-whitelist

ENV RUST_LOG="info"

CMD ["cdn-whitelist"]