FROM ghcr.io/espressosystems/ubuntu-base:ubuntu-26

ARG TARGETARCH

COPY target/$TARGETARCH/release/deploy /bin/deploy
RUN chmod +x /bin/deploy

CMD ["/bin/deploy"]
