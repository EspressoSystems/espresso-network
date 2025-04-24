FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update \
    &&  apt-get install -y curl libcurl4 wait-for-it tini \
    &&  rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["tini", "--"]

COPY target/$TARGETARCH/release/update-permissioned-stake-table /bin/update-permissioned-stake-table
RUN chmod +x /bin/update-permissioned-stake-table

CMD [ "/bin/update-permissioned-stake-table"]

