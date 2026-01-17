# Curly Glance Fix Plan

## Overview
The current `file_curly_glance` implementation needs significant refactoring to meet the specified requirements. This plan outlines a systematic approach to fix the implementation.

## Current Issues
1. **Output format**: Multi-line and verbose (needs to be single-line)
2. **Bracket depth tracking**: Doesn't distinguish top-level vs nested brackets
3. **Backward search**: Missing functionality to find preceding empty/whitespace-only line
4. **starting_line handling**: Incorrect implementation
5. **Range calculation**: Missing start..end line number calculation

## Requirements Analysis

### Core Functionality
1. Find only top-level curly brackets (depth = 0)
2. Match opening { with closing }
3. Search backwards from opening bracket to find preceding empty/whitespace-only line
4. Calculate and display line range (start..end)
5. Display function name or context near the opening bracket

### Optional Features
- starting_line parameter: Start scanning from that line and stop at first unmatched closing bracket

## Implementation Plan

### Phase 1: Data Structures
- Create `BracketPair` struct to store:
  - opening line number
  - closing line number
  - content/description (e.g., function name)
  - preceding whitespace line number (if found)

### Phase 2: Core Algorithm
1. **Initialize**: depth = 0, starting_line = 1 (or provided value)
2. **Scan lines** from starting_line:
   - Track bracket depth
   - When opening { found at depth 0:
     - Record opening line
     - Find preceding empty/whitespace-only line
     - Continue scanning until matching } at depth 0
     - Record closing line
     - Extract context (function name or nearby text)
     - Store as BracketPair
3. **Termination**: If unmatched } encountered when depth = 0, stop scanning

### Phase 3: Output Formatting
- Format as: "start..end: context"
- Example: "10..20: fn main() { ... }"

### Phase 4: Error Handling
- Handle edge cases:
  - Unmatched opening brackets
  - Unmatched closing brackets
  - Empty files
  - Files with no brackets

### Phase 5: Testing Strategy
1. **Unit tests**:
   - Top-level bracket detection
   - Nested bracket ignoring
   - Preceding whitespace line finding
   - starting_line parameter
   - Range calculation
2. **Integration tests**:
   - Complex files with multiple functions
   - Files with unmatched brackets
   - Empty files

## File Structure
- `src/file_curly_glance.rs`: Main implementation
- `tests/`: Unit and integration tests
- `examples/`: Example usage

## Timeline
- Phase 1-2: 2 days
- Phase 3-4: 1 day
- Phase 5: 2 days

## Success Criteria
1. All requirements met
2. All tests passing
3. Code coverage > 90%
4. Documentation complete

## Risks and Mitigations
1. **Complex parsing logic**: Use existing parser libraries where possible
2. **Edge cases**: Comprehensive test coverage
3. **Performance**: Optimize line-by-line processing

## Next Steps
1. Review existing implementation
2. Create initial data structures
3. Implement core algorithm
4. Add output formatting
5. Write tests
6. Iterate based on test results
