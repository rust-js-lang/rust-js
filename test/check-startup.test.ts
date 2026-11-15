import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { startup } from "../scripts/checkFast";
import { underLoad } from "../scripts/timing";
import { fixture } from "./support";

// The build waits for the pruner to finish before doing its remaining work.
// A synchronous build used to prevent the parent observing that fast exit:
// both durations included the build, even though the prune had already ended.
test("startup observes a fast prune while the compiler is still building", async () => {
  const dir = fixture("startup-timing");
  const tail = underLoad(400);
  const commands = {
    prune: [process.execPath, "-e", 'await Bun.write("pruned", "done");'],
    build: [process.execPath, "-e", 'while (!(await Bun.file("pruned").exists())) await Bun.sleep(5); await Bun.sleep('+tail+'); await Bun.write("built", "done");'],
  };
  const pending = startup(dir, commands);
  expect(existsSync(join(dir, "built"))).toBe(false);
  const result = await pending;
  expect(result.build.code).toBe(0);
  expect(result.prune.code).toBe(0);
  expect(result.prune.seconds).toBeLessThan(result.build.seconds - tail / 2000);
  expect(readFileSync(join(dir, "built"), "utf8")).toBe("done");
});

for (const failing of ["build", "prune"] as const) {
  test("startup retains the " + failing + " failure and waits for both children", async () => {
    const dir = fixture("startup-failure-" + failing);
    const command = (name: string) => [process.execPath, "-e", 'await Bun.sleep(25); await Bun.write("'+name+'", "done"); process.exit('+(name===failing ? 7 : 0)+');'];
    const result = await startup(dir, { build: command("build"), prune: command("prune") });
    expect(result[failing].code).toBe(7);
    expect(result[failing === "build" ? "prune" : "build"].code).toBe(0);
    expect(existsSync(join(dir,"build"))).toBe(true);
    expect(existsSync(join(dir,"prune"))).toBe(true);
  });
}
