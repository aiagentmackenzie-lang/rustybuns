# RustyBuns — Build Continuation Guide

**Last updated:** April 9, 2026  
**Project:** Red-team implant (Rust) + controller (Bun) for authorized offensive security research

---

## Project Overview

RustyBuns is a lab prototype for a command-and-control (C2) toolkit consisting of:
- **Implant**: Standalone Rust binary that runs on target systems, communicates over HTTPS to a controller
- **Controller**: Bun/TypeScript HTTP server that manages sessions, queues tasks, and receives results

### Key Files

| File | Purpose |
|------|---------|
| `/Users/main/Security Apps/RustyBuns/SPEC.md` | Full technical spec, architecture, phased build plan |
| `/Users/main/Security Apps/RustyBuns/PROGRESS.md` | Detailed progress log, what's completed, remaining issues |
| `/Users/main/Security Apps/RustyBuns/implant/src/main.rs` | Rust implant — beacon loop, task execution, C2 client |
| `/Users/main/Security Apps/RustyBuns/implant/Cargo.toml` | Rust dependencies |
| `/Users/main/Security Apps/RustyBuns/implant/src/platform/mod.rs` | Platform trait + Task enum |
| `/Users/main/Security Apps/RustyBuns/implant/src/platform/windows.rs` | WindowsPlatform impl (Win32 API) |
| `/Users/main/Security Apps/RustyBuns/implant/src/platform/unix.rs` | UnixPlatform impl |
| `/Users/main/Security Apps/RustyBuns/implant/src/transport/mod.rs` | Transport trait |
| `/Users/main/Security Apps/RustyBuns/implant/src/transport/https.rs` | HttpsTransport implementation |
| `/Users/main/Security Apps/RustyBuns/controller/src/index.ts` | Bun HTTP server + CLI commands |
| `/Users/main/Security Apps/RustyBuns/controller/src/session.ts` | Session manager with auto-expire + DB persistence |
| `/Users/main/Security Apps/RustyBuns/controller/src/task.ts` | Task engine — create/track tasks |
| `/Users/main/Security Apps/RustyBuns/controller/src/db.ts` | SQLite persistence layer |

---

## Current Status: Phase 2 Complete ✓

### What's Built (Phase 0 + Phase 1 + Phase 2)

**Implant:**
- Rust binary with `tokio` async runtime, `reqwest` HTTPS client
- Commands: `whoami`, `hostname`, `pwd`, `ps`, `ls <path>`, `echo`, `sleep`, `id`, `uname`, `shell <cmd>`, `whoami_all`
- Beacon loop with configurable jitter (5-15s default)
- Exponential backoff on failed beacons (2^n seconds, max 60s)
- Auto-expiry kill-switch (8h default)
- Structured JSON logging with secret redaction (AWS keys, passwords, tokens)
- Registers with controller on startup, polls `/tasks/<uuid>`, posts results to `/results/<uuid>`

**Platform Abstraction:**
- `trait Platform` with `WindowsPlatform` and `UnixPlatform` implementations
- Windows `ps` via Win32 `CreateToolhelp32Snapshot` / `Process32FirstW/NextW`
- Windows `whoami /all` via `whoami /all`
- Runtime OS detection routes to correct implementation
- `Task` enum for type-safe task dispatch

**Transport:**
- `trait Transport` with `send()`/`recv()` for pluggable transports
- `HttpsTransport` implementation with reqwest
- Clean separation of transport from business logic

**Controller:**
- Bun.serve HTTP server on port 8080 (HTTP, no TLS for lab mode)
- SQLite persistence via `bun:sqlite` — sessions survive restarts
- Endpoints:
  - `POST /register` — implant registration
  - `GET /tasks/<uuid>` — implant polls for tasks
  - `POST /tasks/<uuid>` — operator queues tasks
  - `POST /results/<uuid>` — implant posts task results
  - `POST /cmd` — controller commands (list, select, task, shell, tag, exit)
- Session manager with auto-expire (removes stale sessions every 60s)
- `lastSeen` updated on each implant poll
- Sessions and tasks persisted to SQLite database

**Build:**
```bash
# Implant
cd "/Users/main/Security Apps/RustyBuns/implant" && cargo build --release

# Controller
cd "/Users/main/Security Apps/RustyBuns/controller" && ~/.bun/bin/bun run src/index.ts
```

---

## How to Test Right Now

### Terminal 1 — Start Controller
```bash
cd "/Users/main/Security Apps/RustyBuns/controller" && ~/.bun/bin/bun run src/index.ts
```

### Terminal 2 — Start Implant (with logging)
```bash
cd "/Users/main/Security Apps/RustyBuns/implant" && LOG_DIR=/tmp ./target/release/rustybuns-implant
```

### Terminal 3 — Register and Task
```bash
# Register (implant auto-registers, but for fixed UUID testing):
FIXED_UUID="test-001"
curl -X POST http://localhost:8080/register \
  -H "Content-Type: application/json" \
  -d "{\"uuid\":\"$FIXED_UUID\",\"hostname\":\"test\",\"username\":\"user\",\"os\":\"macos\",\"version\":\"0.1.0\",\"expiry_hours\":8}"

# Queue tasks
curl -X POST http://localhost:8080/tasks/$FIXED_UUID \
  -H "Content-Type: application/json" \
  -d '{"tasks":[{"id":"t1","command":"whoami"},{"id":"t2","command":"hostname"},{"id":"t3","command":"shell","args":["echo hello"]}]}'

# List sessions
curl -X POST http://localhost:8080/cmd -d 'list'
```

### For Fixed UUID Testing
The implant generates a new random UUID each run. To use a fixed UUID:
```bash
IMPLANT_UUID=test-001 C2_HOST=http://localhost:8080 LOG_DIR=/tmp ./target/release/rustybuns-implant
```

---

## Known Issues

1. **Implant UUID changes each run** — Use `IMPLANT_UUID=<固定值>` env var for consistent testing
2. **HTTP only (no TLS)** — Lab mode uses plain HTTP; production would need HTTPS/TLS
3. **Interactive CLI missing** — Controller lacks stdin read loop; use `POST /cmd` for all commands
4. **PTY streaming not implemented** — `shell` command works but doesn't stream interactively; full PTY requires Bun.Terminal integration and interactive controller

---

## Next Steps (Phase 3)

### 3.1 Credential Access Simulation
- Metadata-only mode: list credential stores (Windows: LSASS, Sam, Vault; Linux: /etc/shadow, ~/.ssh, keyring) but do NOT extract or display actual secrets
- `cred-access-check`: confirm access paths exist and are readable, log result without exfil
- All credential access gated behind `CRED_ACCESS_ENABLED=true` env flag

### 3.2 MITRE ATT&CK Mapping
- Task tags: each task mapped to ATT&CK technique ID (e.g., `T1003.001` for credential dumping)
- Config profiles: "enterprise-workstation", "server", "domain-controller"
- Profile enables/disables techniques based on engagement scope

### 3.3 File Collection (Bounded)
- Collect: system info, running processes, network connections, scheduled tasks
- Size limit: max 1MB per collection, 10MB total per session
- Rate limit: max 1 collection per 30 seconds to avoid network saturation

### 3.4 Scope Enforcement
- Config: allowed CIDRs, blocked processes (antivirus), max data exfil
- Implant refuses tasks outside scope with error code + log entry

---

## Architecture Notes

### Platform Abstraction
The implant uses a `trait Platform` with OS-specific implementations:
- `UnixPlatform` for Linux/macOS — `ps` via `ps aux`, `shell` via `sh -c`
- `WindowsPlatform` for Windows — `ps` via Win32 `CreateToolhelp32Snapshot`, `shell` via `cmd /C`

### Task Enum
`Task` enum replaces the old match-based dispatcher:
```rust
pub enum Task {
    Whoami, Hostname, Pwd, ProcessList,
    Ls(String), Echo(Vec<String>), Sleep(u64),
    Shell(String), Id, Uname, WhoamiAll,
}
```

### Transport Trait
`trait Transport` enables pluggable transports:
```rust
#[async_trait]
pub trait Transport {
    async fn send<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<(), TransportError>;
    async fn recv<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T, TransportError>;
}
```
Currently `HttpsTransport` wraps reqwest.

### Session Persistence
Controller uses `bun:sqlite` to persist sessions and tasks to `./rustybuns.db` (configurable via `DB_PATH` env var). Sessions reload on controller startup.

### JSON Envelope Format
Controller returns:
```json
{ "tasks": [{ "id": "...", "command": "...", "args": [...] }] }
```
Not a raw array.

---

## Environment Variables

**Implant:**
| Variable | Default | Purpose |
|----------|---------|---------|
| `C2_HOST` | `http://localhost:8080` | Controller URL |
| `IMPLANT_UUID` | (random) | Fixed UUID for testing |
| `LOG_DIR` | `.` | Log output directory |
| `JITTER_MIN` | 5 | Min beacon interval (seconds) |
| `JITTER_MAX` | 15 | Max beacon interval (seconds) |
| `EXPIRY_HOURS` | 8 | Auto-kill after N hours |
| `BACKOFF_BASE` | 2 | Exponential backoff base |
| `MAX_BACKOFF` | 60 | Max backoff seconds |

**Controller:**
| Variable | Default | Purpose |
|----------|---------|---------|
| `PORT` | 8080 | HTTP server port |
| `DB_PATH` | `./rustybuns.db` | SQLite database path |

---

## For Your Next Session

When continuing this build, you should:

1. **Read SPEC.md** first to understand the full architecture and build plan
2. **Review PROGRESS.md** for what's completed and remaining issues
3. **Start fresh controller/implant** to verify everything still works after any changes
4. **Test incrementally** — make changes, build, test with fixed UUID before moving on
5. **Phase 3 focus**: Credential access simulation (metadata-only), MITRE ATT&CK mapping, bounded file collection, scope enforcement

### Key Prompt for Continuing:

> Continue the RustyBuns project. Phase 2 is complete with: windows-rs support with Win32 process enumeration, unified Platform trait with WindowsPlatform and UnixPlatform, Task enum for type-safe dispatch, Transport trait with HttpsTransport implementation, SQLite session persistence via bun:sqlite. The implant and controller are working on macOS. Next goals are Phase 3: credential access simulation (metadata-only mode), MITRE ATT&CK technique mapping, bounded file collection, and scope enforcement. Start by verifying the current build works.

---

## Ethical Reminder

RustyBuns is for **authorized offensive security research only**. Always ensure:
- Explicit written authorization from system owners
- Operations within agreed scope
- No unauthorized persistence or lateral movement
- Discovered credentials/secrets handled per AI red-team ethics guidance
