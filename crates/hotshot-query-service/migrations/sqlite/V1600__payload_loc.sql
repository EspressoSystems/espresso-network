-- Locators of block payloads stored in the blob store instead of `payload.data`, by block height.
-- A payload shared by several blocks has one `payload` row and one locator per height.
CREATE TABLE payload_loc (
    height BIGINT PRIMARY KEY,
    loc    TEXT   NOT NULL
);
