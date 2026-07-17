# DEPRECATED: This image will be removed in a future release.
# Consumers should migrate to ghcr.io/espressosystems/espresso-network/espresso-node.
#
# Legacy compatibility image: provides old binary names (sequencer, sequencer-postgres, sequencer-sqlite)
# as symlinks to the new names (espresso-node, espresso-node-postgres, espresso-node-sqlite).
# Pushed to ghcr.io/espressosystems/espresso-sequencer/sequencer for backward compatibility.
ARG BASE_IMAGE
FROM ${BASE_IMAGE}
RUN ln -sf /bin/espresso-node-postgres /bin/sequencer-postgres && \
    ln -sf /bin/espresso-node-sqlite /bin/sequencer-sqlite && \
    ln -sf /bin/espresso-node /bin/sequencer
CMD ["/bin/sequencer", "--", "http"]
