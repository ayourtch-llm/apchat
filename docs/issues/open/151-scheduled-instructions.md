# Issue 151: Scheduled Instructions Feature

## Summary
Implement a scheduled instructions feature that allows users to schedule instructions (messages) to be injected into the input channel at a specified future time. This enables users to plan ahead and have messages automatically sent to the AI at specific dates/times.

## Location
- Database: `crates/apchat-tools/src/memory/db.rs` (extend schema)
- Tools: `crates/apchat-tools/src/memory/tools.rs` (add 3 new tools)
- Poller: `apchat-main/src/scheduled_instructions/poller.rs` (new file)
- Integration: `apchat-main/src/app/repl.rs` (start poller in REPL mode)

## Current Behavior
No scheduled instructions feature exists. Users must manually type or paste messages at the prompt.

## Expected Behavior
1. User can schedule an instruction with a future date/time using a tool
2. A background poller checks for due instructions periodically
3. When an instruction's scheduled time arrives, it's injected as regular user input
4. User can list all scheduled instructions to see what's pending
5. User can delete scheduled instructions before they are processed

## Implementation Details

### Database Schema
Add `scheduled_instructions` table to the memory database:
```sql
CREATE TABLE IF NOT EXISTS scheduled_instructions (
    id TEXT PRIMARY KEY,
    scheduled_time INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    processed_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_scheduled_instructions_status ON scheduled_instructions(status);
CREATE INDEX IF NOT EXISTS idx_scheduled_instructions_scheduled_time ON scheduled_instructions(scheduled_time);
CREATE INDEX IF NOT EXISTS idx_scheduled_instructions_status_time ON scheduled_instructions(status, scheduled_time);
```

### Tools (in `crates/apchat-tools/src/memory/tools.rs`)

1. **AddScheduledInstructionTool**
   - Parameters: `scheduled_time` (Unix timestamp), `content` (string)
   - Returns: `{"message": "Scheduled instruction created", "id": "uuid"}`
   - Creates a new scheduled instruction with status "pending"

2. **ListScheduledInstructionsTool**
   - Parameters: `status` (optional filter: "pending" or "processed")
   - Returns: List of scheduled instructions with id, scheduled_time, content, status
   - Lists all pending/processed instructions

3. **DeleteScheduledInstructionTool**
   - Parameters: `id` (UUID string)
   - Returns: `{"message": "Scheduled instruction deleted"}`
   - Deletes a scheduled instruction by ID

### Background Poller (in `apchat-main/src/scheduled_instructions/poller.rs`)

1. Create a poller struct that holds:
   - Database pool reference
   - MSPC channel reference
   - Stop signal (cancellation token)

2. Run a loop every 30 seconds that:
   - Queries for pending instructions where `scheduled_time <= now`
   - For each due instruction:
     - Inject as `MspcMessage::UserInput(content, Some("scheduled".to_string()))` into MSPC channel
     - Mark as processed by updating `status = 'processed'` and `processed_at = now`
   - Handle errors gracefully

3. Integration with REPL mode:
   - Start poller when REPL mode initializes
   - Stop poller when REPL mode exits

## Testing Strategy

**IMPORTANT: Use test database path to avoid corrupting production data!**
- Set `APCHAT_MEMORY_DB_PATH` environment variable to a test database file
- After tests, delete the test database file to clean up
- Each test should have its own isolated database

### Test Cases
1. **AddScheduledInstructionTool**
   - Test adding a scheduled instruction with future timestamp
   - Verify instruction exists in database with correct fields
   - Verify ID is a valid UUID

2. **ListScheduledInstructionsTool**
   - Test listing all pending instructions
   - Test listing all processed instructions
   - Test listing with no instructions

3. **DeleteScheduledInstructionTool**
   - Test deleting an existing instruction
   - Test deleting a non-existent instruction
   - Verify instruction is removed from database

4. **Background Poller**
   - Test poller finds and processes due instructions
   - Test poller skips instructions scheduled in the future
   - Test poller marks instructions as processed after injection
   - Test multiple instructions processed in one poll cycle

5. **Integration**
   - Test poller started in REPL mode
   - Test poller stopped when REPL mode exits

## Impact
This feature enables users to plan ahead and have messages automatically sent to the AI at specific times. Useful for:
- Scheduling follow-up questions
- Setting reminders to ask about specific topics
- Automating periodic queries to the AI

---
*Created: 2026-02-04*
*Status: Open - Implementation in progress*
