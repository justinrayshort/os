# Code Patterns & Idioms

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Audience:** Code authors, reviewers, code generators

Establishes idiomatic Rust patterns, error handling, testing conventions, and rustdoc standards used throughout this repository.

## Error Handling

### Typed Errors (MANDATORY)

**Rule:** All fallible operations must return `Result<T, E>` with a custom error type.

```rust
// ❌ WRONG: Using String for errors
fn execute_command(cmd: &str) -> Result<Output, String> { ... }

// ✅ CORRECT: Custom error type
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("command not found: {0}")]
    NotFound(String),
    
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    
    #[error("execution failed: {0}")]
    ExecutionFailed(#[from] std::io::Error),
}

fn execute_command(cmd: &str) -> Result<Output, CommandError> { ... }
```

**Use thiserror crate** for error types. Always include:
- Clear error message via `#[error(...)]`
- Descriptive variant names (not generic "Error")
- Chaining with `#[from]` where appropriate

### Error Propagation (MANDATORY)

**Rule:** Propagate errors explicitly; never panic in library code.

```rust
// ❌ WRONG: Panicking
fn load_config(path: &Path) -> Config {
    std::fs::read_to_string(path).expect("failed to read config")
}

// ✅ CORRECT: Returning Result
fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::IoError(e))?;
    let config = serde_json::from_str(&contents)
        .map_err(|e| ConfigError::ParseError(e))?;
    Ok(config)
}
```

**Exception:** Binary entry points (main.rs, Tauri setup) may panic for critical setup failures.

## Rustdoc Standards (MANDATORY)

**Rule:** All public APIs must have complete rustdoc with examples and error semantics.

### Crate-Level Docs

```rust
//! Provides typed host contracts for filesystem, cache, and notification services.
//!
//! This module defines the abstraction boundary between platform-specific implementations
//! (browser/WASM in [platform_host_web] and native/Tauri in [desktop_tauri]) and
//! consumers (desktop_runtime and applications).
//!
//! # Architecture
//!
//! The host layer enables ...
//!
//! # Example
//!
//! ```ignore
//! let cache = HostContext::instance().cache();
//! cache.get("my-key").await?;
//! ```
```

### Module-Level Docs

```rust
//! Command execution and registration interface.
//!
//! Provides [CommandRegistry] for registering commands and [CommandExecutionContext]
//! for executing them within the running desktop session.
```

### Type-Level Docs

```rust
/// Represents the complete state of the desktop shell.
///
/// Contains the reducer-managed state for all shell subsystems: active application,
/// wallpaper, preferences, command history, and theme.
///
/// # Invariants
///
/// - `active_app` may only be set if the app is registered in the app registry
/// - Wallpaper path must be an absolute, validated path or None
/// - Theme tokens must be consistent with the current theme
///
/// # Thread Safety
///
/// DesktopState is not thread-safe. Access it through the effects executor.
#[derive(Clone)]
pub struct DesktopState {
    pub active_app: Option<AppId>,
    // ...
}
```

### Function-Level Docs

```rust
/// Executes a command and returns the output.
///
/// Parses the command line, looks up the command in the registry, and executes it
/// within the provided context. Built-in commands (like `cd`, `theme set`) are
/// executed directly; external commands are dispatched to the terminal process.
///
/// # Arguments
///
/// * `context` - Execution context (filesystem, cache, host services)
/// * `command_line` - Raw command line string
///
/// # Returns
///
/// Returns the command output on success, or a detailed error if:
/// - The command is not found
/// - Arguments are invalid
/// - Execution fails (file not found, permission denied, etc.)
///
/// # Examples
///
/// ```ignore
/// let context = CommandExecutionContext::from_host(host);
/// let output = execute_command(&context, "theme set dark")?;
/// println!("{}", output.stdout);
/// ```
///
/// # Errors
///
/// Returns [CommandError::NotFound] if the command is not registered.
/// Returns [CommandError::InvalidArguments] if argument parsing fails.
pub async fn execute_command(
    context: &CommandExecutionContext,
    command_line: &str,
) -> Result<CommandOutput, CommandError> {
    // ...
}
```

### Error Documentation

```rust
/// Errors that may occur during command execution.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// The command was not found in the registry.
    #[error("command not found: {0}")]
    NotFound(String),

    /// Arguments provided to the command are invalid.
    ///
    /// This includes missing required arguments, invalid argument types,
    /// or conflicting argument combinations.
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    /// The command failed during execution.
    ///
    /// This may be due to permission errors, missing files, or other runtime failures.
    /// The underlying error is included for debugging.
    #[error("execution failed")]
    ExecutionFailed(#[from] std::io::Error),
}
```

## Async/Await Patterns

### Effects-Based State Updates

All side effects in desktop_runtime go through the effects executor, not direct state mutation.

```rust
// ❌ WRONG: Direct mutation in reducer
pub fn reducer(state: &mut DesktopState, action: Action) {
    match action {
        Action::LoadFile(path) => {
            let contents = std::fs::read_to_string(&path).unwrap(); // Blocks!
            state.file_contents = contents;
        }
    }
}

// ✅ CORRECT: Using effects
pub enum Action {
    LoadFile(PathBuf),
    FileLoaded(String),
    LoadFileFailed(String),
}

pub fn reducer(state: &mut DesktopState, action: Action) {
    match action {
        Action::FileLoaded(contents) => {
            state.file_contents = contents;
        }
        Action::LoadFileFailed(err) => {
            state.error = Some(err);
        }
        _ => {}
    }
}

pub async fn file_load_effect(
    context: &HostContext,
    path: PathBuf,
) -> Result<String, String> {
    // Async file load
    context.filesystem().read_to_string(&path).await
        .map_err(|e| e.to_string())
}
```

## Testing Patterns

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reducer_handles_app_launch() {
        let mut state = DesktopState::default();
        let app_id = AppId::new("calc");
        
        state.reducer(Action::LaunchApp(app_id.clone()));
        
        assert_eq!(state.active_app, Some(app_id));
    }

    #[test]
    fn parse_arguments_handles_quoted_strings() {
        let input = r#"open "my file.txt""#;
        let result = parse_arguments(input);
        
        assert_eq!(result, vec!["open", "my file.txt"]);
    }
}
```

### Async Tests

```rust
#[tokio::test]
async fn file_system_loads_file_contents() {
    let fs = TestFileSystem::default();
    fs.write("test.txt", "hello").await;
    
    let contents = fs.read("test.txt").await.unwrap();
    assert_eq!(contents, "hello");
}
```

### Testing Error Paths

```rust
#[test]
fn execute_command_returns_not_found_for_unknown_command() {
    let context = TestCommandContext::default();
    let result = execute_command(&context, "unknown");
    
    match result {
        Err(CommandError::NotFound(cmd)) => assert_eq!(cmd, "unknown"),
        _ => panic!("Expected NotFound error"),
    }
}
```

## Borrowing & Ownership

### References Over Cloning

```rust
// ❌ WRONG: Unnecessary cloning
fn register_app(registry: &mut AppRegistry, app: App) {
    let app_copy = app.clone();
    registry.apps.push(app_copy);
}

// ✅ CORRECT: Take ownership where appropriate
fn register_app(registry: &mut AppRegistry, app: App) {
    registry.apps.push(app);
}

// ✅ CORRECT: Borrow when the caller still needs it
fn get_app(registry: &AppRegistry, id: &AppId) -> Option<&App> {
    registry.apps.iter().find(|app| &app.id == id)
}
```

## Pattern Matching

```rust
// ✅ CORRECT: Use exhaustive matching for Result/Option
match result {
    Ok(output) => println!("{}", output),
    Err(CommandError::NotFound(cmd)) => eprintln!("Command not found: {}", cmd),
    Err(CommandError::InvalidArguments(msg)) => eprintln!("Invalid args: {}", msg),
    Err(CommandError::ExecutionFailed(e)) => eprintln!("Execution failed: {}", e),
}

// ✅ CORRECT: Use if-let for single case
if let Some(app) = state.active_app {
    app.activate();
}
```

## Builder Pattern for Complex Types

```rust
pub struct CommandSpec {
    pub name: String,
    pub description: String,
    pub args: Vec<CommandArgSpec>,
    pub output_shape: CommandOutputShape,
    pub hidden: bool,
}

impl CommandSpec {
    pub fn builder(name: impl Into<String>) -> CommandSpecBuilder {
        CommandSpecBuilder::new(name)
    }
}

pub struct CommandSpecBuilder {
    spec: CommandSpec,
}

impl CommandSpecBuilder {
    fn new(name: impl Into<String>) -> Self {
        Self {
            spec: CommandSpec {
                name: name.into(),
                description: String::new(),
                args: vec![],
                output_shape: CommandOutputShape::default(),
                hidden: false,
            }
        }
    }
    
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.spec.description = desc.into();
        self
    }
    
    pub fn build(self) -> CommandSpec {
        self.spec
    }
}

// Usage
let spec = CommandSpec::builder("theme")
    .description("Manage theme settings")
    .build();
```

## Lifetimes (Use Minimally)

```rust
// ✅ CORRECT: Explicit lifetime when needed
fn parse_command_output<'a>(output: &'a str) -> Result<Command<'a>, ParseError> {
    // ...
}

// ❌ WRONG: Over-specified lifetimes
fn get_active_app(state: &'a DesktopState) -> Option<&'a App> {
    // The lifetime of App doesn't need explicit binding
}

// ✅ CORRECT: Implicit (elided) lifetime
fn get_active_app(state: &DesktopState) -> Option<&App> {
    // ...
}
```

## Module Organization

```rust
// lib.rs: Public API surface
pub mod model;           // Type definitions
pub mod reducer;         // Reducer logic
pub mod host;            // Host integration
pub mod effects;         // Effects implementations

pub use model::{DesktopState, Action};
pub use reducer::reduce;
pub use host::HostContext;

// Unexported internals available internally
mod components;
```

## Doctest Convention

```rust
/// Parse shell command line into argument vector.
///
/// # Examples
///
/// ```ignore
/// let argv = parse_arguments("theme set dark").await?;
/// assert_eq!(argv, vec!["theme", "set", "dark"]);
/// ```
pub async fn parse_arguments(line: &str) -> Result<Vec<String>, ParseError> {
    // ...
}
```

Use `ignore` for doctests that require setup (async, host context, etc.); use normal markers only for self-contained examples.

## Checklist for New Code

- [ ] All public functions have complete rustdoc with Examples, Errors, Invariants (if applicable)
- [ ] All custom error types use `#[derive(thiserror::Error)]`
- [ ] All fallible operations return `Result<T, E>`
- [ ] No unwrap() or panic!() in library code
- [ ] Tests exist for public APIs and error paths
- [ ] No unnecessary cloning; prefer references
- [ ] Async effects are separated from reducer logic
- [ ] Module organization matches logical concerns
