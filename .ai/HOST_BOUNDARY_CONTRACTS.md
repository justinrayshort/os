# Host Boundary Contracts

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Audience:** Platform host implementers, architects, code reviewers

Defines the typed contract layer that abstracts platform-specific functionality (browser/WASM vs. native/Tauri) from the runtime and applications.

## Contract Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│  Consumers (desktop_runtime, apps)                              │
│  - Request services via HostContext                              │
│  - Work with strongly-typed requests/responses                  │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           │ (typed platform_host contracts)
                           ▼
┌──────────────────────────────────────────────────────────────────┐
│  platform_host (Contract Definitions)                            │
│  - FileSystemService trait                                       │
│  - CacheService trait                                            │
│  - NotificationService trait                                     │
│  - ProcessManager trait                                          │
│  - WallpaperManager trait                                        │
│  - SessionManager trait                                          │
│  - ExternalUrlDispatcher trait                                   │
└────────┬─────────────────────────────────────┬────────────────┬──┘
         │                                     │                │
    (impl)                               (impl)            (impl)
         │                                     │                │
         ▼                                     ▼                ▼
┌────────────────────────┐  ┌─────────────────────┐  ┌──────────────┐
│ platform_host_web      │  │ desktop_tauri       │  │ (future)     │
│ (Browser/WASM)         │  │ (Native/Tauri)      │  │ Native impl  │
│ - IndexedDB adapters   │  │ - File system       │  │              │
│ - Storage APIs         │  │ - Notifications     │  └──────────────┘
│ - Mock terminal        │  │ - Terminal process  │
│ - Fetch-based URLs     │  │ - OS integration    │
└────────────────────────┘  └─────────────────────┘
```

## Service Contracts

### FileSystemService

**Purpose:** Abstract filesystem operations (read, write, delete, list directories)

**Key Types:**
- `FileSystemError` – Errors (NotFound, PermissionDenied, etc.)
- `FileMetadata` – File information (size, modified time, is_dir)
- `DirectoryEntry` – Directory listing result

**Key Methods:**
```rust
pub trait FileSystemService {
    async fn read_to_string(&self, path: &Path) -> Result<String, FileSystemError>;
    async fn write(&self, path: &Path, contents: &str) -> Result<(), FileSystemError>;
    async fn delete(&self, path: &Path) -> Result<(), FileSystemError>;
    async fn list_directory(&self, path: &Path) -> Result<Vec<DirectoryEntry>, FileSystemError>;
    async fn metadata(&self, path: &Path) -> Result<FileMetadata, FileSystemError>;
    async fn exists(&self, path: &Path) -> Result<bool, FileSystemError>;
}
```

**Implementation Notes:**
- Browser impl: Uses IndexedDB or browser storage for sandboxed filesystem
- Native impl: Direct filesystem access via Tauri or OS APIs

### CacheService

**Purpose:** Persistent key-value storage (preferences, cache, session data)

**Key Types:**
- `CacheError` – Errors (StorageFull, CorruptedData)
- `CacheKey` – String wrapper for cache keys

**Key Methods:**
```rust
pub trait CacheService {
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), CacheError>;
    async fn delete(&self, key: &str) -> Result<(), CacheError>;
    async fn clear(&self) -> Result<(), CacheError>;
    async fn keys(&self) -> Result<Vec<String>, CacheError>;
}
```

**Implementation Notes:**
- Browser impl: IndexedDB or localStorage
- Native impl: Tauri data dir with JSON/binary persistence

### NotificationService

**Purpose:** Display system notifications (OS/browser notifications)

**Key Types:**
- `Notification` – Title, body, icon, actions
- `NotificationError` – Permission denied, etc.

**Key Methods:**
```rust
pub trait NotificationService {
    async fn show(&self, notification: Notification) -> Result<(), NotificationError>;
    async fn request_permission(&self) -> Result<PermissionStatus, NotificationError>;
}
```

**Implementation Notes:**
- Browser impl: Web Notifications API (with permission prompt)
- Native impl: OS notification APIs (Tauri integration)

### ProcessManager

**Purpose:** Launch and manage child processes (terminal, external commands)

**Key Types:**
- `ProcessHandle` – Reference to running process
- `ProcessOutput` – stdout, stderr, exit code
- `ProcessError` – Spawn failed, timeout, etc.

**Key Methods:**
```rust
pub trait ProcessManager {
    async fn spawn(
        &self,
        command: &str,
        args: &[&str],
        cwd: Option<&Path>,
    ) -> Result<ProcessHandle, ProcessError>;

    async fn wait(&self, handle: ProcessHandle) -> Result<ProcessOutput, ProcessError>;
}
```

**Implementation Notes:**
- Browser impl: Mock/headless (no actual process)
- Native impl: Tauri command execution

### WallpaperManager

**Purpose:** Load and manage desktop wallpaper

**Key Types:**
- `WallpaperPath` – Validated path to wallpaper image
- `WallpaperError` – File not found, invalid format

**Key Methods:**
```rust
pub trait WallpaperManager {
    async fn load(&self, path: &Path) -> Result<Vec<u8>, WallpaperError>;
    async fn list_available(&self) -> Result<Vec<PathBuf>, WallpaperError>;
    async fn set_current(&self, path: &Path) -> Result<(), WallpaperError>;
}
```

**Implementation Notes:**
- Browser impl: Load images from IndexedDB or data URLs
- Native impl: Load from filesystem and apply via Tauri

### SessionManager

**Purpose:** Manage user session (login, logout, session state)

**Key Types:**
- `SessionInfo` – Current user, role, preferences
- `SessionError` – Not authenticated, expired

**Key Methods:**
```rust
pub trait SessionManager {
    async fn current_session(&self) -> Result<SessionInfo, SessionError>;
    async fn logout(&self) -> Result<(), SessionError>;
}
```

## Request/Response Envelope Pattern

All host service calls follow this pattern:

```rust
// Request envelope (input to host service)
pub struct FileReadRequest {
    pub path: PathBuf,
}

// Response envelope (output from host service)
pub struct FileReadResponse {
    pub contents: String,
}

// Service method
pub trait FileSystemService {
    async fn read(&self, req: FileReadRequest) -> Result<FileReadResponse, FileSystemError>;
}
```

**Benefits:**
- Backward-compatible versioning
- Clear request/response semantics
- Facilitates serialization across boundary

## Invariants & Guarantees

### 1. Platform Abstraction

**Invariant:** Implementation details are completely hidden from consumers.

- Apps don't know if they're running in browser or native
- desktop_runtime doesn't call platform-specific APIs directly
- All platform checks happen in implementation layer only

### 2. Error Semantics

**Invariant:** Errors are consistent across implementations; semantic errors are mapped.

Example:
- Browser's `DOMException: Quota exceeded` → `CacheError::StorageFull`
- Native's "Permission denied" → `FileSystemError::PermissionDenied`

### 3. Async All The Way

**Invariant:** All host operations are async (or at minimum can be async).

This enables:
- Graceful degradation in browser (permissions, network latency)
- Consistent API for native and web

### 4. No Blocking Operations

**Invariant:** Host services never block the runtime thread.

- All I/O is asynchronous
- No synchronous filesystem access
- Effects executor serializes host calls

## Cross-Boundary Communication

### Allowed
- Typed request/response through platform_host traits
- Effects executor routing calls to appropriate implementation
- Opaque impl details within desktop_tauri/platform_host_web

### Forbidden
- Direct tauri::* imports in desktop_runtime or apps
- Direct web-sys::* imports in desktop_runtime or apps
- Mixing implementations (web impl calling native code)
- Exposing platform-specific types across boundary

## Implementation Checklist

When adding a new host service:

- [ ] Define trait in platform_host with clear method signatures
- [ ] Define request/response types (if needed)
- [ ] Define error type (derive thiserror::Error)
- [ ] Document all invariants in rustdoc
- [ ] Implement in platform_host_web (browser version)
- [ ] Implement in desktop_tauri (native version)
- [ ] Add integration tests for both implementations
- [ ] Update HostContext to expose new service
- [ ] Update desktop_runtime to use new service (if needed)

## Testing Strategy

### Unit Tests
- Test each implementation in isolation
- Use mock/fake host services in runtime/app tests

### Integration Tests
- Test full cross-boundary flow
- Verify request/response serialization
- Test error handling for both implementations

### Platform-Specific Tests
- Browser tests in platform_host_web tests
- Native tests in desktop_tauri tests

## Versioning & Evolution

When changing a service contract:

1. **Add new method** (backward-compatible):
   - Add new method to trait
   - Implement in both implementations
   - Update HostContext

2. **Change existing method** (breaking):
   - Deprecated old method first (release cycle)
   - Add new method with new signature
   - Update all callsites
   - Document migration path in Wiki

3. **Remove service** (breaking):
   - Deprecate first
   - Provide migration guidance
   - Remove in next major version

Always coordinate with code changes and documentation updates per AGENTS.md section 6.
