#![cfg(feature = "python-sandbox")]

use apchat_toolcore::{Tool, ToolParameters, ToolContext, ToolRegistry};
use apchat_policy::PolicyManager;
use apchat_tools::PythonSandboxTool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

fn create_test_context_with_registry() -> ToolContext {
    let mut registry = ToolRegistry::new();
    // Register the sandbox tool itself to test self-exclusion
    registry.register(PythonSandboxTool);

    ToolContext::new(
        PathBuf::from("/tmp"),
        "test-session".to_string(),
        PolicyManager::default(),
    )
    .with_tool_registry(Arc::new(registry))
}

#[tokio::test]
async fn test_python_sandbox_tool_metadata() {
    let tool = PythonSandboxTool;

    assert_eq!(tool.name(), "python_sandbox");
    assert!(!tool.description().is_empty());
    assert!(tool.description().contains("sandbox"));

    let params = tool.parameters();
    assert!(params.contains_key("code"));
    assert!(params["code"].required);
    assert!(params.contains_key("list_functions"));
    assert!(!params["list_functions"].required);
}

#[tokio::test]
async fn test_python_sandbox_basic_execution() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "1 + 2");

    let result = tool.execute(params, &context).await;
    assert!(result.success, "Basic Python should succeed: {:?}", result.error);
    assert!(result.content.contains("3"), "Result should contain 3, got: {}", result.content);
}

#[tokio::test]
async fn test_python_sandbox_string_operations() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "'hello' + ' ' + 'world'");

    let result = tool.execute(params, &context).await;
    assert!(result.success, "String concat should succeed: {:?}", result.error);
    assert!(result.content.contains("hello world"), "Result should contain 'hello world', got: {}", result.content);
}

#[tokio::test]
async fn test_python_sandbox_print_output() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "print('hello from sandbox')");

    let result = tool.execute(params, &context).await;
    assert!(result.success, "Print should succeed: {:?}", result.error);
    assert!(result.content.contains("hello from sandbox"), "Output should contain printed text, got: {}", result.content);
}

#[tokio::test]
async fn test_python_sandbox_multiline() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "x = 10\ny = 20\nx + y");

    let result = tool.execute(params, &context).await;
    assert!(result.success, "Multiline should succeed: {:?}", result.error);
    assert!(result.content.contains("30"), "Result should contain 30, got: {}", result.content);
}

#[tokio::test]
async fn test_python_sandbox_syntax_error() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "def foo(");

    let result = tool.execute(params, &context).await;
    assert!(!result.success, "Syntax error should fail");
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_python_sandbox_runtime_error() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "1 / 0");

    let result = tool.execute(params, &context).await;
    assert!(!result.success, "Division by zero should fail");
    assert!(result.error.is_some());
}

#[tokio::test]
async fn test_python_sandbox_list_functions() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("list_functions", true);

    let result = tool.execute(params, &context).await;
    assert!(result.success, "list_functions should succeed: {:?}", result.error);
    assert!(result.content.contains("Available Python functions"), "Should list functions, got: {}", result.content);
}

#[tokio::test]
async fn test_python_sandbox_no_registry() {
    let tool = PythonSandboxTool;
    // Context without tool registry
    let context = ToolContext::new(
        PathBuf::from("/tmp"),
        "test-session".to_string(),
        PolicyManager::default(),
    );

    let mut params = ToolParameters::new();
    params.set("code", "1 + 1");

    let result = tool.execute(params, &context).await;
    assert!(!result.success, "Should fail without registry");
    assert!(result.error.unwrap().contains("registry"));
}

#[tokio::test]
async fn test_python_sandbox_missing_code_param() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let params = ToolParameters::new();

    let result = tool.execute(params, &context).await;
    assert!(!result.success, "Missing code should fail");
}

#[tokio::test]
async fn test_python_sandbox_stdlib_modules() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "import json\njson.dumps({'key': 'value'})");

    let result = tool.execute(params, &context).await;
    assert!(result.success, "Stdlib import should succeed: {:?}", result.error);
    assert!(result.content.contains("key"), "Should contain JSON output, got: {}", result.content);
}

#[tokio::test]
async fn test_python_sandbox_function_definition() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "def double(x):\n    return x * 2\ndouble(21)");

    let result = tool.execute(params, &context).await;
    assert!(result.success, "Function definition should succeed: {:?}", result.error);
    assert!(result.content.contains("42"), "Result should contain 42, got: {}", result.content);
}

#[tokio::test]
async fn test_python_sandbox_list_comprehension() {
    let tool = PythonSandboxTool;
    let context = create_test_context_with_registry();

    let mut params = ToolParameters::new();
    params.set("code", "[x**2 for x in range(5)]");

    let result = tool.execute(params, &context).await;
    assert!(result.success, "List comprehension should succeed: {:?}", result.error);
    assert!(result.content.contains("16"), "Result should contain 16, got: {}", result.content);
}

#[tokio::test]
async fn test_python_sandbox_calls_mock_tool() {
    use apchat_toolcore::ParameterDefinition;

    struct MockGreetTool;

    #[async_trait::async_trait]
    impl Tool for MockGreetTool {
        fn name(&self) -> &str { "greet" }
        fn description(&self) -> &str { "Returns a greeting for the given name" }
        fn parameters(&self) -> HashMap<String, ParameterDefinition> {
            HashMap::from([(
                "name".to_string(),
                ParameterDefinition {
                    param_type: "string".to_string(),
                    description: "Name to greet".to_string(),
                    required: true,
                    default: None,
                },
            )])
        }
        async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> apchat_toolcore::ToolResult {
            let name: String = params.get_required("name").unwrap_or_default();
            apchat_toolcore::ToolResult::success(format!("Hello, {name}!"))
        }
    }

    let mut registry = ToolRegistry::new();
    registry.register(PythonSandboxTool);
    registry.register(MockGreetTool);

    let context = ToolContext::new(
        PathBuf::from("/tmp"),
        "test-session".to_string(),
        PolicyManager::default(),
    )
    .with_tool_registry(Arc::new(registry));

    let tool = PythonSandboxTool;
    let mut params = ToolParameters::new();
    params.set("code", "result = greet(name='World')\nresult");

    let result = tool.execute(params, &context).await;
    assert!(result.success, "Mock tool call should succeed: {:?}", result.error);
    assert!(
        result.content.contains("Hello, World!"),
        "Should contain greeting, got: {}",
        result.content
    );
}
