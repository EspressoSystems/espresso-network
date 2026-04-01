CREATE TABLE event_stream (
	id                  INTEGER PRIMARY KEY AUTOINCREMENT,
	last_processed_view BIGINT
);
