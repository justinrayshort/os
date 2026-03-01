# Naming Conventions

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Audience:** Code authors, reviewers, code generators

All naming must follow idiomatic Rust conventions with project-specific refinements. Consistency is enforced through code review and pattern matching.

## Crate Naming

**Pattern:** `{domain}_{purpose}` in snake_case

Examples:
- `platform_host` (platform domain, host contracts)
- `desktop_runtime` (desktop domain, runtime system)
- `system_ui` (system domain, UI layer)
- `desktop_app_calculator` (desktop apps, calculator)

**Multi-item crates:** Use plural container names
- `crates/apps/` (holds multiple app crates)
- `crates/system_ui/src/primitives/` (holds multiple primitive components)

**Abbreviations:** Avoid unless established (ok: app, ui, fs, io; avoid: mgr, cfg, impl)

## Module Organization

**Pattern:** Logical grouping with shallow nesting

```rust
// crates/desktop_runtime/src/
lib.rs              // Crate root, public API re-exports
mod reducer;        // Reducer logic
mod host;           // Host integration
mod apps;           // App management
mod components;     // Shell view components
mod model;          // Type definitions
mod persistence;    // Persistence effects
mod wallpaper;      // Wallpaper effects
```

**Guidelines:**
- One concern per module file (reducer.rs is the entire reducer, not a directory)
- Submodules (host/, apps/, components/) group related concerns
- Private implementation details prefixed with `_` if necessary (rare)
- Public API surfaces explicitly re-exported in module.rs or lib.rs

## Type & Trait Naming

**Pattern:** PascalCase, descriptive (not abbreviated)

Examples:
- `DesktopState` (not DState, State)
- `CommandRegistry` (not CmdReg, Registry)
- `FileSystemError` (not FSErr, Error)

**Traits:** Descriptive, verb/capability-oriented
- `CommandHandler` (can handle commands)
- `Serializable` (can be serialized)
- `CacheProvider` (provides caching)

**Avoid:** Generic T, U, V names except in heavily generic contexts (Result<T, E> is ok; avoid `struct Container<T, U>` without clear semantics)

**Error Types:** Always suffixed with `Error`
- `PersistenceError`
- `FileSystemError`
- `CommandExecutionError`

## Function & Method Naming

**Pattern:** snake_case, verb-forward

Examples:
- `execute_command()` (not command_execute)
- `parse_arguments()` (not arguments_parse)
- `register_app()` (not app_register)

**Getters/Setters:** No prefix
- `fn state(&self) -> &DesktopState` (not get_state)
- `fn set_state(&mut self, s: DesktopState)` (rare; prefer builder pattern)

**Factory functions:** Descriptive
- `DesktopState::new()` or `DesktopState::default()` (standard constructors)
- `DesktopState::from_config()` (from specific source)
- `DesktopState::builder()` (builder pattern)

**Async functions:** No special prefix; context makes it clear
- `async fn load_configuration()` (not async_load_config)

## Constant Naming

**Pattern:** SCREAMING_SNAKE_CASE

Examples:
- `const DEFAULT_BUFFER_SIZE: usize = 8192;`
- `const MAX_CONCURRENT_COMMANDS: u32 = 10;`

**Avoid:** Generic constants. Always be specific about what the constant is.
- ❌ `const MAX: u32 = 100;`
- ✅ `const MAX_COMMAND_QUEUE_SIZE: u32 = 100;`

## Feature Flags

**Pattern:** kebab-case, descriptive

Examples:
- `enable-profiling` (profiling instrumentation)
- `experimental-wallpaper-effects` (preview feature)
- `dev-logging` (dev-only verbose logging)

**Convention:** `dev-*` prefix for development-only features, `experimental-*` for preview features

## File Organization

**Pattern:** Match module structure

```
crates/desktop_runtime/src/
├── lib.rs                          # Crate root
├── reducer.rs                      # Main reducer
├── host/                           # Host integration submodule
│   ├── mod.rs
│   ├── boot.rs
│   ├── effects.rs
│   ├── app_bus.rs
│   └── persistence_effects.rs
├── apps.rs                         # App management
├── components.rs                   # Shell components
├── model.rs                        # Type definitions
└── shell/                          # Shell submodule
    ├── mod.rs
    ├── policy.rs
    ├── commands/
    │   ├── mod.rs
    │   ├── apps.rs
    │   ├── filesystem.rs
    │   ├── config.rs
    │   └── theme.rs
    └── ...
```

**Guidelines:**
- File per module (desktop_runtime/src/reducer.rs is the reducer module)
- Submodule directory when grouping related files (host/, components/)
- Private modules stay private; public API re-exported in parent module.rs
- src/tests.rs is optional; prefer #[cfg(test)] in item module or dedicated test subdir

## Icon Naming (system_ui)

**Pattern:** Descriptive, semantic IconName enum variants

Examples:
- `IconName::FileSave`
- `IconName::SettingsGear`
- `IconName::TerminalPrompt`

**Convention:** Match Fluent icon naming where possible; project-specific icons get descriptive names

## Variable & Parameter Naming

**Pattern:** snake_case, descriptive

Examples:
```rust
fn execute_command(context: &CommandExecutionContext, command_line: String) -> Result<CommandOutput> {
    let parsed_argv = parse_arguments(&command_line)?;
    let result = execute_builtin_or_external(&parsed_argv, context)?;
    Ok(CommandOutput { ... })
}
```

**Avoid:** Single letters except in tight loops (i, j for loop counters is ok; x for a state variable is not)

## Enum Variant Naming

**Pattern:** PascalCase, concise but descriptive

Examples:
```rust
pub enum CommandError {
    NotFound,                   // Command not found
    InvalidArguments(String),   // Invalid args with context
    ExecutionFailed(String),    // Execution error with message
}
```

**Never** repeat enum name in variant:
- ❌ `CommandError::CommandNotFound`
- ✅ `CommandError::NotFound` (context provides clarity)

## Test Function Naming

**Pattern:** `test_{feature}_{scenario}` or `{feature}_should_{behavior}`

Examples:
```rust
#[test]
fn reducer_handles_app_launch() { ... }

#[test]
fn file_system_returns_error_on_missing_file() { ... }

#[test]
fn parse_arguments_handles_quoted_strings() { ... }
```

## Struct Field Naming

**Pattern:** snake_case, clear semantics

Examples:
```rust
pub struct DesktopState {
    pub active_app: Option<AppId>,
    pub wallpaper_path: PathBuf,
    pub command_history: Vec<String>,
    pub theme_tokens: ThemeTokens,
}
```

**Avoid:** Redundant field names
- ❌ `struct AppInfo { app_name: String, app_state: AppState }`
- ✅ `struct AppInfo { name: String, state: AppState }`

## Comment & Documentation Naming

**Rustdoc sections:** Use standard headings
- `# Examples` (for runnable examples)
- `# Errors` (for error semantics)
- `# Panics` (if panic behavior documented)
- `# Safety` (for unsafe blocks)

**Code comments:** Use explanatory, not statement-repeating
- ❌ `// increment counter` above `counter += 1;`
- ✅ `// limit queue depth to prevent unbounded growth` before queue size check

## Acronym Handling

**Avoid acronyms in public APIs; spell out:**
- ❌ `FSError`, `IOContext`, `UIToken`
- ✅ `FileSystemError`, `InputOutputContext`, `UserInterfaceToken`

**Exceptions:** Well-established (RPC, HTTP, JSON, WASM, IPC) and single-crate internal code

## Checklist

When creating new code:
- [ ] All functions/types have clear, verb-forward names
- [ ] Error types end with `Error`
- [ ] Constants are SCREAMING_SNAKE_CASE and specific
- [ ] File/module structure matches logical organization
- [ ] No abbreviated type names in public API
- [ ] Traits are capability-oriented (e.g., CommandHandler)
- [ ] Test functions describe what they test (test_X_does_Y)
