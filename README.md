# RustyBuns

Red-team implant (Rust) + controller (Bun) for authorized offensive security research.

## Overview

RustyBuns is a compact, cross-platform C2 framework designed for security researchers and red teams. The Rust implant provides a memory-safe beacon with platform-specific implementations, while the Bun controller offers high-performance session management and an operator interface.

**Use cases:**
- Validate detection rules in isolated lab environments
- Simulate post-exploitation tradecraft within authorized scope
- Test blue team coverage against MITRE ATT&CK techniques

**Authorized use only.** See [Ethical Framework](#ethical-framework).

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Operator Machine (macOS / Linux)                           │
│                                                             │
│  ┌─────────────┐    HTTPS/JSON    ┌──────────────────────┐ │
│  │ Bun Controller │ ◄──────────────► │  Rust Implant        │ │
│  │  (port 8080)  │   GET /tasks    │  (target host)       │ │
│  │               │   POST /results  │                      │ │
│  │  • CLI cmds   │                │  • Task execution    │ │
│  │  • SQLite     │                │  • Platform abstr.   │ │
│  │  • Telemetry  │                │  • JSON logging      │ │
│  └─────────────┘                └──────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Features

| Component | Capability |
|-----------|-----------|
| **Implant** | whoami, hostname, pwd, ps, ls, shell, id, uname, whoami_all, cred-access-check, list-env, list-ssh, collect, enumerate |
| **Platforms** | Windows (Win32 API), Unix (macOS/Linux) |
| **Transport** | HTTPS with strict TLS validation, optional lab bypass, exponential backoff, jitter, auto-expiry |
| **Safety** | Kill-switch, crash loop protection, scope enforcement, process blocking |
| **Telemetry** | JSONL export with MITRE ATT&CK IDs, SIEM-compatible |
| **Controller** | SQLite persistence, session tagging, global shutdown, stale session pruning |

## Quick Start

### Build

```bash
# Implant
cd implant && cargo build --release

# Controller
cd controller && bun install
```

### Run

```bash
# Terminal 1 — controller
cd controller && bun run src/index.ts

# Terminal 2 — implant (lab mode)
cd implant && \
  IMPLANT_UUID=lab-host \
  C2_HOST=http://localhost:8080 \
  LOG_DIR=/tmp \
  ./target/release/rustybuns-implant
```

### Operate

```bash
# List sessions
curl -X POST http://localhost:8080/cmd -d 'list'

# Select a session
curl -X POST http://localhost:8080/cmd -d 'select lab-host'

# Queue a task
curl -X POST http://localhost:8080/cmd -d 'task whoami'

# Queue a one-shot shell command (requires SHELL_ENABLED=true)
curl -X POST http://localhost:8080/cmd -d 'shell-cmd id'

# Shutdown all implants
curl -X POST http://localhost:8080/cmd -d 'shutdown'

# Reset shutdown state
curl -X POST http://localhost:8080/cmd -d 'reset-shutdown'
```

## Implant Commands

| Command | MITRE ID | Description |
|---------|----------|-------------|
| `whoami` | T1033 | Current user |
| `hostname` | T1106 | Hostname |
| `pwd` | T1083 | Current directory |
| `ps` | T1057 | Process list (machine-parseable, macOS/Linux compatible) |
| `ls <path>` | T1083 | Directory listing |
| `shell <cmd>` | T1059 | Execute shell command (implant-side, one-shot) |
| `id` | T1033 | User identity |
| `uname` | T1082 | System info |
| `whoami_all` | T1033 | Privileges (Windows: `whoami /all`; Unix: `id`) |
| `cred-access-check` | T1003 | List credential store paths (metadata-only) |
| `list-env` | T1082 | Environment variable keys (values redacted for sensitive keys) |
| `list-ssh` | T1082 | SSH directory metadata (file names, sizes; no key contents) |
| `collect <path>` | T1074 | Enumerate file metadata (names, sizes) within scope |
| `enumerate <path>` | T1082 | Alias for `collect` |
| `__shutdown` | — | Internal kill-switch command (queued automatically by controller) |

## Scope Enforcement

Control implant capabilities via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `CRED_ACCESS_ENABLED` | `false` | Enable credential access simulation (`cred-access-check`) |
| `COLLECTION_ENABLED` | `false` | Enable `collect`, `enumerate`, `list-env`, and `list-ssh` |
| `SHELL_ENABLED` | `false` | Enable shell command execution |
| `ALLOWED_PATHS` | (all) | Comma-separated allowed collection paths; paths are canonicalized before comparison |
| `BLOCKED_PROCESSES` | (none) | Comma-separated process names blocked from `shell` execution |
| `COLLECTION_INTERVAL` | `30` | Seconds between collections |
| `EXPIRY_HOURS` | `8` | Auto-expire after N hours |
| `C2_INSECURE` | (unset) | Set to `1` to disable TLS certificate validation (lab/self-signed only) |

**Important:** `SHELL_ENABLED` defaults to `false`. Explicitly set `SHELL_ENABLED=true` to allow remote shell execution. `C2_INSECURE=1` prints a loud stderr warning on implant startup.

## Safety Features

- **Kill-switch**: `shutdown` command persists to SQLite; all implants receive `__shutdown` on next beacon
- **Auto-expiry**: Implants terminate after `EXPIRY_HOURS`
- **Crash loop protection**: Halts after 3 failures in 60 seconds
- **Rate limiting**: Collection limited to 1 per 30s, max 10MB aggregate lifetime
- **Size limits**: Max 1MB per collection, 100 files per collection
- **Path traversal prevention**: `ALLOWED_PATHS` are canonicalized before comparison
- **Process blocking**: `BLOCKED_PROCESSES` prevents execution of named binaries via `shell`
- **Output redaction**: Automatic regex-based redaction of AWS keys, bearer tokens, and generic secrets in task results

## Telemetry

JSONL logs written to `telemetry.jsonl` with CIM-aligned fields. Set the path via `TELEMETRY_PATH`.

```json
{"@timestamp":"2026-04-10T00:00:00.000Z","event_type":"session_register","session_id":"abc","hostname":"target","username":"user","os":"linux","level":"INFO"}
{"@timestamp":"2026-04-10T00:00:05.000Z","event_type":"task_completed","session_id":"abc","task_id":"t1","command":"whoami","mitre_id":"T1033","technique":"whoami","success":true,"duration_ms":4,"output_length":27,"level":"INFO"}
```

## Project Structure

```
RustyBuns/
├── implant/                   # Rust implant
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Beacon loop, transport, task dispatch, scope enforcement
│       ├── platform/
│       │   ├── mod.rs         # Platform trait + Task enum + ProcessEntry
│       │   ├── windows.rs     # WindowsPlatform (Win32 API via windows crate)
│       │   ├── unix.rs        # UnixPlatform (macOS/Linux shell commands)
│       │   └── task.rs        # Task definitions, parsing, execution dispatch
│       └── transport/
│           ├── mod.rs          # Transport trait
│           └── https.rs       # HttpsTransport with TLS toggle
├── controller/                 # Bun controller
│   ├── package.json
│   └── src/
│       ├── index.ts           # HTTP server + CLI commands
│       ├── session.ts         # SessionManager (register, select, stale check)
│       ├── task.ts            # TaskEngine (create, store result)
│       ├── db.ts              # SQLite persistence (sessions, tasks, config)
│       └── telemetry.ts       # JSONL telemetry export
├── .github/workflows/ci.yml   # GitHub Actions
├── README.md                  # This file
└── LICENSE                    # Apache 2.0
```

## Ethical Framework

RustyBuns is designed for **authorized offensive security research only**.

### Permitted use
- Red-team exercises with explicit written authorization
- Security testing in environments you own or have permission to test
- Training and research where blue teams are aware

### Prohibited use
- Any system without explicit authorization
- Unauthorized access to any system or data
- Destructive or disruptive activities beyond agreed scope

**You are responsible for ensuring your use complies with all applicable laws and regulations.**

## License

Apache 2.0 — see LICENSE file.