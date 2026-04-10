import type { Session } from "./session.ts";
import { completeTask } from "./db.ts";

export interface Task {
  id: string;
  command: string;
  args?: string[];
}

export interface TaskResult {
  task_id: string;
  success: boolean;
  output: string;
  error?: string;
  duration_ms: number;
  mitre_id?: string;
  technique?: string;
}

export class TaskEngine {
  private pending: Map<string, { command: string; args?: string[]; createdAt: number }> = new Map();
  private results: Map<string, TaskResult> = new Map();

  createTask(command: string, args?: string[]): Task {
    const id = crypto.randomUUID();
    this.pending.set(id, { command, args, createdAt: Date.now() });
    return { id, command, args };
  }

  getPending(id: string) {
    return this.pending.get(id);
  }

  storeResult(result: TaskResult): void {
    this.pending.delete(result.task_id);
    this.results.set(result.task_id, result);
    const mitreStr = result.mitre_id ? ` [${result.mitre_id}]` : "";
    const techStr = result.technique ? ` (${result.technique})` : "";
    console.log(`[~] Result for task ${result.task_id}: ${result.success ? "OK" : "FAIL"} (${result.duration_ms}ms)${mitreStr}${techStr}`);
    if (!result.success && result.error) {
      console.error(`    Error: ${result.error}`);
    }
    if (result.success && result.output) {
      const lines = result.output.split("\n").slice(0, 3);
      for (const line of lines) {
        console.log(`    ${line}`);
      }
      if (result.output.split("\n").length > 3) {
        console.log(`    ... (${result.output.split("\n").length} lines total)`);
      }
    }
    completeTask(result.task_id, result.output, result.mitre_id, result.technique);
  }

  getResult(id: string): TaskResult | undefined {
    return this.results.get(id);
  }

  async waitForResult(id: string, timeoutMs = 30000): Promise<TaskResult | null> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      const result = this.results.get(id);
      if (result) return result;
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    return null;
  }
}

export const taskEngine = new TaskEngine();