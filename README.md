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
│  │  (port 8080)  │   GET /tasks    │  (target host)        │ │
│  │               │   POST /results│                       │ │
│  │  • CLI cmds   │                │  • Task execution    │ │
│  │  • SQLite     │                │  • Platform abstr.   │ │
│  │  • Telemetry  │                │  • JSON logging      │ │
│  └─────────────┘                └──────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Features

| Component | Capability |
|-----------|-----------|
| **Implant** | whoami, hostname, pwd, ps, ls, shell, id, uname, cred-access-check, list-env, list-ssh, collect |
| **Platforms** | Windows (Win32 API), Unix (macOS/Linux) |
| **Transport** | HTTPS with exponential backoff, jitter, auto-expiry |
| **Safety** | Kill-switch, crash loop protection, scope enforcement |
| **Telemetry** | JSONL export with MITRE ATT&CK IDs, SIEM-compatible |
| **Controller** | SQLite persistence, session tagging, global shutdown |

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

# Queue a task
curl -X POST http://localhost:8080/tasks/<uuid> \
  -H "Content-Type: application/json" \
  -d '{"tasks":[{"id":"t1","command":"whoami"}]}'

# Shutdown all implants
curl -X POST http://localhost:8080/cmd -d 'shutdown'
```

## Implant Commands

| Command | MITRE ID | Description |
|---------|----------|-------------|
| `whoami` | T1033 | Current user |
| `hostname` | T1106 | Hostname |
| `pwd` | T1083 | Current directory |
| `ps` | T1057 | Process list |
| `ls <path>` | T1083 | Directory listing |
| `shell <cmd>` | T1059 | Execute shell command |
| `id` | T1033 | User identity |
| `uname` | T1082 | System info |
| `whoami_all` | T1033 | Privileges (Windows) |
| `cred-access-check` | T1003 | List credential store paths (metadata-only) |
| `list-env` | T1082 | Environment variable keys |
| `list-ssh` | T1082 | SSH directory (metadata-only) |
| `collect <path>` | T1074 | Enumerate file metadata (names, sizes) within scope |
| `enumerate <path>` | T1082 | Alias for `collect` — enumerate file metadata (names, sizes) within scope |

## Scope Enforcement

Control implant capabilities via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `CRED_ACCESS_ENABLED` | `false` | Enable credential access simulation |
| `COLLECTION_ENABLED` | `false` | Enable file collection |
| `SHELL_ENABLED` | `true` | Enable shell command execution |
| `ALLOWED_PATHS` | (all) | Comma-separated allowed collection paths |
| `COLLECTION_INTERVAL` | `30` | Seconds between collections |
| `EXPIRY_HOURS` | `8` | Auto-expire after N hours |

## Safety Features

- **Kill-switch**: `shutdown` command halts all implants on next beacon
- **Auto-expiry**: Implants terminate after `EXPIRY_HOURS`
- **Crash loop protection**: Halts after 3 failures in 60 seconds
- **Rate limiting**: Collection limited to 1 per 30s, max 10MB total
- **Size limits**: Max 1MB per collection, 100 files per collection

## Telemetry

JSONL logs written to `telemetry.jsonl` with CIM-aligned fields:

```json
{"@timestamp":"2026-04-10T00:00:00.000Z","event_type":"session_register","session_id":"abc","hostname":"target","username":"user","os":"linux","level":"INFO"}
{"@timestamp":"2026-04-10T00:00:05.000Z","event_type":"task_completed","session_id":"abc","task_id":"t1","command":"whoami","mitre_id":"T1033","technique":"whoami","success":true,"duration_ms":4,"level":"INFO"}
```

Set path via `TELEMETRY_PATH` environment variable.

## Project Structure

```
RustyBuns/
├── implant/                   # Rust implant
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # Beacon loop, transport, task dispatch
│       ├── platform/
│       │   ├── mod.rs       # Platform trait + Task enum
│       │   ├── windows.rs    # WindowsPlatform
│       │   ├── unix.rs       # UnixPlatform
│       │   └── task.rs       # Task definitions
│       └── transport/
│           ├── mod.rs        # Transport trait
│           └── https.rs      # HttpsTransport
├── controller/               # Bun controller
│   ├── package.json
│   └── src/
│       ├── index.ts         # HTTP server + CLI commands
│       ├── session.ts       # SessionManager
│       ├── task.ts          # TaskEngine
│       ├── db.ts            # SQLite persistence
│       └── telemetry.ts      # JSONL telemetry export
├── .github/workflows/ci.yml  # GitHub Actions
├── SPEC.md                   # Full technical specification
└── PROGRESS.md               # Build progress log
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

## Collaborations

Feel free to contribute, improve or expand on this project.
