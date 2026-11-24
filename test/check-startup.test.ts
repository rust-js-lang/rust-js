import { expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { startup } from "../scripts/checkFast";
import { underLoad } from "../scripts/timing";
import { fixture } from "./support";

const above = () => false;

// The build waits for the pruner to finish before doing its remaining work.
// A synchronous build used to prevent the parent observing that fast exit:
// both durations included the build, even though the prune had already ended.
test("startup observes a fast prune while the compiler is still building", async () => {
  const dir = fixture("startup-timing");
  const tail = underLoad(400);
  const commands = {
    prune: [process.execPath, "-e", 'await Bun.write("pruned", "done");'],
    // Waiting for the prune a while at most, so a start that ran it only
    // after the build leaves no build waiting forever, and says so.
    build: [process.execPath, "-e", 'const until = Date.now() + '+underLoad(5_000)+'; while (!(await Bun.file("pruned").exists()) && Date.now() < until) await Bun.sleep(5); const seen = await Bun.file("pruned").exists(); await Bun.sleep('+tail+'); await Bun.write("built", seen ? "done" : "no prune beside it");'],
  };
  const pending = startup(dir, commands, above);
  expect(existsSync(join(dir, "built"))).toBe(false);
  const result = await pending;
  expect(result.build.code).toBe(0);
  const prune = await result.prune;
  expect(prune.code).toBe(0);
  expect(prune.seconds).toBeLessThan(result.build.seconds - tail / 2000);
  expect(readFileSync(join(dir, "built"), "utf8")).toBe("done");
});

// The jobs need the build, not the prune: a check's start waited on the
// prune's walk of the stores, 25.7 s of every check on the M1 (#2498). A
// prune that ends only once the jobs have begun, as the test says they
// have, ends beside them; a start that waited for it would wait forever,
// and the test's deadline says so.
test("startup ends with the build, the prune going on beside the jobs", async () => {
  const dir = fixture("startup-beside");
  const commands = {
    prune: [process.execPath, "-e", 'while (!(await Bun.file("jobs").exists())) await Bun.sleep(5); console.log("pruned beside");'],
    build: [process.execPath, "-e", 'await Bun.write("built", "done");'],
  };
  const deadline = new Promise<"waited on the prune">((done) => setTimeout(() => done("waited on the prune"), underLoad(5_000)));
  const started = startup(dir, commands, above);
  const result = await Promise.race([started, deadline]);
  writeFileSync(join(dir, "jobs"), "begun");
  expect(result).not.toBe("waited on the prune");
  if (result === "waited on the prune") return;
  expect(result.build.code).toBe(0);
  expect(result.waited).toBe(false);
  const prune = await result.prune;
  expect(prune.code).toBe(0);
  expect(prune.stdout).toBe("pruned beside\n");
});

// Below the disk's floor the jobs would keep nothing they make: they wait
// for what the prune frees.
test("startup below the disk's floor waits for the prune", async () => {
  const dir = fixture("startup-below-floor");
  const commands = {
    prune: [process.execPath, "-e", 'await Bun.sleep(50); await Bun.write("pruned", "done");'],
    build: [process.execPath, "-e", 'await Bun.write("built", "done");'],
  };
  const result = await startup(dir, commands, () => true);
  expect(result.waited).toBe(true);
  expect(existsSync(join(dir, "pruned"))).toBe(true);
  expect((await result.prune).code).toBe(0);
});

for (const failing of ["build", "prune"] as const) {
  test("startup retains the " + failing + " failure and waits for both children", async () => {
    const dir = fixture("startup-failure-" + failing);
    const command = (name: string) => [process.execPath, "-e", 'await Bun.sleep(25); await Bun.write("'+name+'", "done"); process.exit('+(name===failing ? 7 : 0)+');'];
    const result = await startup(dir, { build: command("build"), prune: command("prune") }, above);
    const prune = await result.prune;
    expect((failing === "build" ? result.build : prune).code).toBe(7);
    expect((failing === "build" ? prune : result.build).code).toBe(0);
    expect(existsSync(join(dir,"build"))).toBe(true);
    expect(existsSync(join(dir,"prune"))).toBe(true);
  });
}

// A build that failed stops the check: it waits for the prune first, so
// nothing of the check's is left running.
test("startup waits for the prune where the build failed", async () => {
  const dir = fixture("startup-build-failed");
  const commands = {
    prune: [process.execPath, "-e", 'await Bun.sleep(50); await Bun.write("pruned", "done");'],
    build: [process.execPath, "-e", "process.exit(3);"],
  };
  const result = await startup(dir, commands, above);
  expect(result.build.code).toBe(3);
  expect(result.waited).toBe(true);
  expect(existsSync(join(dir, "pruned"))).toBe(true);
});
