# RustyBuns: Standalone Rust + Bun Red-Team Tool – Technical Spec and Build Plan

## 1. Purpose and Role (Red-Team Lead Perspective)

RustyBuns is a **standalone red-team implant and operator console** built with a Rust core and a Bun-based control layer, designed for use in **authorized offensive security assessments only**.
Its purpose is to provide a compact, cross-platform, memory-conscious tool for simulating modern adversary tradecraft on workstations and servers while maintaining strong operational safety and ethical controls.

The design is inspired by public offensive-Rust work (Black Hat Rust, OffensiveRust, RustRedOps) that demonstrates Rust's suitability for implants, process injection, and low-level evasion, combined with Bun's high-performance runtime and PTY capabilities for operator UX and payload flexibility.

---

## 2. Ethical and Legal Framework

RustyBuns must only be used:

- Under **explicit written authorization** (rules of engagement, signed testing scope) from the system owner.
- For **training, research, and red-team exercises** where blue teams are aware at the organizational level, even if not at the tactical level.
- With documented **data handling, logging, and cleanup procedures** ensuring no residual sensitive data or uncontrolled persistence.

Key ethical constraints:

- Operations must remain **within scope**; unauthorized lateral movement, exfiltration from third parties, or persistence outside the agreed environment is prohibited.
- Discovered credentials, secrets, or sensitive data must be **minimized, redacted, and reported** in line with AI/red-team ethical guidance and incident-handling best practice.
- The tool must include explicit **opt-in flags** for any destructive or high-impact actions (e.g., overwriting binaries, destructive privilege-escalation tests).

This aligns with contemporary guidance on red-team ethics and AI/LLM red-teaming practice, emphasizing non-maleficence, informed consent, and legal compliance.

---

## 3. High-Level Concept

### 3.1 Architecture Reality

After research, the architecture is clarified as follows:

- **Rust Implant**: A standalone binary compiled from Rust. Runs on target systems. Does NOT include or embed Bun — Bun is a runtime, not an embeddable library. The implant communicates over network (HTTPS C2) to a controller.

- **Bun Controller**: Runs on the operator's machine (macOS, Linux) — NOT on targets. Provides CLI/TUI, PTY shells (POSIX), session management, tasking. FFI (if used) runs here on the operator's side, not on targets.

- **FFI Scope**: `bun:ffi` is experimental but acceptable for controller-side use on operator infrastructure. FFI is never deployed to target machines.

- **PTY Limitation**: Bun PTY is POSIX-only (Linux/macOS). Windows targets will receive a non-PTY shell experience (raw output streaming). This is an accepted gap.

### 3.2 Operator View

From an operator's perspective, RustyBuns is:

- A **single binary implant** that can be dropped on a target (or delivered via other vectors) and connects back to a controller.
- A **Bun-powered controller** that provides:
  - Encrypted command-and-control (C2) over HTTPS.
  - An interactive shell using Bun's PTY capabilities (POSIX targets).
  - A modular tasking system for credential access, reconnaissance, and collection.

### 3.3 Design Goals

- **Stealth by default**: small runtime footprint, minimal dependencies, no installer.
- **Memory safety for the core**: Rust is used to reduce typical implant instability and crash risks while still allowing controlled use of `unsafe` where necessary.
- **Cross-platform**: Windows and Linux implant targets; macOS controller operator.
- **Operator ergonomics**: modern CLI/terminal experience via Bun, with scriptable operations and composable tasks.

---

## 4. Threat Model and Red-Team Use Cases

RustyBuns is intended to emulate **post-exploitation tradecraft** similar to publicly discussed red-team tools and implants.

### 4.1 Core Use Cases

1. **Initial Foothold Validation**
   - Verify that a delivered payload can maintain a stable beacon under network and host controls.
   - Exercise detection rules for suspicious process trees, unusual child processes, and outbound beacons.

2. **Credential and Secret Access Simulation**
   - Simulate attempts to access sensitive material (e.g., environment files, configuration secrets, keystores) under strict scope controls.
   - Compare detections to MITRE ATT&CK techniques for credential dumping and secret discovery.

3. **Living-off-the-Land and Process Masquerading**
   - Generate process trees that resemble real threats (parent PID spoofing, LOLBins usage) while using Rust for core execution.

4. **Resilient C2 and Operator Workflow**
   - Provide a realistic C2 channel with jitter and multiple transports so blue teams can tune detections on beaconing behavior, protocol fingerprinting, and traffic patterns.

### 4.2 Out-of-Scope Activities

The following are explicitly out of scope for RustyBuns design:

- Long-term persistence frameworks with UEFI, firmware, or destructive disk modification.
- Automated exploitation of internet-facing systems outside a tightly defined test environment.
- Autonomous worm-like propagation.

---

## 5. Architecture Overview

### 5.1 Major Components

- **Rust Implant Core** (target-side binary)
  - Beaconing engine (HTTPS transport, encoding, jitter).
  - Task execution framework (modules for recon, collection, limited credential access).
  - Platform abstraction for Windows/Linux primitives.
  - Self-contained: no Bun dependency, no FFI to controller.

- **Bun Controller and UX Layer** (operator-side, macOS/Linux)
  - CLI for operator interaction.
  - Session manager: track implants by ID, hostname, tags, engagement scope.
  - Tasking API and scriptable scenarios in TypeScript.
  - PTY-based interactive shells using `Bun.Terminal` and `terminal` option in `Bun.spawn`.
  - Structured logging with redaction hooks.

- **FFI Bridge (Controller-Side Only)**
  - Bun side uses `bun:ffi` to call into Rust shared libraries for performance-sensitive local tasks (e.g., encryption helpers, decode/encode routines, protocol packing).
  - FFI is NEVER deployed to target implants.

### 5.2 Deployment Models

1. **Single-Operator Lab Mode**
   - Bun controller and Rust implant both run on a lab host or local VM to simulate compromise and test detections in an isolated environment.

2. **Client Red-Team Engagement**
   - Bun controller runs on a red-team infrastructure host.
   - Rust implant is deployed on authorized target endpoints.
   - All connectivity constrained to agreed C2 domains and IP ranges.

---

## 6. Rust Implant Core – Design

### 6.1 Language and Safety Strategy

Rust is the primary language for the implant, chosen for memory safety, performance, and strong ecosystem support for systems programming and cryptography.

Guidelines:

- Keep `unsafe` blocks minimal, encapsulated, and thoroughly reviewed following secure Rust guidelines that emphasize avoiding unnecessary unsafe, input validation, and strict ownership patterns.
- Use community-reviewed crates where possible, particularly for crypto, serialization, and cross-platform abstractions.

### 6.2 Core Modules

1. **Configuration and Bootstrap**
   - Static configuration: C2 endpoints, jitter interval, kill-switch, allowed capabilities.
   - Dynamic configuration via environment variables for lab vs live engagement modes.

2. **Transport Layer**
   - HTTPS beaconing with `reqwest`.
   - Simple framing and obfuscation (XOR or base64, not custom crypto).
   - Jitter configuration for beacon interval randomization.

3. **Task Dispatcher**
   - Queue-based execution of tasks received from the controller.
   - Sandboxed execution of higher-risk modules, controlled by configuration flags.

4. **Reconnaissance and Enumeration**
   - Host metadata (OS, users, processes, network configuration) within scope.
   - File-system enumeration respecting exclusion lists to avoid unnecessary data exposure.

5. **Credential and Secret Access (Scoped)**
   - Focused on techniques commonly used by offenders and documented in credential-dumping and red-team literature.
   - **Metadata-only mode**: confirms access paths without extracting or storing real credentials.
   - All credential access gated behind `CRED_ACCESS_ENABLED` config flag.

6. **Collection and Exfiltration Modules**
   - Controlled modules for collecting specific evidence types agreed with the client.
   - Strict size limits and rate limits to avoid disrupting systems or saturating networks.

7. **Self-Protection and Kill-Switch**
   - Built-in mechanism to shut down on receiving an authenticated kill command.
   - Auto-expiration based on deployment timestamp.
   - Safeguards to avoid repeated crash loops or uncontrolled resource usage.

---

## 7. Bun Controller and Operator Experience

### 7.1 Rationale for Bun

Bun is chosen for the controller because it combines a high-performance JS runtime, modern tooling, and robust FFI support to interface with Rust when needed.

Key advantages:

- Fast startup and low-latency scripts for operator workflows.
- `bun:ffi` for calling into compiled Rust helpers for crypto/encoding on the controller side.
- Built-in PTY support (`terminal` option in `Bun.spawn`) for creating interactive shells over the C2 channel — works on macOS and Linux controllers and for POSIX targets.

### 7.2 Controller Components

1. **Session Manager**
   - Tracks implants by ID, hostname, tags, and engagement scope.
   - Exposes commands to list, select, and tag sessions.

2. **Tasking Engine**
   - Sends commands to selected implants and tracks task status.
   - Composable tasks (e.g., structured sequences for initial recon, lateral-movement simulation, data collection within policy boundaries).

3. **Interactive Shell**
   - Uses Bun's PTY capabilities to provide a usable, interactive shell linked to the implant's command-execution module.
   - Supports basic history, resizing, and multiple concurrent shells per operator.
   - **Note**: PTY only works for POSIX targets (Linux/macOS). Windows targets receive streaming output via stdout pipe.

4. **Scripting and Automation Interface**
   - Operators can write TypeScript scripts that interact with the controller API for repeatable exercises.
   - Example: scripts to simulate a known TTP chain using tasks and shell commands for blue-team training.

5. **Logging and Evidence Handling**
   - Structured logs for commands issued, outputs, and timestamps.
   - Optional redaction hooks for sensitive content (e.g., patterns for secrets) to support safe report generation.

---

## 8. Rust–Bun FFI and Integration

### 8.1 FFI Design Principles

`bun:ffi` is experimental — this is acceptable for controller-side use only. FFI boundaries are never deployed to targets.

Design guidelines:

- Keep FFI surfaces **narrow and well-defined**: small, focused functions for cryptography, encoding, and packing/unpacking messages.
- Prefer stable, fixed-size types (e.g., 32-bit and 64-bit integers, pointers, byte slices) to minimize ABI issues.
- Handle input validation on the Rust side to avoid malformed data triggering undefined behavior.
- Fall back to pure-Rust crypto if FFI proves unstable — no hard dependency on FFI for implant operation.

### 8.2 FFI Integration Pattern (Optional, Phase 1+)

- Rust exposes a function `pack_message` that takes a pointer to a buffer and length, returning an encoded message.
- Bun controller imports `pack_message` via `dlopen` and `FFIType` definitions, then uses it for high-throughput message processing.
- This is optional — implant operates independently via HTTPS + JSON even without FFI active.

---

## 9. Security Properties and Defensive Considerations

### 9.1 Secure Coding Practices

RustyBuns must adhere to secure Rust guidelines:

- Minimize `unsafe` and isolate it in well-reviewed modules.
- Perform thorough input validation for all external data, including C2 messages, environment configuration, and filesystem paths.
- Use community-audited crates for cryptography and avoid bespoke crypto.

### 9.2 Detection Surface and Blue-Team Value

While RustyBuns is designed to emulate sophisticated threats, it should **not** aim for unbounded stealth. Instead, it should:

- Expose realistic artifacts for blue teams to detect: process trees, network connections, scheduled tasks, memory patterns.
- Provide configuration presets that map to common MITRE ATT&CK techniques so blue teams can test coverage.
- Offer logging modes that capture what defenders should see during an engagement (e.g., Windows events, syslog entries) to aid in post-exercise analysis.

### 9.3 Safety Controls

- Global configuration flags to disable certain capabilities (e.g., credential access, destructive operations) per client or lab.
- Kill-switch mechanisms and time-bounded operation (auto-expiration) to avoid implants lingering beyond the engagement.

---

## 10. Research Findings (Build Decisions)

### 10.1 Bun FFI Status

`bun:ffi` is **experimental** per official Bun documentation. However, since FFI only runs on the operator's controller machine (not on targets), this is an acceptable risk for a self-hosted tool. The implant itself never uses FFI.

### 10.2 Bun PTY Limitation

Bun PTY is **POSIX-only** (Linux/macOS). Windows targets will have non-interactive shell output via stdout pipe — no terminal features. This is an accepted architectural gap.

### 10.3 Bun Embedding

Bun cannot be embedded inside a Rust binary. The implant is a standalone Rust binary. Bun only runs on the controller machine. This changes the architecture from what was originally described — the implant does not "include Bun."

### 10.4 Recommended FFI Approach for Phase 0

Start without FFI. Use pure-Rust crypto (ring or aes-gcm crate) in the implant and standard library in Bun. FFI can be introduced in Phase 1 if a performance need is identified.

### 10.5 Rust Crates (Key Choices)

- **HTTPS C2 client**: `reqwest` with native-tls or rustls
- **Serialization**: `serde` + `serde_json`
- **Async runtime**: `tokio` (used by reqwest, good ecosystem support)
- **Crypto (Phase 1+)**: `ring` for AES-GCM, `chacha20poly1305` for ChaCha20
- **Windows API bindings**: `windows-rs`
- **Process enumeration Linux**: `/proc` filesystem via standard library

---

## 11. Build Roadmap

### Phase 0 — Project Scaffold & Lab Prototype
**Goal**: Working implant + controller on the same machine. No FFI. No credential access. Lab-only.

#### 0.1 Project Structure
```
RustyBuns/
├── SPEC.md                    (this file)
├── implant/                   (Rust crate)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── controller/                (Bun TypeScript project)
│   ├── package.json
│   ├── tsconfig.json
│   ├── src/
│   │   ├── index.ts           (CLI entry)
│   │   ├── session.ts         (session manager)
│   │   └── task.ts           (tasking engine)
│   └── scripts/               (operator scripts)
└── docs/                      (architecture, operator guide)
```

#### 0.2 Rust Implant — Core Features
- [ ] `implant/Cargo.toml`: `tokio`, `reqwest`, `serde`, `serde_json`, `tracing`
- [ ] HTTPS C2 client: beacon to `https://<controller>:8080`, register on start
- [ ] Task executor: `whoami`, `hostname`, `ps` (process list), `ls <path>` — all benign
- [ ] Kill-switch: `exit` command received from controller halts implant
- [ ] Config: hardcoded C2 endpoint, jitter 5-15s, auto-expire after 8h
- [ ] Build: `cargo build --release` → static binary, minimal dependencies

#### 0.3 Bun Controller — Core Features
- [ ] `controller/package.json`: bun, typescript, `@types/node`
- [ ] `controller/tsconfig.json`: strict mode
- [ ] Session manager: `sessions` array, track ID, hostname, last seen, tags
- [ ] Tasking: send task to implant by session ID, await response, print
- [ ] IPC listener: `Bun.spawn` with IPC handler to receive implant check-ins
- [ ] Commands: `list`, `select <id>`, `task <cmd>`, `exit`
- [ ] HTTPS listener: `Bun.serve` on port 8080 to receive implant beacons

#### 0.4 Testing
- [ ] Run implant and controller on same Mac (lab mode)
- [ ] Implant registers → appears in session list
- [ ] Task command returns output from implant
- [ ] Kill-switch stops implant cleanly

---

### Phase 1 — Operator Ergonomics and Transport Layer
**Goal**: Production-ready transport, PTY shells on POSIX, structured logging.

#### 1.1 Transport Refinement
- [ ] Implant: proper JSON message framing (task request / response envelope)
- [ ] Retry logic with exponential backoff on failed beacons
- [ ] Jitter: random interval between beacons, configurable
- [ ] Controller: HTTPS server with TLS (self-signed cert for lab, proper cert for engagements)

#### 1.2 Bun PTY Shell
- [ ] Controller: `Bun.spawn` with `terminal:` option for POSIX targets
- [ ] Interactive command loop: `shell <session-id>`
- [ ] PTY data forwarded between operator terminal and implant stdin/stdout
- [ ] Windows fallback: stream-based shell without PTY (output only)
- [ ] Shell resize support: forward terminal resize to implant

#### 1.3 Logging and Evidence
- [ ] Structured JSON logs: timestamp, session ID, command, output, duration
- [ ] Redaction hooks: patterns for secrets (AWS keys, passwords, tokens) replaced with `[REDACTED]`
- [ ] Log file rotation: daily files, configurable retention
- [ ] Export: JSONL format suitable for SIEM ingestion

#### 1.4 Session Manager Enhancements
- [ ] Tag sessions: `tag <id> <label>`, filter by tag
- [ ] Session metadata: OS, username, hostname stored at registration
- [ ] Auto-expire sessions: mark stale if no beacon > 2x jitter interval

---

### Phase 2 — Cross-Platform Implant Core
**Goal**: Reliable implant on both Linux and Windows targets.

#### 2.1 Windows Support
- [ ] `windows-rs` for Win32 APIs: process enumeration, token manipulation
- [ ] Implant: detect target OS at runtime, route to Windows-specific code
- [ ] Tasks: `ps` (Windows process list via Win32), `whoami` / `whoami /all`, registry read
- [ ] Parent PID spoofing pattern (documented, not stealth-focused)

#### 2.2 Linux Support
- [ ] `/proc` enumeration: process list, user info, network connections
- [ ] Tasks: `ps aux`, `whoami`, `id`, `netstat` equivalent
- [ ] Support for multiple distributions (tested on Ubuntu, Debian, Arch)

#### 2.3 Unified Task Interface
- [ ] Task enum: `Whoami | Hostname | ProcessList | Shell(cmd) | LS(path) | ...`
- [ ] OS abstraction layer: `trait Platform` with Windows/Linux implementations
- [ ] Task result: `Result<String, TaskError>` — consistent error handling

#### 2.4 Transport Pluggability
- [ ] Transport trait: `trait Transport { async fn send(&self, msg: Vec<u8>) -> Vec<u8>; async fn recv(&self) -> Vec<u8>; }`
- [ ] HTTPS transport (default)
- [ ] (Stretch) DNS transport: TXT record exfil for very constrained environments

---

### Phase 3 — Scoped Offensive Capabilities
**Goal**: Credential access simulation, MITRE ATT&CK mapping, configuration profiles.

#### 3.1 Credential Access Simulation
- [ ] Metadata-only mode: list credential stores (Windows: LSASS, Sam, Vault; Linux: /etc/shadow, ~/.ssh, keyring) but do NOT extract or display actual secrets
- [ ] `cred-access-check`: confirm access paths exist and are readable, log result without exfil
- [ ] All credential access gated behind `CRED_ACCESS_ENABLED=true` env flag
- [ ] In metadata-only mode: output shows "path accessible" not "secret value"

#### 3.2 MITRE ATT&CK Mapping
- [ ] Task tags: each task mapped to ATT&CK technique ID (e.g., `T1003.001` for credential dumping)
- [ ] Config profiles: "enterprise-workstation", "server", "domain-controller"
- [ ] Profile enables/disables techniques based on engagement scope

#### 3.3 File Collection (Bounded)
- [ ] Collect: system info, running processes, network connections, scheduled tasks
- [ ] Size limit: max 1MB per collection, 10MB total per session
- [ ] Rate limit: max 1 collection per 30 seconds to avoid network saturation

#### 3.4 Scope Enforcement
- [ ] Config: allowed CIDRs, blocked processes (antivirus), max data exfil
- [ ] Implant refuses tasks outside scope with error code + log entry

---

### Phase 4 — Blue-Team Integration and Hardening
**Goal**: Telemetry export, safety review, open-source prep.

#### 4.1 Telemetry Export
- [ ] Structured event log: implant events (task received, executed, completed, failed) → JSONL
- [ ] SIEM format: align with Common Information Model (CIM) fields
- [ ] Export endpoint: write to file, or POST to SIEM intake endpoint

#### 4.2 Safety Features
- [ ] Global kill-switch: `shutdown` command from controller halts all implants
- [ ] Time-bounded operation: implant expires at `EXPIRY_TIMESTAMP`, self-terminates
- [ ] Crash loop protection: if implant fails 3 times in 60s, halt and log
- [ ] Scope-based capability toggling: per-engagement flag file

#### 4.3 Security Audit
- [ ] Review all `unsafe` blocks: document rationale, add safety comments
- [ ] FFI boundary review: validate all pointer arithmetic, buffer bounds
- [ ] Fuzzing: test message parsing on implant with malformed inputs

#### 4.4 Open Source Prep
- [ ] Documentation: README, architecture overview, operator guide, ethical framework
- [ ] License: AGPLv3 (ensure contributors agree with copyleft) or Apache 2.0
- [ ] CI: build matrix for Linux (x64, ARM), macOS (x64, ARM), Windows (x64)
- [ ] Release: GitHub releases with pre-built implant binaries + controller

---

## 12. Summary

RustyBuns is specified as a **compact, cross-platform red-team tool** that leverages Rust's safety and performance for an implant core and Bun's modern runtime for an operator-friendly controller.

It is designed within a clear ethical and legal framework, with explicit safety controls and a focus on realistic but bounded threat simulation.

The architecture uses Bun **only on the controller** (operator side) and **only Rust on the implant** (target side). FFI is a controller-side option, not a target-side dependency.

The phased build plan starts with a working lab prototype and builds toward a production-grade, open-source tool with MITRE ATT&CK mapping, structured telemetry, and proper safety controls.

---

## Appendix A: Bun FFI Quick Reference

For Bun controller-side FFI when needed (Phase 1+):

```rust
// Rust cdylib export example
#[no_mangle]
pub extern "C" fn pack_message(ptr: *const u8, len: usize) -> u64 {
    // pointer to buffer, length → encode
}
```

```typescript
import { dlopen, FFIType, suffix } from "bun:ffi";

const lib = dlopen(`./target/release/libimplant_helpers.${suffix}`, {
  pack_message: {
    args: [FFIType.ptr, FFIType.u64],
    returns: FFIType.u64,
  },
});
```

FFI types: `i32`, `i64`, `u8`, `u64`, `ptr`, `buffer`, `cstring`. Buffers use `TypedArray`. Async not yet supported.

---

## Appendix B: Bun PTY Quick Reference

```typescript
const proc = Bun.spawn(["bash"], {
  terminal: {
    cols: 80,
    rows: 24,
    data(term, data) {
      process.stdout.write(data);
    },
  },
});

proc.terminal.write("echo hello\n");
proc.terminal.resize(120, 40);
await proc.exited;
proc.terminal.close();
```

PTY requires POSIX (Linux/macOS). Not available on Windows.