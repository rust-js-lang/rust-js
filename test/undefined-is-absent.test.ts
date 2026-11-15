// A binding marked `#[rust_js::undefined_is_absent]` takes a trailing
// `undefined` of an optional argument as no argument, so a call of it gives
// a trailing `None` as `undefined`, where another's is a call of each count
// (`bindings::undefined_is_absent`, #1842, ADR 0443, amended). Each mark is
// held here: one the webapi generator places of a WebIDL operation or
// constructor, which WebIDL's overload resolution takes so, is one
// @webref/idl declares; any other, a hand's, is one of `shown`'s, whose
// row, in Bun and in Chromium, shows its calls given trailing `undefined`s
// do what they do without them.

import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";
import { webidlCalls } from "../webapi/coverage";
import { root } from "./support";
import { underLoad } from "../scripts/timing";

const mark = "rust_js::undefined_is_absent)";

/** Each binding a hand marks, by its `link_name`, and its evidence: what a
 * page answers of its calls, each count's with and without trailing
 * `undefined`s, which must be alike. */
type Shown = { name: string; evidence: () => [string, string][] | Promise<[string, string][]> };

// An `Error`'s and an `AggregateError`'s own properties, message, cause and
// the stack's first frame, made at one site of the arguments given, each
// count's alike with trailing `undefined`s (ES2025 §20.5.1.1, §20.5.7.1).
const errors = (): [string, string][] => {
  const made = (of: ErrorConstructor | AggregateErrorConstructor, args: unknown[]) => Reflect.construct(of, args) as Error;
  const called = (of: ErrorConstructor, args: unknown[]) => (of as (...args: unknown[]) => Error)(...args);
  const seen = (e: Error) =>
    JSON.stringify([
      Object.getOwnPropertyNames(e).sort(),
      e.message,
      "cause" in e,
      (e as { cause?: unknown }).cause,
      (e.stack ?? "").split("\n").find((line) => /^\s+at /.test(line)) ?? "",
    ]);
  const cause = { why: 1 };
  const pairs: [string, string][] = [];
  // Each pair made at one site, so its stacks' frames are alike.
  const each = (given: unknown[][], count: number, make: (args: unknown[]) => Error) => {
    for (const args of given) {
      const [left, right] = [args, [...args, ...Array(count - args.length).fill(undefined)]].map((a) => seen(make(a)));
      pairs.push([left, right]);
    }
  };
  each([[], ["m"], ["m", { cause }]], 2, (args) => made(Error, args));
  each([[], ["m"], ["m", { cause }]], 2, (args) => called(Error, args));
  each([[[1]], [[1], "m"], [[1], "m", { cause }]], 3, (args) => made(AggregateError, args));
  return pairs;
};

// The timers' firing order, each call given its trailing `undefined`s or
// none, alike: a timeout's and an interval's delay, HTML's `optional long
// timeout = 0`, Node's 1 ms where it's none; a clear of none clears nothing.
const timers = async (): Promise<[string, string][]> => {
  const order = (padded: boolean) =>
    new Promise<string>((resolve) => {
      const seen: string[] = [];
      const more = padded ? [undefined] : [];
      const call = (f: (...args: unknown[]) => unknown, ...args: unknown[]) => f(...args, ...more);
      setTimeout(() => seen.push("zero"), 0);
      call(setTimeout as (...args: unknown[]) => unknown, () => seen.push("timeout"));
      setTimeout(() => seen.push("one"), 1);
      setTimeout(() => seen.push("kept"), 5);
      call(clearTimeout as (...args: unknown[]) => unknown);
      // An interval's ticks apart, which the load moves among timeouts.
      let ticks = 0;
      const interval = call(setInterval as (...args: unknown[]) => unknown, () => {
        if (++ticks === 2) clearInterval(interval as ReturnType<typeof setInterval>);
      });
      call(clearInterval as (...args: unknown[]) => unknown);
      setTimeout(() => resolve(`${seen.join(",")} ticks ${ticks}`), 60);
    });
  return [[await order(false), await order(true)]];
};

const shown: Shown[] = [
  { name: "new Error", evidence: errors },
  { name: "new AggregateError", evidence: errors },
  { name: "setTimeout", evidence: timers },
  { name: "clearTimeout", evidence: timers },
  { name: "setInterval", evidence: timers },
  { name: "clearInterval", evidence: timers },
];

// Each mark, of a Rust source the repository tracks, by its `link_name`.
const marks = () => {
  const files = spawnSync("git", ["grep", "-l", mark, "--", "*.rs", ":!test"], { cwd: root, encoding: "utf8" }).stdout.trim().split("\n");
  const binding = /#\[cfg_attr\(rust_js, rust_js::link_name = "([^"]+)"\)\]\n((?:\s*#\[[^\n]*\]\n)*)\s*(?:pub )?fn /g;
  return files.flatMap((file) => {
    const source = readFileSync(join(root, file), "utf8");
    const found = [...source.matchAll(binding)].filter(([, , attrs]) => attrs.includes(mark)).map(([, name]) => ({ file, name }));
    // Each mark is of a binding the pattern reads.
    expect([file, found.length]).toEqual([file, source.split(mark).length - 1]);
    return found;
  });
};

test("each binding marked undefined_is_absent is a WebIDL operation or constructor, or shown here", async () => {
  const names = await webidlCalls();
  const all = marks();
  const generated = all.filter((m) => m.file === "webapi/src/lib.rs");
  expect(generated.length).toBeGreaterThan(500);
  const call = (name: string) => name.replace(/^new /, "").split(/[.#]/).at(-1)!;
  const hand = all.filter((m) => m.file !== "webapi/src/lib.rs" || !names.has(call(m.name))).map((m) => m.name);
  // Of a hand's, each has its row, and each row is of a mark.
  expect([...new Set(hand)].sort()).toEqual([...shown.map((s) => s.name), "console.timeStamp"].sort());
});

for (const { name, evidence } of shown) {
  test(`${name} takes a trailing undefined as no argument, in Bun and in Chromium`, async () => {
    const inBun = await evidence();
    expect(inBun.length).toBeGreaterThan(0);
    for (const [given, padded] of inBun) expect(padded).toBe(given);
    const browser = await chromium.launch();
    try {
      const page = await browser.newPage();
      const inPage = await page.evaluate((code) => new Function(`return (${code})();`)() as Promise<[string, string][]>, evidence.toString());
      for (const [given, padded] of inPage) expect(padded).toBe(given);
    } finally {
      await browser.close();
    }
  }, underLoad(120_000));
}

// `console.timeStamp`, Chrome's DevTools' extensibility API, which no
// WebIDL declares: DevTools records the same `TimeStamp` event of each
// count's call, its trailing ones `undefined` or left out, but each event's
// sample id. Bun has no DevTools trace; its `timeStamp` records nothing.
test("console.timeStamp takes a trailing undefined as no argument, in Chromium's trace", async () => {
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto("about:blank");
    const cdp = await page.context().newCDPSession(page);
    const events: { name: string; args?: { data?: Record<string, unknown> } }[] = [];
    cdp.on("Tracing.dataCollected", (e) => events.push(...(e.value as unknown as typeof events)));
    const done = new Promise<void>((resolve) => cdp.once("Tracing.tracingComplete", () => resolve()));
    await cdp.send("Tracing.start", { traceConfig: { includedCategories: ["devtools.timeline"] }, transferMode: "ReportEvents" });
    await page.evaluate(() => {
      const values = [performance.now(), "given-0", "track", "group", "primary"];
      const stamp = (console as unknown as { timeStamp: (...args: unknown[]) => void }).timeStamp;
      for (let k = 0; k <= 5; k++) {
        const given = values.slice(0, k);
        stamp(`given-${k}`, ...given);
        stamp(`undefined-${k}`, ...given, ...Array(5 - k).fill(undefined));
      }
    });
    await cdp.send("Tracing.end");
    await done;
    const recorded = new Map<string, string>();
    for (const e of events.filter((e) => e.name === "TimeStamp")) {
      const { message, name: _name, sampleTraceId: _id, ...data } = e.args?.data ?? {};
      recorded.set(String(message), JSON.stringify(data));
    }
    for (let k = 0; k <= 5; k++) {
      expect(recorded.get(`undefined-${k}`)).toBe(recorded.get(`given-${k}`)!);
    }
    expect(JSON.parse(recorded.get("given-5")!)).toHaveProperty("color", "primary");
  } finally {
    await browser.close();
  }
}, underLoad(120_000));
