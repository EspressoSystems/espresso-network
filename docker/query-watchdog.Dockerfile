FROM ghcr.io/espressosystems/ubuntu-base:main

ARG TARGETARCH
ARG LITESTREAM_VERSION=0.5.18

RUN case "${TARGETARCH}" in \
      amd64) litestream_arch=x86_64 ;; \
      arm64) litestream_arch=arm64 ;; \
      *) echo "unsupported arch: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && deb="litestream-${LITESTREAM_VERSION}-linux-${litestream_arch}.deb" \
    && url="https://github.com/benbjohnson/litestream/releases/download/v${LITESTREAM_VERSION}" \
    && tmp="$(mktemp -d)" \
    && curl --fail --silent --show-error --location --output "${tmp}/${deb}" "${url}/${deb}" \
    && curl --fail --silent --show-error --location --output "${tmp}/checksums.txt" "${url}/checksums.txt" \
    && (cd "${tmp}" && grep " ${deb}\$" checksums.txt | sha256sum --check -) \
    && dpkg -i "${tmp}/${deb}" \
    && rm -rf "${tmp}"

COPY target/$TARGETARCH/release/query-watchdog /bin/query-watchdog
RUN chmod +x /bin/query-watchdog

COPY docker/query-watchdog/litestream.yml /etc/litestream.yml
COPY docker/query-watchdog/entrypoint.sh /bin/query-watchdog-entrypoint
RUN chmod +x /bin/query-watchdog-entrypoint

# Run a web server on this port by default. Port can be overridden by the container orchestrator.
ENV ESPRESSO_QUERY_WATCHDOG_PORT=80
# Fixed path instead of the platform default: the entrypoint (litestream restore) and
# /etc/litestream.yml read the same variable, so all three agree on one file.
ENV ESPRESSO_QUERY_WATCHDOG_DB_PATH=/data/state.sqlite

CMD [ "/bin/query-watchdog-entrypoint" ]
HEALTHCHECK --interval=1s --timeout=1s --retries=100 CMD curl --fail http://localhost:${ESPRESSO_QUERY_WATCHDOG_PORT}/healthcheck || exit 1
EXPOSE ${ESPRESSO_QUERY_WATCHDOG_PORT}
