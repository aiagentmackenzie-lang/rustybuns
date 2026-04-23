import { Database } from "bun:sqlite";
import type { Session } from "./session.ts";

const DB_PATH = process.env.DB_PATH ?? "./rustybuns.db";

let db: Database | null = null;

export function getDb(): Database {
  if (!db) {
    const resolved = DB_PATH.startsWith("/") ? DB_PATH : `${process.cwd()}/${DB_PATH}`;
    const dir = resolved.substring(0, resolved.lastIndexOf("/"));
    if (dir && !Bun.file(dir).exists()) {
      try {
        const parts = dir.split("/").filter(Boolean);
        let built = "/";
        for (const part of parts) {
          built += part + "/";
          try { Bun.mkdir(built); } catch {}
        }
      } catch {}
    }
    db = new Database(DB_PATH);
    initSchema();
  }
  return db;
}

export function initSchema() {
  if (!db) return;
  db.run(`
    CREATE TABLE IF NOT EXISTS sessions (
      id TEXT PRIMARY KEY,
      hostname TEXT NOT NULL,
      username TEXT NOT NULL,
      os TEXT NOT NULL,
      version TEXT NOT NULL,
      last_seen INTEGER NOT NULL,
      tags TEXT NOT NULL DEFAULT '[]',
      expiry_hours INTEGER NOT NULL,
      created_at INTEGER NOT NULL
    )
  `);
  db.run(`
    CREATE TABLE IF NOT EXISTS tasks (
      id TEXT PRIMARY KEY,
      session_id TEXT NOT NULL,
      command TEXT NOT NULL,
      args TEXT,
      status TEXT NOT NULL DEFAULT 'pending',
      result TEXT,
      mitre_id TEXT,
      technique TEXT,
      created_at INTEGER NOT NULL,
      completed_at INTEGER,
      FOREIGN KEY (session_id) REFERENCES sessions(id)
    )
  `);
  db.run(`
    CREATE TABLE IF NOT EXISTS config (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    )
  `);
}

export function saveSession(session: Session): void {
  const db = getDb();
  const stmt = db.prepare(`
    INSERT OR REPLACE INTO sessions (id, hostname, username, os, version, last_seen, tags, expiry_hours, created_at)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);
  stmt.run(
    session.id,
    session.hostname,
    session.username,
    session.os,
    session.version,
    session.lastSeen,
    JSON.stringify(session.tags),
    session.expiryHours,
    Date.now()
  );
}

export function loadSessions(): Session[] {
  const db = getDb();
  const rows = db.query("SELECT * FROM sessions").all() as any[];
  return rows.map((row) => ({
    id: row.id,
    hostname: row.hostname,
    username: row.username,
    os: row.os,
    version: row.version,
    lastSeen: row.last_seen,
    tags: JSON.parse(row.tags),
    expiryHours: row.expiry_hours,
  }));
}

export function removeSession(id: string): void {
  const db = getDb();
  db.run("DELETE FROM sessions WHERE id = ?", id);
  db.run("DELETE FROM tasks WHERE session_id = ?", id);
}

export function savePendingTasks(sessionId: string, tasks: { id: string; command: string; args?: string[] }[]): void {
  const db = getDb();
  db.run("DELETE FROM tasks WHERE session_id = ? AND status = 'pending'", sessionId);
  for (const task of tasks) {
    const stmt = db.prepare(`
      INSERT OR REPLACE INTO tasks (id, session_id, command, args, status, created_at)
      VALUES (?, ?, ?, ?, 'pending', ?)
    `);
    stmt.run(task.id, sessionId, task.command, JSON.stringify(task.args ?? []), Date.now());
  }
}

export function loadPendingTasks(sessionId: string): { id: string; command: string; args?: string[] }[] {
  const db = getDb();
  const rows = db.query("SELECT * FROM tasks WHERE session_id = ? AND status = 'pending'", sessionId).all() as any[];
  return rows.map((row) => ({
    id: row.id,
    command: row.command,
    args: JSON.parse(row.args),
  }));
}

export function completeTask(taskId: string, result: string, mitreId?: string, technique?: string): void {
  const db = getDb();
  db.run("UPDATE tasks SET status = 'completed', result = ?, completed_at = ?, mitre_id = ?, technique = ? WHERE id = ?", result, Date.now(), mitreId ?? null, technique ?? null, taskId);
}

export function saveConfig(key: string, value: string): void {
  const db = getDb();
  db.run("INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)", key, value);
}

export function loadConfig(key: string): string | undefined {
  const db = getDb();
  const row = db.query("SELECT value FROM config WHERE key = ?", key).get() as { value: string } | undefined;
  return row?.value;
}

export function getAllPendingTasks(): Map<string, { id: string; command: string; args?: string[] }[]> {
  const db = getDb();
  const rows = db.query("SELECT id, session_id, command, args FROM tasks WHERE status = 'pending'").all() as any[];
  const map = new Map<string, { id: string; command: string; args?: string[] }[]>();
  for (const row of rows) {
    const sessionId = row.session_id as string;
    const list = map.get(sessionId) ?? [];
    list.push({ id: row.id, command: row.command, args: JSON.parse(row.args) });
    map.set(sessionId, list);
  }
  return map;
}

export function deletePendingTasks(sessionId: string): void {
  const db = getDb();
  db.run("DELETE FROM tasks WHERE session_id = ? AND status = 'pending'", sessionId);
}