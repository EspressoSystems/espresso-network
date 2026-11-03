FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH
ARG LITESTREAM_VERSION=0.5.18

RUN case "${TARGETARCH}" in \
      amd64) litestream_arch=x86_64 ;; \
      arm64) litestream_arch=arm64 ;; \
      *) echo "unsupported arch: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && release="https://github.com/benbjohnson/litestream/releases/download/v${LITESTREAM_VERSION}" \
    && tarball="litestream-${LITESTREAM_VERSION}-linux-${litestream_arch}.tar.gz" \
    && workdir="$(mktemp --directory)" \
    && cd "${workdir}" \
    && curl --fail --silent --show-error --location --remote-name "${release}/checksums.txt" \
    && curl --fail --silent --show-error --location --remote-name "${release}/${tarball}" \
    && grep " ${tarball}\$" checksums.txt | sha256sum --check - \
    && tar --extract --gzip --file "${tarball}" --directory /usr/local/bin litestream \
    && cd / \
    && rm -rf "${workdir}"

COPY target/$TARGETARCH/release/query-watchdog /bin/query-watchdog
RUN chmod +x /bin/query-watchdog

COPY docker/query-watchdog/litestream.yml /etc/litestream.yml
COPY docker/query-watchdog/entrypoint.sh /bin/query-watchdog-entrypoint
RUN chmod +x /bin/query-watchdog-entrypoint

# Run a web server on this port by default. Port can be overridden by the container orchestrator.
ENV ESPRESSO_QUERY_WATCHDOG_PORT=80
ENV ESPRESSO_QUERY_WATCHDOG_DB_PATH=/data/state.sqlite

CMD [ "/bin/query-watchdog-entrypoint" ]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_QUERY_WATCHDOG_PORT}/healthcheck || exit 1
EXPOSE ${ESPRESSO_QUERY_WATCHDOG_PORT}
