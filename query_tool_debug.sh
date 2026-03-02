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

# Show recent failures
echo -e "\n🔴 RECENT FAILURES (last 10):"
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

# Show recent successes
echo -e "\n🟢 RECENT SUCCESSS (last 10):"
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

# Show statistics
echo -e "\n📊 STATISTICS:"
echo "=========================================="
sqlite3 "$DB_PATH" "
SELECT 
    'Total logs: ' || COUNT(*) as stat,
    'Successes: ' || SUM(CASE WHEN parse_success = 1 THEN 1 ELSE 0 END) as success,
    'Failures: ' || SUM(CASE WHEN parse_success = 0 THEN 1 ELSE 0 END) as failures
FROM tool_parse_logs;
"

# Show unique tools
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

# Show unique error messages
echo -e "\n⚠️  UNIQUE ERROR MESSAGES:"
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
echo "  Clear all logs:     sqlite3 $DB_PATH 'DELETE FROM tool_parse_logs'"
echo "  Search by tool:     sqlite3 $DB_PATH \"SELECT * FROM tool_parse_logs WHERE tool_name='read_file'\""