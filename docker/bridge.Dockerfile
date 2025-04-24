FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/espresso-bridge /bin/espresso-bridge
RUN chmod +x /bin/espresso-bridge

RUN ln -s /bin/espresso-bridge /bin/bridge

CMD [ "/bin/espresso-bridge"]
