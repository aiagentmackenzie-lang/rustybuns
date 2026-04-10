# RustyBuns — Build Progress

**Last updated:** April 10, 2026  
**Status:** Phase 4 COMPLETE — crash loop protection, global kill-switch, JSONL telemetry, CI, README

---

## What Was Built

### Project Structure
```
RustyBuns/
├── SPEC.md                          ← Updated technical spec with build plan
├── README.md                        ← Project overview and usage
├── PROGRESS.md                      ← Build progress log
├── implant/                         ← Rust crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                 ← Implant binary
│       ├── platform/
│       │   ├── mod.rs              ← Platform trait + Task enum
│       │   ├── windows.rs          ← WindowsPlatform impl (Win32 API)
│       │   ├── unix.rs             ← UnixPlatform impl
│       │   └── task.rs             ← Task enum with execute()
│       └── transport/
│           ├── mod.rs              ← Transport trait
│           └── https.rs            ← HttpsTransport impl
└── controller/                      ← Bun TypeScript project
    ├── package.json
    ├── tsconfig.json
    └── src/
        ├── index.ts               ← HTTP server + CLI commands
        ├── session.ts             ← SessionManager with DB persistence
        ├── task.ts                ← TaskEngine — MITRE ID display
        ├── db.ts                  ← SQLite persistence layer
        └── telemetry.ts           ← JSONL telemetry export
    └── .github/workflows/ci.yml   ← GitHub Actions CI
```

### Implant (`implant/`)
- Rust binary with `tokio` async runtime, `reqwest` HTTPS client
- Commands: `whoami`, `hostname`, `pwd`, `ps`, `ls <path>`, `echo`, `sleep`, `id`, `uname`, `shell <cmd>`, `whoami_all`
- Platform abstraction: `trait Platform` with `WindowsPlatform` (Win32 API) and `UnixPlatform`
- `Task` enum for type-safe dispatch
- `trait Transport` with `HttpsTransport` implementation
- Structured JSON logging with secret redaction
- Exponential backoff, jitter, auto-expiry kill-switch
- **Phase 3 additions:**
  - `cred-access-check`: metadata-only check of credential store paths (no real secrets)
  - `list-env`: list environment variable keys (sensitive values redacted)
  - `list-ssh`: list SSH directory metadata (no key contents)
  - `collect <path>`: bounded file collection (max 100 files, 1MB total, rate-limited 30s)
  - MITRE ATT&CK technique IDs per task (`Task::mitre_id()`)
  - Scope enforcement via env flags: `CRED_ACCESS_ENABLED`, `COLLECTION_ENABLED`, `SHELL_ENABLED`, `ALLOWED_PATHS`
  - Rate limiting: max 1 collection per 30s, max 10MB total per session
  - Result payloads include `mitre_id` and `technique` fields

### Controller (`controller/`)
- Bun.serve HTTP server on port 8080 (HTTP, no TLS for lab mode)
- SQLite persistence via `bun:sqlite` — sessions survive restarts
- Endpoints: `/register`, `/tasks/<uuid>`, `/results/<uuid>`, `/cmd`
- Commands: list, select, task, shell, tag, exit
- **Phase 3 additions:**
  - `mitre_id` and `technique` stored in task results (DB schema updated)
  - Result output shows `[Txxxxx] (technique-name)` format
  - First 3 lines of output printed to console on result

---

## Phase 1 Completed ✓

### 1.1 Transport Refinement
- **Exponential backoff** — Failed beacons trigger backoff: 2^n seconds, max 60s, capped at 5 attempts
- **Environment config** — `BACKOFF_BASE`, `MAX_BACKOFF`, `JITTER_MIN`, `JITTER_MAX` all configurable
- **Jitter validation** — Proper jitter range calculation

### 1.2 Bun PTY Shell
- **New implant commands** — `id`, `uname`, `shell <cmd>` for POSIX targets
- **Controller `shell` command** — queues interactive shell tasks via `POST /cmd`

### 1.3 Structured Logging
- **JSON logs** — All implant logs in structured JSON format to rolling file
- **Redaction** — AWS keys, passwords, tokens, long hex strings auto-redacted
- **Event logging** — `task_received`, `task_completed`, `task_failed`, `backoff`, `implant_expired`

### 1.4 Session Manager
- **Auto-expire stale sessions** — Every 60s, removes sessions with no beacon > 3x jitter interval
- **`touch()` on poll** — `lastSeen` updated on each `GET /tasks/<uuid>`
- **Stale check lifecycle** — `startStaleCheck()` called on server start

---

## Phase 2 Completed ✓

### 2.1 Windows Support
- **windows-rs** crate with Win32 API bindings
- **Win32 ps** — `CreateToolhelp32Snapshot` / `Process32FirstW/NextW` for process enumeration
- **whoami /all** — Windows privilege info via `whoami /all`
- **Runtime OS detection** — `get_os()` and `#[cfg(target_os)]` compile-time routing

### 2.2 Unified Task Interface
- **Platform trait** — `whoami()`, `hostname()`, `pwd()`, `ps()`, `ls()`, `echo()`, `sleep()`, `shell()`, `id()`, `uname()`, `whoami_all()`
- **WindowsPlatform** and **UnixPlatform** implementations
- **Task enum** — `Whoami | Hostname | Pwd | ProcessList | Ls | Echo | Sleep | Shell | Id | Uname | WhoamiAll`
- **Type-safe dispatch** — `Task::from_command()` and `task.execute()`

### 2.3 Transport Pluggability
- **Transport trait** — `send()` and `recv()` with async_trait
- **HttpsTransport** — reqwest-based HTTPS implementation
- **Separation of concerns** — transport logic decoupled from business logic

### 2.4 Session Persistence
- **bun:sqlite** — native SQLite support in Bun
- **Sessions persisted** — survive controller restarts
- **Tasks persisted** — pending tasks stored in SQLite
- **DB_PATH env var** — configurable database location

---

## Phase 3 Completed ✓

### 3.1 MITRE ATT&CK Mapping
- `Task::mitre_id()` method returns technique ID for every task
- Result payloads include `mitre_id` and `technique` fields
- Controller displays `[Txxxx] (name)` on result output

### 3.2 Credential Access Simulation
- `cred-access-check`: lists credential access paths (Windows registry, Unix files) — metadata only
- `list-env`: lists env var keys (sensitive values like `PASSWORD`, `*_KEY`, `TOKEN` shown as `***REDACTED***`)
- `list-ssh`: lists `.ssh` directory entries (type, size, name — no private key contents)
- All gated behind `CRED_ACCESS_ENABLED=true` env flag
- Scope enforcement: commands blocked with clear error message when flag is false

### 3.3 Bounded File Collection
- `collect <path>`: recursive directory scan with limits
- Size limit: max 1MB per collection
- File limit: max 100 files per collection
- Total session limit: max 10MB across all collections
- Rate limit: max 1 collection per 30 seconds
- All gated behind `COLLECTION_ENABLED=true` env flag
- Scope enforcement via `ALLOWED_PATHS` (comma-separated paths)

### 3.4 Scope Enforcement
- `ScopeConfig` struct reads from env vars at startup
- `CRED_ACCESS_ENABLED`, `COLLECTION_ENABLED`, `SHELL_ENABLED` flags
- `ALLOWED_PATHS` comma-separated list for collection scope
- Blocked commands return clear error message (no silent rejection)
- `blocked_processes` field available for future process-level blocking

### 3.5 Controller Result Display
- MITRE ATT&CK IDs shown in result output: `[~] Result for task t1: OK (3ms) [T1033] (whoami)`
- First 3 lines of output printed to console
- Full results stored in SQLite with `mitre_id` and `technique` columns
- `db.ts` schema updated to include new columns

### Verified End-to-End (Phase 3)
```
[~] Result for task t1: OK (0ms) [T1082] (list-env)
    === Environment Variables (Keys Only) ===
    
    HOMEBREW_REPOSITORY=[VALUE]
    ... (44 lines total)

[~] Result for task t2: OK (0ms) [T1082] (list-ssh)
    === SSH Directory (Metadata-Only) ===
    
    SSH dir: /Users/main/.ssh
    ... (9 lines total)

[~] Result for task t3: OK (3ms) [T1074] (collect)
    === Collection Report: /tmp ===
    
    [DIR] Scanning (max 100 files, 1MB total)...
    ... (37 lines total)
```

---

## Phase 4 Completed ✓

### 4.1 Safety Features
- **Global kill-switch**: `shutdown` command via controller halts all implants on next beacon
- **Kill-switch endpoint**: `GET /shutdown` returns `{shutdown: bool}`, implant checks on each beacon
- **Halt command**: `__shutdown` task injected into task queue, implant breaks beacon loop
- **Crash loop protection**: implant halts after 3 fetch failures within 60 seconds

### 4.2 Telemetry Export
- `telemetry.ts`: JSONL export with CIM-aligned fields
- Events: `session_register`, `task_completed`, `task_failed`, `global_shutdown`, `session_stale`
- `@timestamp` ISO8601 field for SIEM compatibility
- Configurable path via `TELEMETRY_PATH` env var (default: `./telemetry.jsonl`)

### 4.3 CI Build Matrix
- `.github/workflows/ci.yml`: GitHub Actions pipeline
- Implant builds: Ubuntu (x64, ARM64), macOS (x64, ARM64), Windows (x64)
- Controller: Bun build step
- Artifacts uploaded for all targets

### 4.4 Documentation
- `README.md`: full project overview, architecture, usage, command reference, ethical framework

---

## Next Agent Prompt

Copy the following to continue from where we left off:

---

You are continuing the RustyBuns project. This is a red-team implant (Rust) + controller (Bun) build for authorized offensive security research.

### Current Status
Phase 4 is complete. All phased features are built: implant, controller, Windows support, transport pluggability, SQLite persistence, MITRE ATT&CK mapping, credential access simulation, bounded collection, scope enforcement, crash loop protection, global kill-switch, JSONL telemetry, CI, and README.

### What's Built
- **Implant**: Rust binary with Platform trait (Windows + Unix), Task enum with MITRE IDs, HttpsTransport, structured JSON logging, scope enforcement, crash loop protection
- **Controller**: Bun HTTP server with SQLite persistence, MITRE ID display, global shutdown command, JSONL telemetry
- **CI**: GitHub Actions build matrix for all platforms
- **Docs**: README with usage, architecture, command reference, ethical framework

### All Phases Complete
Phase 0 (lab prototype) → Phase 1 (transport/logging) → Phase 2 (Windows/unified interface) → Phase 3 (credential simulation/ATT&CK) → Phase 4 (safety/telemetry/CI)

### Known Issues
1. No TLS (lab mode HTTP only)
2. No interactive CLI (use POST /cmd for commands)
3. No PTY streaming (shell works but no interactive terminal)

### To Test
```bash
# Terminal 1 — controller
cd "/Users/main/Security Apps/RustyBuns/controller" && ~/.bun/bin/bun run src/index.ts

# Terminal 2 — implant
cd "/Users/main/Security Apps/RustyBuns/implant" && \
  CRED_ACCESS_ENABLED=true COLLECTION_ENABLED=true \
  IMPLANT_UUID=test C2_HOST=http://localhost:8080 LOG_DIR=/tmp \
  ./target/release/rustybuns-implant

# Terminal 3 — commands
curl -X POST http://localhost:8080/cmd -d 'list'
curl -X POST http://localhost:8080/cmd -d 'shutdown'
```

### Project Spec
Full phased build plan is in `/Users/main/Security Apps/RustyBuns/SPEC.md`.

### Implant (`implant/`)
- Rust binary with `tokio` async runtime, `reqwest` HTTPS client
- Commands: `whoami`, `hostname`, `pwd`, `ps`, `ls <path>`, `echo`, `sleep`, `id`, `uname`, `shell <cmd>`, `whoami_all`
- Beacon loop with configurable jitter (5-15s default), auto-expiry (8h default)
- Registers with controller on startup, polls `/tasks/<uuid>` on each beacon cycle
- Posts results to `/results/<uuid>` after task execution
- Kill-switch via expiry timeout
- Platform abstraction (Windows + Unix implementations)
- Transport trait for pluggable transport
- **Build:** `cargo build --release` → `implant/target/release/rustybuns-implant`
- **Run:** `C2_HOST=http://localhost:8080 ./rustybuns-implant`

### Controller (`controller/`)
- Bun.serve HTTP server on port 8080 (HTTP, no TLS for lab mode)
- SQLite persistence via `bun:sqlite` — sessions survive restarts
- Endpoints:
  - `POST /register` — implant registration, creates session
  - `GET /tasks/<uuid>` — implant polls for tasks, controller returns queued tasks then clears queue
  - `POST /tasks/<uuid>` — operator queues tasks for implant
  - `POST /results/<uuid>` — implant posts task results
  - `POST /cmd` — controller shell commands (list, select, task, tag, exit, help)
- Session manager tracks active implants by UUID with auto-expire
- **Run:** `~/.bun/bin/bun run src/index.ts`

---

## Phase 1 Completed ✓

### 1.1 Transport Refinement
- **Exponential backoff** — Failed beacons trigger backoff: 2^n seconds, max 60s, capped at 5 attempts
- **Environment config** — `BACKOFF_BASE`, `MAX_BACKOFF`, `JITTER_MIN`, `JITTER_MAX` all configurable
- **Jitter validation** — Proper jitter range calculation

### 1.2 Bun PTY Shell
- **New implant commands** — `id`, `uname`, `shell <cmd>` for POSIX targets
- **Controller `shell` command** — queues interactive shell tasks via `POST /cmd`

### 1.3 Structured Logging
- **JSON logs** — All implant logs in structured JSON format to rolling file
- **Redaction** — AWS keys, passwords, tokens, long hex strings auto-redacted
- **Event logging** — `task_received`, `task_completed`, `task_failed`, `backoff`, `implant_expired`

### 1.4 Session Manager
- **Auto-expire stale sessions** — Every 60s, removes sessions with no beacon > 3x jitter interval
- **`touch()` on poll** — `lastSeen` updated on each `GET /tasks/<uuid>`
- **Stale check lifecycle** — `startStaleCheck()` called on server start

---

## Phase 2 Completed ✓

### 2.1 Windows Support
- **windows-rs** crate with Win32 API bindings
- **Win32 ps** — `CreateToolhelp32Snapshot` / `Process32FirstW/NextW` for process enumeration
- **whoami /all** — Windows privilege info via `whoami /all`
- **Runtime OS detection** — `get_os()` and `#[cfg(target_os)]` compile-time routing

### 2.2 Unified Task Interface
- **Platform trait** — `whoami()`, `hostname()`, `pwd()`, `ps()`, `ls()`, `echo()`, `sleep()`, `shell()`, `id()`, `uname()`, `whoami_all()`
- **WindowsPlatform** and **UnixPlatform** implementations
- **Task enum** — `Whoami | Hostname | Pwd | ProcessList | Ls | Echo | Sleep | Shell | Id | Uname | WhoamiAll`
- **Type-safe dispatch** — `Task::from_command()` and `task.execute()`

### 2.3 Transport Pluggability
- **Transport trait** — `send()` and `recv()` with async_trait
- **HttpsTransport** — reqwest-based HTTPS implementation
- **Separation of concerns** — transport logic decoupled from business logic

### 2.4 Session Persistence
- **bun:sqlite** — native SQLite support in Bun
- **Sessions persisted** — survive controller restarts
- **Tasks persisted** — pending tasks stored in SQLite
- **DB_PATH env var** — configurable database location

### Verified End-to-End (Phase 2)
```
[~] Result for task p1: OK (8ms)   ← whoami
[~] Result for task p2: OK (4ms)   ← hostname
[~] Result for task p3: OK (4ms)   ← ps
[~] Result for task p4: OK (7ms)   ← id
[~] Result for task p5: OK (5ms)   ← shell echo
```

---

## Phase 3 Preview (Next)
- Credential access simulation (metadata-only mode)
- MITRE ATT&CK technique mapping
- Bounded file collection
- Scope enforcement

---

## Key Files

| File | Purpose |
|------|---------|
| `SPEC.md` | Updated spec with full phased build plan |
| `implant/src/main.rs` | Rust implant — beacon loop, task execution, HTTPS C2 |
| `implant/src/platform/mod.rs` | Platform trait + Task enum |
| `implant/src/platform/windows.rs` | WindowsPlatform (Win32 API) |
| `implant/src/platform/unix.rs` | UnixPlatform |
| `implant/src/transport/mod.rs` | Transport trait |
| `implant/src/transport/https.rs` | HttpsTransport implementation |
| `implant/Cargo.toml` | Rust dependencies: tokio, reqwest, windows-rs, async-trait |
| `controller/src/index.ts` | Bun HTTP server — register, task queue, results, command endpoint |
| `controller/src/session.ts` | SessionManager with SQLite persistence |
| `controller/src/task.ts` | TaskEngine — create/track tasks, wait for results |
| `controller/src/db.ts` | SQLite persistence layer via bun:sqlite |

---

## Next Agent Prompt

Copy the following to continue from where we left off:

---

You are continuing the RustyBuns project. This is a red-team implant (Rust) + controller (Bun) build for authorized offensive security research.

### Current Status
Phase 3 is complete. The project has MITRE ATT&CK mapping, credential access simulation (metadata-only), bounded file collection, and scope enforcement.

### What's Built
- **Implant**: Rust binary with Platform trait (Windows + Unix), Task enum with MITRE IDs, HttpsTransport, structured JSON logging, scope enforcement
- **Controller**: Bun HTTP server with SQLite persistence, MITRE ID display in results
- New implant commands: `cred-access-check`, `list-env`, `list-ssh`, `collect <path>`
- Scope flags: `CRED_ACCESS_ENABLED`, `COLLECTION_ENABLED`, `SHELL_ENABLED`, `ALLOWED_PATHS`

### Known Issues
1. No TLS (lab mode HTTP only)
2. No interactive CLI (use POST /cmd for commands)
3. No PTY streaming (shell works but no interactive terminal)

### Phase 3 Verified End-to-End
```
[~] Result for task t1: OK (0ms) [T1082] (list-env)
[~] Result for task t2: OK (0ms) [T1082] (list-ssh)
[~] Result for task t3: OK (3ms) [T1074] (collect)
```

### Immediate Goals
1. Phase 4 next: telemetry export (JSONL SIEM format), global kill-switch, crash loop protection refinement, CI build matrix
2. Test with `CRED_ACCESS_ENABLED=true COLLECTION_ENABLED=true ./rustybuns-implant`

### To Test
```bash
# Terminal 1 — start controller
cd "/Users/main/Security Apps/RustyBuns/controller" && ~/.bun/bin/bun run src/index.ts

# Terminal 2 — start implant with Phase 3 features enabled
cd "/Users/main/Security Apps/RustyBuns/implant" && \
  CRED_ACCESS_ENABLED=true COLLECTION_ENABLED=true \
  IMPLANT_UUID=test C2_HOST=http://localhost:8080 LOG_DIR=/tmp \
  ./target/release/rustybuns-implant

# Terminal 3 — queue commands
curl -X POST http://localhost:8080/cmd -d 'list'
curl -X POST http://localhost:8080/tasks/test -H "Content-Type: application/json" \
  -d '{"tasks":[{"id":"t1","command":"list-env"},{"id":"t2","command":"list-ssh"},{"id":"t3","command":"collect","args":["/tmp"]}]}'
```

### Project Spec
Full phased build plan is in `/Users/main/Security Apps/RustyBuns/SPEC.md`.

### Bun Note
Bun is installed at `~/.bun/bin/bun`. Use the full path or ensure `~/.bun/bin` is in your `$PATH`.