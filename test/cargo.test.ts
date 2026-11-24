import { expect, test } from "bun:test";
import { chmodSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { cargoSaid, rustcShim, watched } from "../tooling/cargo.js";
import { fixture, root } from "./support";
import { underLoad } from "../scripts/timing";

// Every Cargo build with one compiler runs the same rustc shim, and each
// writes it again. Rewritten in place while another's Cargo runs it, that
// run fails, Linux's ETXTBSY, as the parallel suite's Cargo and Vite tests
// did. Written whole and moved into place, a running one keeps its file.
test("the rustc shim is rewritten while other builds run it, and none fails", () => {
  const dir = fixture("rustc shim");
  const compiler = join(dir, "rust-js");
  writeFileSync(compiler, "#!/bin/sh\nexit 0\n");
  chmodSync(compiler, 0o755);
  const shim = rustcShim(compiler);
  const writer = `import { rustcShim } from ${JSON.stringify(join(root, "tooling", "cargo.js"))};
// One rewriting in place can itself be refused, the file being run: on.
for (;;) {
  try {
    rustcShim(${JSON.stringify(compiler)});
  } catch {}
}`;
  const writers = [1, 2, 3].map(() => Bun.spawn([process.execPath, "-e", writer], { stdout: "ignore", stderr: "ignore" }));
  const failed: string[] = [];
  try {
    const until = Date.now() + 2000;
    while (Date.now() < until) {
      const p = Bun.spawnSync([shim, "--version"], { stdout: "ignore", stderr: "pipe" });
      if (p.exitCode !== 0) failed.push(p.stderr.toString() || `exit ${p.exitCode}`);
    }
  } catch (error) {
    failed.push(String(error));
  } finally {
    for (const w of writers) w.kill();
  }
  expect(failed.slice(0, 3)).toEqual([]);
}, underLoad(60_000));

// A Cargo command that failed saying nothing, `cargo metadata` under load
// in test/manifest.test.ts, says how it ended: killed by a signal, or not
// spawned at all, which its stderr never has.
test("a Cargo command that failed with nothing on stderr says how it ended", () => {
  expect(cargoSaid({ stderr: "error: no such package\n", message: "Command failed: cargo metadata" })).toBe("error: no such package\n");
  expect(cargoSaid({ stderr: "", message: "Command failed: cargo metadata", signal: "SIGKILL", code: null })).toBe("Command failed: cargo metadata, killed by SIGKILL");
  expect(cargoSaid({ stderr: "", message: "spawn cargo EAGAIN", code: "EAGAIN" })).toBe("spawn cargo EAGAIN");
});

// A command tooling runs that's past the suite's notice says which it is
// and how far it got, where a test that hung at its timeout said nothing
// (#2441): still running, or exited with its `close` never come.
test("a command past its notice says which it is, and whether it exited", async () => {
  const notices = async (child: { pid: number; exitCode: number | null; signalCode: string | null }, ends: boolean) => {
    const said: string[] = [];
    let end = () => {};
    const done = new Promise<void>((resolve) => (end = resolve));
    const watching = watched(done, child as never, "cargo", ["metadata", "--manifest-path", "a/Cargo.toml"], (line) => said.push(line), { RUST_JS_COMMAND_NOTICE_MS: "20" });
    await Bun.sleep(60);
    end();
    await watching;
    if (ends) {
      // One that ends first says nothing.
      const quiet: string[] = [];
      await watched(Promise.resolve(), child as never, "cargo", [], (line) => quiet.push(line), { RUST_JS_COMMAND_NOTICE_MS: "20" });
      await Bun.sleep(40);
      expect(quiet).toEqual([]);
    }
    return said;
  };
  expect(await notices({ pid: 7, exitCode: null, signalCode: null }, true)).toEqual(["rust-js: `cargo metadata --manifest-path a/Cargo.toml` (pid 7) unfinished after 20 ms: still running"]);
  expect(await notices({ pid: 8, exitCode: 0, signalCode: null }, false)).toEqual(["rust-js: `cargo metadata --manifest-path a/Cargo.toml` (pid 8) unfinished after 20 ms: it exited (0) and its `close` hasn't come"]);
  // Unset, none.
  const unset: string[] = [];
  let end = () => {};
  const done = watched(new Promise<void>((resolve) => (end = resolve)), { pid: 9, exitCode: null, signalCode: null } as never, "cargo", [], (line) => unset.push(line), {});
  await Bun.sleep(40);
  end();
  await done;
  expect(unset).toEqual([]);
});
