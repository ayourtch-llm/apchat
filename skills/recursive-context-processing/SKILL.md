---
name: recursive-context-processing
description: Process inputs that exceed context window limits using Recursive Language Model (RLM) patterns. Use when handling large files, codebases, log collections, or any input too large to fit in a single LLM call. Based on the RLM paper (arXiv:2512.24601).
---

# Recursive Context Processing (RLM Pattern)

Process arbitrarily large inputs by treating them as external variables on disk, explored programmatically through code execution and recursive sub-agent delegation.

Based on [Recursive Language Models](https://arxiv.org/abs/2512.24601) (Zhang, Kraska & Khattab, 2026).

## When to Use

- Input exceeds ~50KB of text (roughly half a typical context window)
- Processing a full codebase, large log file, long document, or multi-file dataset
- A previous attempt failed due to context limits or produced incomplete results
- The task requires examining ALL of a large input (not just a sample)

## Core Principles

| Principle | Standard Approach | RLM Approach |
|-----------|------------------|--------------|
| **Prompt location** | Load everything into context | Keep on disk as a file variable |
| **Exploration** | Read and reason in one pass | Write code to probe, filter, search |
| **Recursion** | None — single forward pass | Delegate chunks to sub-agents |
| **Output** | Return in conversation | Write results to files |

## The RLM Pipeline

Follow these phases in order. Each phase reduces the work for the next.

### Phase 1: PROBE — Understand the Input

Before any processing, characterize the input with code:

```
run_command: wc -l <file>                    # How many lines?
run_command: wc -c <file>                    # How many bytes?
run_command: head -50 <file>                 # What does it look like?
run_command: file <file>                     # What format?
run_command: find <dir> -type f | wc -l      # How many files?
run_command: find <dir> -name '*.rs' | head  # What structure?
```

Record: total size, format, structure, estimated chunk count.

### Phase 2: FILTER — Reduce Before Processing

Use code to eliminate irrelevant content BEFORE any LLM processing:

```
run_command: grep -r "pattern" <dir> --include="*.rs" -l   # Find relevant files
run_command: grep -v "^#" <file> | grep -v "^$"             # Remove comments/blanks
run_command: awk '/START/,/END/' <file>                      # Extract sections
```

Target: reduce input by 50-80% through deterministic filtering. Code is cheaper than LLM tokens.

### Phase 3: CHUNK — Split into Manageable Pieces

Use the `rlm_context_chunk` tool to split the input:

```
rlm_context_chunk:
  input_path: "./large_file.txt"
  chunk_strategy: "lines"        # or "chars", "separator", "files"
  chunk_size: 500                # lines per chunk (or chars, depending on strategy)
  output_dir: "/tmp/rlm_chunks"  # where to write chunks
```

The tool returns a manifest with chunk paths and metadata. Each chunk is a self-contained file on disk.

**Chunk strategy selection:**
- `lines` — best for logs, CSVs, line-oriented data (chunk_size = number of lines)
- `chars` — best for prose, documents (chunk_size = number of characters, default 60000)
- `separator` — best for structured data with known delimiters (set `separator` parameter)
- `files` — best for directories/codebases (groups files up to chunk_size total characters)

### Phase 4: DELEGATE — Process Chunks via Sub-Agents

For each chunk, launch a sub-agent with a focused task:

```
launch_subagent:
  task: "Read /tmp/rlm_chunks/chunk_001.txt and <specific instruction>.
        Write your result as JSON to /tmp/rlm_results/result_001.json"
```

Key rules for delegation:
- **One chunk per sub-agent** — keep tasks focused
- **Results go to FILES** — never ask sub-agents to return large content in conversation
- **Structured output** — request JSON so aggregation is deterministic
- **Include context** — tell the sub-agent what the overall task is, not just "process this chunk"
- **Parallel when possible** — chunks are independent, launch multiple sub-agents

### Phase 5: AGGREGATE — Combine Results with Code

Use code to combine sub-agent results — don't load everything back into context:

```
run_command: cat /tmp/rlm_results/*.json | python3 -c "
import json, sys
results = [json.loads(line) for line in sys.stdin if line.strip()]
# Combine, deduplicate, summarize...
combined = {'items': [r for result in results for r in result.get('items', [])]}
print(json.dumps(combined, indent=2))
" > /tmp/rlm_final/aggregated.json
```

### Phase 6: FINAL ANSWER — Synthesize from Aggregated Data

Read the aggregated result file and produce the final answer. At this point the data should be small enough to reason about directly.

## Strategy Patterns

| Pattern | When to Use | Example |
|---------|------------|---------|
| **Filter-Then-Process** | Looking for specific items | "Find the API key leak in this repo" |
| **Map-Reduce** | Process everything uniformly | "Classify every log entry by severity" |
| **Hierarchical** | Very large inputs (1M+ tokens) | "Summarize this 10M-line codebase" |
| **Iterative Refinement** | Need precision | "Find all security issues, then rank by severity" |

### Map-Reduce Pattern

1. Chunk the input
2. Map: each sub-agent processes one chunk → writes partial result
3. Reduce: code merges partial results → final answer

### Hierarchical Pattern (for very large inputs)

1. First pass: chunk into large segments, sub-agents produce summaries
2. Second pass: chunk summaries, sub-agents analyze further
3. Continue until result fits in context

## RLM-Lite (No Sub-Agents)

For simpler tasks, skip Phase 4 (delegation) entirely. Use only code execution to process chunks:

```
run_command: for f in /tmp/rlm_chunks/*.txt; do
  grep -c "ERROR" "$f" >> /tmp/rlm_results/counts.txt
done
```

RLM-lite is sufficient for: keyword counting, pattern extraction, structural analysis, data filtering.

## Cost Control

- **Filter aggressively** — eliminate 80%+ of input before any LLM processing
- **Code first** — use deterministic code for counting, sorting, formatting, filtering
- **Start with RLM-lite** — escalate to full RLM (with sub-agents) only when semantic understanding is required
- **Limit depth** — avoid recursive sub-agent chains deeper than 2-3 levels

## Anti-Patterns

- ❌ Loading a 500KB file into a `read_file` call and hoping for the best
- ❌ Asking a sub-agent to "return all the data" (results should go to files)
- ❌ Chunking without filtering first (wastes sub-agent calls on irrelevant data)
- ❌ Using LLM calls for work that code can do deterministically
- ❌ Skipping the probe phase — you need to know the input's size and structure first

## Mapping to APChat Tools

| RLM Concept | APChat Tool |
|-------------|-------------|
| REPL environment | `run_command` |
| Context variable | Files on disk (managed by `read_file`, `write_file`) |
| Chunking | `rlm_context_chunk` tool |
| `lm_query()` sub-calls | `launch_subagent` |
| File exploration | `list_files`, `search_files` |
| Result aggregation | `run_command` with shell/Python scripts |
| FINAL_ANSWER | `write_file` or direct conversation response |
