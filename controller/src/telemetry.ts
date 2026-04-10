const TELEMETRY_PATH = process.env.TELEMETRY_PATH ?? "./telemetry.jsonl";

interface TelemetryEvent {
  "@timestamp": string;
  event_type: string;
  session_id?: string;
  hostname?: string;
  username?: string;
  os?: string;
  command?: string;
  mitre_id?: string;
  technique?: string;
  task_id?: string;
  success?: boolean;
  duration_ms?: number;
  output_length?: number;
  error?: string;
  level: "INFO" | "WARN" | "ERROR";
  [key: string]: unknown;
}

function writeTelemetry(event: TelemetryEvent): void {
  try {
    const line = JSON.stringify(event) + "\n";
    Bun.write(Bun.file(TELEMETRY_PATH), line, { flag: "a" });
  } catch {
  }
}

export function logTelemetrySessionRegister(session: {
  id: string;
  hostname: string;
  username: string;
  os: string;
  version: string;
}): void {
  writeTelemetry({
    "@timestamp": new Date().toISOString(),
    event_type: "session_register",
    session_id: session.id,
    hostname: session.hostname,
    username: session.username,
    os: session.os,
    level: "INFO",
  });
}

export function logTelemetryTaskResult(result: {
  task_id: string;
  session_id?: string;
  command: string;
  mitre_id?: string;
  technique?: string;
  success: boolean;
  duration_ms: number;
  output_length?: number;
  error?: string;
}): void {
  writeTelemetry({
    "@timestamp": new Date().toISOString(),
    event_type: result.success ? "task_completed" : "task_failed",
    session_id: result.session_id,
    task_id: result.task_id,
    command: result.command,
    mitre_id: result.mitre_id,
    technique: result.technique,
    success: result.success,
    duration_ms: result.duration_ms,
    output_length: result.output_length,
    error: result.error,
    level: result.success ? "INFO" : "ERROR",
  });
}

export function logTelemetryShutdown(sessionId: string): void {
  writeTelemetry({
    "@timestamp": new Date().toISOString(),
    event_type: "global_shutdown",
    session_id: sessionId,
    level: "WARN",
  });
}

export function logTelemetrySessionStale(session: {
  id: string;
  hostname: string;
  username: string;
}): void {
  writeTelemetry({
    "@timestamp": new Date().toISOString(),
    event_type: "session_stale",
    session_id: session.id,
    hostname: session.hostname,
    username: session.username,
    level: "WARN",
  });
}
