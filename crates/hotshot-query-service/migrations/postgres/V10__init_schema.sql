CREATE TABLE header
(
    height    BIGINT  PRIMARY KEY,
    hash      VARCHAR NOT NULL UNIQUE,
    payload_hash VARCHAR NOT NULL,
    timestamp BIGINT NOT NULL,

    -- For convenience, we store the entire application-specific header type as JSON. Just like
    -- `leaf.leaf` and `leaf.qc`, this allows us to easily reconstruct the entire header using
    -- `serde_json`, and to run queries and create indexes on application-specific header fields
    -- without having a specific column for those fields. In many cases, this will enable new
    -- application-specific API endpoints to be implemented without altering the schema (beyond
    -- possibly adding an index for performance reasons).
    data JSONB NOT NULL
);

CREATE INDEX header_timestamp_idx ON header (timestamp);

CREATE TABLE payload
(
    height BIGINT PRIMARY KEY REFERENCES header (height) ON DELETE CASCADE,
    size   INTEGER,
    data   BYTEA
);

CREATE TABLE vid
(
    height BIGINT PRIMARY KEY REFERENCES header (height) ON DELETE CASCADE,
    common BYTEA  NOT NULL,
    share  BYTEA
);

CREATE TABLE leaf
(
    height     BIGINT  PRIMARY KEY REFERENCES header (height) ON DELETE CASCADE,
    hash       VARCHAR NOT NULL UNIQUE,
    block_hash VARCHAR NOT NULL REFERENCES header (hash) ON DELETE CASCADE,

    -- For convenience, we store the entire leaf and justifying QC as JSON blobs. There is a bit of
    -- redundancy here with the indexed fields above, but it makes it easy to reconstruct the entire
    -- leaf without depending on the specific fields of the application-specific leaf type. We
    -- choose JSON over a binary format, even though it has a larger storage footprint, because
    -- Postgres actually has decent JSON support: we don't have to worry about escaping non-ASCII
    -- characters in inputs, and we can even do queries on the JSON and add indices over sub-objects
    -- of the JSON blobs.
    leaf JSONB NOT NULL,
    qc   JSONB NOT NULL
);

CREATE TABLE transaction
(
    hash VARCHAR NOT NULL,
    -- Block containing this transaction.
    block_height BIGINT NOT NULL REFERENCES header(height) ON DELETE CASCADE,
    -- Position within the block. Transaction indices are an application-specific type, so we store
    -- it as a serialized blob. We use JSON instead of a binary format so that the application can
    -- make use of the transaction index in its own SQL queries.
    index JSONB NOT NULL
);
ALTER TABLE transaction ADD CONSTRAINT transaction_pk PRIMARY KEY (block_height, index);
-- This index is not unique, because nothing stops HotShot from sequencing duplicate transactions.
CREATE INDEX transaction_hash ON transaction (hash);

CREATE TABLE pruned_height (
    id SERIAL PRIMARY KEY,
    -- The height of the last pruned block.
    last_height BIGINT NOT NULL
);

CREATE TABLE last_merklized_state_height (
    id SERIAL PRIMARY KEY,
    height BIGINT NOT NULL
);