FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH

COPY target/$TARGETARCH/release/espresso-bridge /bin/espresso-bridge
RUN chmod +x /bin/espresso-bridge

RUN ln -s /bin/espresso-bridge /bin/bridge

CMD [ "/bin/espresso-bridge"]
