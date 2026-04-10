import { saveSession, loadSessions, removeSession as dbRemoveSession } from "./db.ts";

export interface Session {
  id: string;
  hostname: string;
  username: string;
  os: string;
  version: string;
  lastSeen: number;
  tags: string[];
  expiryHours: number;
}

export class SessionManager {
  private sessions: Map<string, Session> = new Map();
  private staleCheckInterval: ReturnType<typeof setInterval> | null = null;
  private readonly DEFAULT_JITTER_MIN = 5;
  private readonly DEFAULT_JITTER_MAX = 15;
  private readonly STALE_MULTIPLIER = 3;

  constructor() {
    const persisted = loadSessions();
    for (const s of persisted) {
      this.sessions.set(s.id, s);
    }
  }

  register(payload: {
    uuid: string;
    hostname: string;
    username: string;
    os: string;
    version: string;
    expiry_hours: number;
  }): Session {
    const existing = this.sessions.get(payload.uuid);
    if (existing) {
      existing.lastSeen = Date.now();
      saveSession(existing);
      return existing;
    }

    const session: Session = {
      id: payload.uuid,
      hostname: payload.hostname,
      username: payload.username,
      os: payload.os,
      version: payload.version,
      lastSeen: Date.now(),
      tags: [],
      expiryHours: payload.expiry_hours,
    };

    this.sessions.set(payload.uuid, session);
    saveSession(session);
    console.log(`[+] New session: ${session.id} (${session.username}@${session.hostname} / ${session.os})`);
    return session;
  }

  startStaleCheck(intervalMs = 60000, onStale?: (session: Session) => void): void {
    if (this.staleCheckInterval) {
      clearInterval(this.staleCheckInterval);
    }
    this.staleCheckInterval = setInterval(() => {
      const staleIds = this.stale();
      for (const id of staleIds) {
        const session = this.sessions.get(id);
        if (session) {
          console.log(`[!] Removing stale session: ${id} (${session.username}@${session.hostname})`);
          if (onStale) onStale(session);
          this.sessions.delete(id);
          dbRemoveSession(id);
        }
      }
    }, intervalMs);
    console.log(`[*] Stale session check started (interval: ${intervalMs}ms)`);
  }

  stopStaleCheck(): void {
    if (this.staleCheckInterval) {
      clearInterval(this.staleCheckInterval);
      this.staleCheckInterval = null;
    }
  }

  touch(id: string): void {
    const session = this.sessions.get(id);
    if (session) {
      session.lastSeen = Date.now();
    }
  }

  get(id: string): Session | undefined {
    return this.sessions.get(id);
  }

  getAll(): Session[] {
    return Array.from(this.sessions.values());
  }

  list(): void {
    const sessions = this.getAll();
    if (sessions.length === 0) {
      console.log("No active sessions.");
      return;
    }

    console.log(`\nActive Sessions (${sessions.length}):`);
    console.log("─".repeat(80));
    console.log("ID".padEnd(38) + "USER".padEnd(16) + "HOST".padEnd(20) + "OS".padEnd(10) + "LAST SEEN");
    console.log("─".repeat(80));

    for (const s of sessions) {
      const lastSeen = new Date(s.lastSeen).toLocaleTimeString();
      console.log(
        s.id.substring(0, 36).padEnd(38) +
        s.username.substring(0, 14).padEnd(16) +
        s.hostname.substring(0, 18).padEnd(20) +
        s.os.padEnd(10) +
        lastSeen
      );
    }
    console.log("");
  }

  tag(id: string, label: string): boolean {
    const session = this.sessions.get(id);
    if (!session) {
      console.error(`Session ${id} not found.`);
      return false;
    }
    session.tags.push(label);
    console.log(`Tagged session ${id} with: ${label}`);
    return true;
  }

  select(id: string): Session | undefined {
    return this.sessions.get(id);
  }

  stale(): string[] {
    const stale: string[] = [];
    const jitterRange = this.DEFAULT_JITTER_MAX - this.DEFAULT_JITTER_MIN + 1;
    const checkInterval = 2 * (this.DEFAULT_JITTER_MIN + Math.floor(Math.random() * jitterRange)) * 1000;
    const staleThreshold = checkInterval * this.STALE_MULTIPLIER;
    const now = Date.now();
    for (const [id, session] of this.sessions) {
      if (now - session.lastSeen > staleThreshold) {
        stale.push(id);
      }
    }
    return stale;
  }

  remove(id: string): void {
    this.sessions.delete(id);
  }
}