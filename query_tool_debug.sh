#!/usr/bin/env bash
# Query the tool debug database

DB_PATH="${1:-/tmp/tool_debug.db}"

if [ ! -f "$DB_PATH" ]; then
    echo "Database not found at: $DB_PATH"
    echo "Make sure apchat is running with SQL logging enabled"
    exit 1
fi

echo "Database: $DB_PATH"
echo "=========================================="

# Show tool parsing failures
echo -e "\n🔴 TOOL PARSING FAILURES (last 10):"
echo "=========================================="
sqlite3 -header -column "$DB_PATH" "
SELECT 
    id,
    timestamp,
    tool_name,
    parse_error,
    raw_llm_output
FROM tool_parse_logs 
WHERE parse_success = 0 
ORDER BY timestamp DESC 
LIMIT 10;
"

# Show tool parsing successes
echo -e "\n🟢 TOOL PARSING SUCCESS (last 10):"
echo "=========================================="
sqlite3 -header -column "$DB_PATH" "
SELECT 
    id,
    timestamp,
    tool_name,
    parsed_arguments
FROM tool_parse_logs 
WHERE parse_success = 1 
ORDER BY timestamp DESC 
LIMIT 10;
"

# Show HTTP call errors
echo -e "\n❌ HTTP CALL ERRORS (last 10):"
echo "=========================================="
sqlite3 -header -column "$DB_PATH" "
SELECT 
    id,
    timestamp,
    provider,
    model,
    endpoint,
    status_code,
    latency_ms,
    error
FROM calls 
WHERE status_code >= 400 OR error IS NOT NULL
ORDER BY timestamp DESC 
LIMIT 10;
"

# Show recent HTTP calls
echo -e "\n📡 RECENT HTTP CALLS (last 10):"
echo "=========================================="
sqlite3 -header -column "$DB_PATH" "
SELECT 
    id,
    timestamp,
    provider,
    model,
    endpoint,
    status_code,
    latency_ms,
    ttft_ms,
    input_tokens,
    output_tokens,
    cost_usd
FROM calls 
ORDER BY timestamp DESC 
LIMIT 10;
"

# Show statistics
echo -e "\n📊 TOOL PARSING STATISTICS:"
echo "=========================================="
sqlite3 "$DB_PATH" "
SELECT 
    'Total logs: ' || COUNT(*) as stat,
    'Successes: ' || SUM(CASE WHEN parse_success = 1 THEN 1 ELSE 0 END) as success,
    'Failures: ' || SUM(CASE WHEN parse_success = 0 THEN 1 ELSE 0 END) as failures
FROM tool_parse_logs;
"

echo -e "\n📊 HTTP CALL STATISTICS:"
echo "=========================================="
sqlite3 -header -column "$DB_PATH" "
SELECT 
    COUNT(*) as total_calls,
    COALESCE(SUM(input_tokens), 0) as total_input_tokens,
    COALESCE(SUM(output_tokens), 0) as total_output_tokens,
    COALESCE(SUM(cost_usd), 0.0) as total_cost_usd,
    COALESCE(AVG(latency_ms), 0.0) as avg_latency_ms,
    COALESCE(SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END), 0) as error_count
FROM calls;
"

# Show unique tools with failures
echo -e "\n🔧 TOOLS WITH FAILURES:"
echo "=========================================="
sqlite3 -header -column "$DB_PATH" "
SELECT 
    tool_name,
    COUNT(*) as failure_count
FROM tool_parse_logs 
WHERE parse_success = 0 AND tool_name IS NOT NULL
GROUP BY tool_name 
ORDER BY failure_count DESC;
"

# Show HTTP calls by model
echo -e "\n🤖 HTTP CALLS BY MODEL:"
echo "=========================================="
sqlite3 -header -column "$DB_PATH" "
SELECT 
    model,
    COUNT(*) as calls,
    COALESCE(SUM(cost_usd), 0.0) as total_cost,
    COALESCE(AVG(latency_ms), 0.0) as avg_latency_ms
FROM calls 
GROUP BY model 
ORDER BY calls DESC
LIMIT 10;
"

# Show unique error messages
echo -e "\n⚠️  UNIQUE TOOL ERROR MESSAGES:"
echo "=========================================="
sqlite3 -header -column "$DB_PATH" "
SELECT 
    parse_error,
    COUNT(*) as count
FROM tool_parse_logs 
WHERE parse_success = 0 AND parse_error IS NOT NULL
GROUP BY parse_error 
ORDER BY count DESC
LIMIT 10;
"

echo -e "\n💡 TIPS:"
echo "=========================================="
echo "  View full details: sqlite3 $DB_PATH"
echo "  Clear all logs:     sqlite3 $DB_PATH 'DELETE FROM tool_parse_logs; DELETE FROM calls;'"
echo "  Search by tool:     sqlite3 $DB_PATH \"SELECT * FROM tool_parse_logs WHERE tool_name='read_file'\""
echo "  Search by model:    sqlite3 $DB_PATH \"SELECT * FROM calls WHERE model LIKE '%claude%'\""
echo "  OpenTrace compat:   The 'calls' table is compatible with https://github.com/jmamda/OpenTrace"