CREATE TABLE system_metrics (
    host STRING,
    memory_util DOUBLE,
    ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP(),
    PRIMARY KEY(host),
    TIME INDEX(ts)
);

INSERT INTO system_metrics
VALUES
    ("host1", 10.3, "2024-10-10T13:59:00Z"),
    ("host2", 90.0, "2024-12-10T13:59:00Z"),
    ("host2", 90.0, "2024-12-31T13:59:00Z"),
    ("host1", 10.3, "2025-01-01T00:00:00Z"),
    ("host2", 90.0, "2025-01-03T13:59:00Z"),
    ("host1", 10.3, "2025-01-13T18:59:00Z"),
    ("host2", 90.0, "2025-02-01T00:00:00Z"),
    ("host2", 90.0, "2025-02-13T12:11:06Z"),
    ("host1", 40.6, "2025-11-01T00:00:00Z"),
    ("host1", 10.3, "2025-11-30T23:59:59Z"),
    ("host2", 90.0, "2025-12-01T00:00:00Z"),
    ("host2", 90.0, "2025-12-01T01:00:00Z"),
    ("host2", 90.0, "2025-12-01T01:59:59Z"),
    ("host2", 90.0, "2025-12-01T01:00:00Z"),
    ("host2", 90.0, "2025-12-01T01:00:00Z"),
    ("host2", 90.0, "2025-12-01T01:00:59Z"),
    ("host2", 90.0, "2025-12-01T01:01:00Z"),
    ("host1", 10.3, "2025-12-31T01:11:59Z"),
    ("host1", 10.3, "2025-12-31T01:12:00Z"),
    ("host2", 90.0, "2025-12-31T01:12:00Z"),
    ("host1", 10.3, "2025-12-31T01:12:59Z"),
    ("host1", 10.3, "2025-12-31T01:13:00Z"),
    ("host2", 90.0, "2025-12-01T02:00:00Z"),
    ("host2", 90.0, "2025-12-01T23:59:59Z"),
    ("host2", 90.0, "2025-12-02T00:00:00Z"),
    ("host2", 90.0, "2025-12-15T13:42:12Z"),
    ("host1", 40.6, "2025-12-31T23:59:00Z"),
    ("host1", 40.6, "2025-12-31T23:59:59Z"),
    ("host1", 10.3, "2026-01-01T00:00:00Z"),
    ("host2", 90.0, "2026-01-01T23:59:59Z"),
    ("host2", 90.0, "2026-12-31T13:59:00Z");

SELECT * FROM system_metrics WHERE ts WITHIN '2023' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2024' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2026' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2027' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-01' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-05' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12-01' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12-24' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12-01T01' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12-01T15' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12-01T01:00' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12-01T15:00' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12-31T01:12:59' ORDER BY host, ts;

SELECT * FROM system_metrics WHERE ts WITHIN '2025-12-31T01:12:55' ORDER BY host, ts;

-- SQLNESS REPLACE (metrics.*) REDACTED
-- SQLNESS REPLACE (-+) -
-- SQLNESS REPLACE (\s\s+) _
-- SQLNESS REPLACE (peers.*) REDACTED
-- SQLNESS REPLACE (RoundRobinBatch.*) REDACTED
-- SQLNESS REPLACE region=\d+\(\d+,\s+\d+\) region=REDACTED
EXPLAIN ANALYZE SELECT * FROM system_metrics WHERE ts WITHIN '2025';

DROP TABLE system_metrics;
