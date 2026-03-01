use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;

#[cfg(feature = "python-sandbox")]
use ouros::{ReplSession, ReplProgress, Object, CollectStringPrint, ExternalResult, ExcType, Exception};

/// Tool that provides a sandboxed Python execution environment via ouros.
///
/// All registered tools from the agent's tool registry are automatically
/// exposed as Python functions. When Python code calls a tool function,
/// execution pauses, the tool is invoked via the registry, and the result
/// is returned to the Python environment.
pub struct PythonSandboxTool;

#[cfg(feature = "python-sandbox")]
fn object_to_json(obj: &Object) -> serde_json::Value {
    match obj {
        Object::String(s) => serde_json::Value::String(s.clone()),
        Object::Int(i) => serde_json::json!(i),
        Object::Float(f) => serde_json::json!(f),
        Object::Bool(b) => serde_json::Value::Bool(*b),
        Object::None => serde_json::Value::Null,
        other => serde_json::Value::String(format!("{other:?}")),
    }
}

#[cfg(feature = "python-sandbox")]
fn generate_tool_help(registry: &apchat_toolcore::ToolRegistry) -> String {
    let mut help = String::from("Available Python functions (backed by agent tools):\n\n");
    let mut tool_names: Vec<_> = registry.get_tool_names();
    tool_names.sort();
    for name in &tool_names {
        if name == "python_sandbox" {
            continue;
        }
        if let Some(tool) = registry.get_tool(name) {
            let params = tool.parameters();
            let mut param_parts: Vec<String> = Vec::new();
            let mut sorted_params: Vec<_> = params.iter().collect();
            sorted_params.sort_by_key(|(k, _)| k.clone());
            for (pname, pdef) in &sorted_params {
                if pdef.required {
                    param_parts.push(format!("{pname}: {}", pdef.param_type));
                } else {
                    param_parts.push(format!("{pname}: {} = ...", pdef.param_type));
                }
            }
            let sig = param_parts.join(", ");
            help.push_str(&format!("  {name}({sig})\n"));
            help.push_str(&format!("    {}\n\n", tool.description()));
        }
    }
    help
}

#[async_trait]
impl Tool for PythonSandboxTool {
    fn name(&self) -> &str {
        "python_sandbox"
    }

    fn description(&self) -> &str {
        "Execute Python code in a sandboxed environment. All agent tools are available as Python functions \
         that can be called directly (e.g., `result = read_file(file_path='README.md')`). \
         The sandbox has no filesystem, network, or subprocess access — tool functions are the only \
         way to interact with the outside world. Uses ouros (https://github.com/parcadei/ouros)."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("code", "string", "Python code to execute in the sandbox. Agent tools are available as functions.", required),
            param!("list_functions", "boolean", "If true, returns a list of available Python functions instead of executing code.", optional, false),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        #[cfg(feature = "python-sandbox")]
        {
            self.execute_sandbox(params, context).await
        }

        #[cfg(not(feature = "python-sandbox"))]
        {
            let _ = (params, context);
            ToolResult::error(
                "Python sandbox is not available. Build with `cargo build --features python-sandbox`".to_string(),
            )
        }
    }
}

#[cfg(feature = "python-sandbox")]
impl PythonSandboxTool {
    async fn execute_sandbox(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let registry = match &context.tool_registry {
            Some(r) => r.clone(),
            None => return ToolResult::error("Tool registry not available in context".to_string()),
        };

        let list_functions: bool = params.get_optional("list_functions").unwrap_or(None).unwrap_or(false);
        if list_functions {
            return ToolResult::success(generate_tool_help(&registry));
        }

        let code = match params.get_required::<String>("code") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Collect tool names as external functions, excluding ourselves
        let external_functions: Vec<String> = registry
            .get_tool_names()
            .into_iter()
            .filter(|name| name != "python_sandbox")
            .collect();

        // Run the ouros session in a blocking task since it's synchronous
        let context_clone = context.clone();
        let result = tokio::task::spawn_blocking(move || {
            run_sandbox_session(&code, external_functions, &registry, &context_clone)
        })
        .await;

        match result {
            Ok(tool_result) => tool_result,
            Err(e) => ToolResult::error(format!("Sandbox execution panicked: {e}")),
        }
    }
}

#[cfg(feature = "python-sandbox")]
fn run_sandbox_session(
    code: &str,
    external_functions: Vec<String>,
    registry: &apchat_toolcore::ToolRegistry,
    context: &ToolContext,
) -> ToolResult {
    let mut session = ReplSession::new(external_functions, "sandbox.py");
    let mut print_output = CollectStringPrint::new();

    let mut progress = match session.execute_interactive(code, &mut print_output) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("Python error: {e}")),
    };

    // Iteration limit to prevent infinite loops
    let max_iterations = 1000;
    let mut iterations = 0;

    loop {
        iterations += 1;
        if iterations > max_iterations {
            return ToolResult::error(format!(
                "Sandbox exceeded maximum of {max_iterations} external function calls"
            ));
        }

        match progress {
            ReplProgress::Complete(obj) => {
                let printed = print_output.output();
                let result_str = format!("{obj:?}");
                let mut output = String::new();
                if !printed.is_empty() {
                    output.push_str(&format!("[stdout]\n{printed}\n"));
                }
                if result_str != "None" {
                    output.push_str(&format!("[result]\n{result_str}"));
                } else if output.is_empty() {
                    output.push_str("[result]\nNone");
                }
                return ToolResult::success(output.trim().to_string());
            }
            ReplProgress::FunctionCall {
                function_name,
                args,
                kwargs,
                call_id: _,
            } => {
                let tool_result = execute_tool_sync(
                    &function_name,
                    &args,
                    &kwargs,
                    registry,
                    context,
                );

                let resume_value: ExternalResult = match tool_result {
                    Ok(content) => Object::String(content).into(),
                    Err(err_msg) => {
                        Exception::new(ExcType::RuntimeError, Some(err_msg)).into()
                    }
                };

                progress = match session.resume(resume_value, &mut print_output) {
                    Ok(p) => p,
                    Err(e) => return ToolResult::error(format!("Python error after resuming: {e}")),
                };
            }
            ReplProgress::ProxyCall { .. } => {
                return ToolResult::error("Proxy calls are not supported in the sandbox".to_string());
            }
            ReplProgress::ResolveFutures { .. } => {
                return ToolResult::error(
                    "Async futures resolution is not supported in the sandbox".to_string(),
                );
            }
        }
    }
}

#[cfg(feature = "python-sandbox")]
fn execute_tool_sync(
    function_name: &str,
    args: &[Object],
    kwargs: &[(Object, Object)],
    registry: &apchat_toolcore::ToolRegistry,
    context: &ToolContext,
) -> Result<String, String> {
    // Build ToolParameters from kwargs (named arguments) and positional args
    let mut params = ToolParameters::new();

    // Get the tool to understand its parameter names
    let tool = registry
        .get_tool(function_name)
        .ok_or_else(|| format!("Tool '{function_name}' not found"))?;

    let param_defs = tool.parameters();
    let mut sorted_params: Vec<_> = param_defs.iter().collect();
    sorted_params.sort_by_key(|(k, _)| k.clone());

    // Map positional args to parameter names (by sorted order)
    for (i, arg) in args.iter().enumerate() {
        if let Some((pname, _)) = sorted_params.get(i) {
            params.data.insert((*pname).clone(), object_to_json(arg));
        }
    }

    // Map kwargs to parameter names
    for (key, value) in kwargs {
        if let Object::String(k) = key {
            params.data.insert(k.clone(), object_to_json(value));
        }
    }

    // Execute the tool synchronously using a new tokio runtime
    // We're already inside spawn_blocking, so we need a new runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {e}"))?;

    let result = rt.block_on(registry.execute_tool(function_name, params, context));

    if result.success {
        Ok(result.content)
    } else {
        Err(result.error.unwrap_or_else(|| "Unknown tool error".to_string()))
    }
}
