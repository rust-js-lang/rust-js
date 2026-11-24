// A server a test starts, on a port no other process has: suites run at
// once, and a port probed free, `listen(0)` then closed, and handed to a
// server, was taken between the two by another suite's probe (lily agent's
// push: a Next test's EADDRINUSE on 55158, passing alone on rerun).
import type { ChildProcess } from "node:child_process";
import { createServer } from "node:net";

/** A server started and ready, the port it listens on, and all it has said
 * so far, read on. */
export type Served = { server: ChildProcess; port: number; output: () => string };

/** Next.js's line once it listens, the port it bound: `- Local:
 * http://localhost:51234`, then `Ready`. */
export const nextReady = /Local:\s+https?:\/\/[^\s:]+:(\d+)[\s\S]*Ready/;

/** `start` of a server, ready once what it says matches `ready`, whose
 * first group is the port it listens on. Where the server binds the port
 * it's given, `0` is the system's choice, said back, as Next.js's and
 * Vite's are, and nothing races (`bound`, the default). One that says only
 * the port it was given, a program's own server, is given one probed free
 * and started again on another where it says `EADDRINUSE` before it's
 * ready, a few times. A server that ends before it's ready fails with what
 * it said. */
export async function served(start: (port: number) => ChildProcess, ready: RegExp, { bound = true } = {}): Promise<Served> {
  for (let tries = bound ? 1 : 5; ; tries--) {
    const server = start(bound ? 0 : await probed());
    let said = "";
    server.stdout?.on("data", (data) => (said += data));
    server.stderr?.on("data", (data) => (said += data));
    for (;;) {
      const matched = said.match(ready);
      if (matched) return { server, port: Number(matched[1]), output: () => said };
      if (server.exitCode !== null || server.signalCode !== null) break;
      await Bun.sleep(100);
    }
    if (tries > 1 && said.includes("EADDRINUSE")) continue;
    throw new Error(`the server ended before it was ready, ${server.exitCode ?? server.signalCode}:\n${said}`);
  }
}

/** A port no process listens on now, the system's choice of its ephemeral
 * ones, past each that fetch refuses as reserved (3659, 6000, ..): a random
 * one of a range met another app's, 7265 (#1269's check). */
function probed(): Promise<number> {
  return new Promise((done, failed) => {
    const probe = createServer();
    probe.on("error", failed);
    probe.listen(0, () => {
      const { port } = probe.address() as { port: number };
      probe.close(() => done(port));
    });
  });
}
