# LLM Call Tool

## Description

The `llm_oneshot` tool allows models to make one-shot calls to LLM models without starting a full agent session. This is useful for simple tasks that don't require complex tool usage or conversation history.

## Parameters

- `model_color` (required, string): The model color to call - "red", "grn", or "blu"
- `instruction` (required, string): The instruction or prompt to send to the LLM
- `file_path` (optional, string): Path to a file whose contents will be appended to the instruction

## Examples

### Simple instruction without file

```xml
<tool_call name="llm_oneshot">
  <parameter name="model_color">grn</parameter>
  <parameter name="instruction">Explain the concept of recursion in programming</parameter>
</tool_call>
```

### Instruction with file contents

```xml
<tool_call name="llm_oneshot">
  <parameter name="model_color">blu</parameter>
  <parameter name="instruction">Analyze this code and suggest improvements</parameter>
  <parameter name="file_path">src/main.rs</parameter>
</tool_call>
```

## Use Cases

- **Code analysis and review**: Quick analysis of code snippets or file contents
- **Documentation generation**: Creating documentation from code comments
- **Simple Q&A**: Answering questions without agent overhead
- **Format conversion**: Converting text formats (markdown to HTML, etc.)
- **Lightweight text processing**: Text summarization, editing, or transformation
- **Code generation**: Generating simple code snippets or functions

## Error Handling

The tool handles the following error scenarios:

1. **Invalid model color**: Returns error if `model_color` is not "red", "grn", or "blu"
2. **Missing required parameters**: Returns error if `model_color` or `instruction` are not provided
3. **File reading errors**: Returns error if `file_path` is provided but cannot be read
4. **LLM client unavailable**: Returns error if no LLM client is configured for the specified model color
5. **LLM API failures**: Returns error if the LLM API call fails

## Best Practices

- Use for simple, self-contained tasks that don't require tool chaining
- For complex tasks requiring multiple tools or steps, use a full agent session instead
- When using `file_path`, ensure the file exists and is readable before calling the tool
- Choose the appropriate model color based on task complexity (grn for cost efficiency, blu for speed, red for specialized tasks)

## Integration with Other Tools

The `llm_oneshot` tool can be combined with file operations tools:

```xml
<tool_call name="open_file">
  <parameter name="file_path">src/lib.rs</parameter>
</tool_call>

<tool_call name="llm_oneshot">
  <parameter name="model_color">grn</parameter>
  <parameter name="instruction">Review this Rust code for best practices</parameter>
  <parameter name="file_path">src/lib.rs</parameter>
</tool_call>
```
