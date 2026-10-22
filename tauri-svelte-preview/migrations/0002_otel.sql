-- OpenTelemetry received from agents, kept for reports. The GenAI fields are
-- columns; everything else the record carried, after redaction, is the JSON
-- `attributes` column. Rows older than 90 days are deleted at startup.

CREATE TABLE IF NOT EXISTS otel_spans (
    id INTEGER PRIMARY KEY,
    time_ms INTEGER NOT NULL,
    end_ms INTEGER,
    trace_id TEXT,
    span_id TEXT,
    parent_span_id TEXT,
    name TEXT NOT NULL,
    service TEXT,
    session_id TEXT,
    owned_id TEXT,
    run_id TEXT,
    node_run_id TEXT,
    model TEXT,
    operation TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cache_read_tokens INTEGER,
    cache_write_tokens INTEGER,
    cost_micros INTEGER,
    duration_ms INTEGER,
    ttft_ms INTEGER,
    request_id TEXT,
    tool_name TEXT,
    status TEXT,
    attributes TEXT NOT NULL CHECK (json_valid(attributes))
);
CREATE INDEX IF NOT EXISTS otel_spans_time_idx ON otel_spans (time_ms);
CREATE INDEX IF NOT EXISTS otel_spans_owner_idx ON otel_spans (owned_id, time_ms);
CREATE INDEX IF NOT EXISTS otel_spans_step_idx ON otel_spans (node_run_id);
CREATE INDEX IF NOT EXISTS otel_spans_parent_idx ON otel_spans (trace_id, parent_span_id);

CREATE TABLE IF NOT EXISTS otel_log_events (
    id INTEGER PRIMARY KEY,
    time_ms INTEGER NOT NULL,
    trace_id TEXT,
    span_id TEXT,
    name TEXT NOT NULL,
    service TEXT,
    session_id TEXT,
    owned_id TEXT,
    run_id TEXT,
    node_run_id TEXT,
    model TEXT,
    operation TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cache_read_tokens INTEGER,
    cache_write_tokens INTEGER,
    cost_micros INTEGER,
    duration_ms INTEGER,
    ttft_ms INTEGER,
    request_id TEXT,
    tool_name TEXT,
    status TEXT,
    attributes TEXT NOT NULL CHECK (json_valid(attributes))
);
CREATE INDEX IF NOT EXISTS otel_log_events_time_idx ON otel_log_events (time_ms);
CREATE INDEX IF NOT EXISTS otel_log_events_owner_idx ON otel_log_events (owned_id, time_ms);
CREATE INDEX IF NOT EXISTS otel_log_events_step_idx ON otel_log_events (node_run_id);

CREATE TABLE IF NOT EXISTS otel_metric_points (
    id INTEGER PRIMARY KEY,
    time_ms INTEGER NOT NULL,
    name TEXT NOT NULL,
    unit TEXT,
    value REAL,
    service TEXT,
    session_id TEXT,
    owned_id TEXT,
    run_id TEXT,
    node_run_id TEXT,
    model TEXT,
    attributes TEXT NOT NULL CHECK (json_valid(attributes))
);
CREATE INDEX IF NOT EXISTS otel_metric_points_time_idx ON otel_metric_points (time_ms);
CREATE INDEX IF NOT EXISTS otel_metric_points_owner_idx ON otel_metric_points (owned_id, time_ms);
