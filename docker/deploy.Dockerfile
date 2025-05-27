FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/deploy /bin/deploy
RUN chmod +x /bin/deploy

CMD [ "/bin/deploy"]
