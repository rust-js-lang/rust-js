// While std's runtime holds a line left unended, `console.log` is wrapped,
// so a crate's whole line, `println!`'s (ADR 0087), comes after it, as one
// `LineWriter` writes every crate's natively (#2320); unwrapped once nothing's
// held. What's held is written first by the host's own write, which its
// `console.log` comes after (#2347): Bun's the descriptor's, `fs.writeSync`,
// as its console writes the descriptor itself; Node's the stream's, which
// its console writes through. A browser's console, whose every
// `console.log` is a line, has the held text joined to the line.
import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { runInNewContext } from "node:vm";

import { underLoad } from "../scripts/timing";
import { helpersModule } from "./packages-of";
import { fixture } from "./support";

const probe = `
const original = console.log;
const rt = await import(process.argv[2]);
const said = {};
rt.$print("partial ");
said.wrapped = console.log !== original;
console.log("lib");
said.unwrapped = console.log === original;
rt.$print("held ");
rt.$print("ended\\n");
said.unwrappedByItsLine = console.log === original;
rt.$print("x");
console.log("values", 1);
rt.$printBytes([0xff], 0);
console.log("after bytes");
// A whole line the runtime writes, then a console.log line, each 20 KB:
// Bun's stream wrote 6 of 200 after the line made after it (#2347).
for (let i = 0; i < 200; i++) {
  rt.$print("p".repeat(20000) + i + "\\n");
  console.log("L" + i);
}
// And standard error's, the runtime's eprint! then a console.error.
for (let i = 0; i < 200; i++) {
  rt.$eprint("e".repeat(20000) + i + "\\n");
  console.error("E" + i);
}
(await import("node:fs")).writeFileSync(process.argv[3], JSON.stringify(said));
`;

test("what's held comes before a console.log line, and a runtime's whole line before the next, stdout's and stderr's, on Node and Bun through a pipe", () => {
  const dir = fixture("stdout-hold");
  const file = join(dir, "probe.mjs");
  writeFileSync(file, probe);
  const lines = Array.from({ length: 200 }, (_, i) => `${"p".repeat(20000)}${i}\nL${i}\n`).join("");
  const expected = Buffer.concat([Buffer.from("partial lib\nheld ended\nxvalues 1\n"), Buffer.from([0xff]), Buffer.from(`after bytes\n${lines}`)]);
  for (const host of ["node", "bun"]) {
    const said = join(dir, `${host}.json`);
    const ran = Bun.spawnSync([host, file, pathToFileURL(helpersModule).href, said], { cwd: dir, timeout: 60_000, stdout: "pipe", stderr: "pipe" });
    expect([host, JSON.parse(readFileSync(said, "utf8"))]).toEqual([host, { wrapped: true, unwrapped: true, unwrappedByItsLine: true }]);
    const errors = Array.from({ length: 200 }, (_, i) => `${"e".repeat(20000)}${i}\nE${i}\n`).join("");
    expect([host, ran.stderr.toString() === errors]).toEqual([host, true]);
    const out = Buffer.from(ran.stdout);
    expect([host, out.length, out.equals(expected)]).toEqual([host, expected.length, true]);
  }
}, underLoad(120_000));

// A descriptor's write that would block, `EAGAIN`, is tried again after a
// wait, never at once in a hot loop, and no more than its limit in a row;
// what's written in parts is written whole; any other error, at once.
test("a write that would block is tried again after a wait, up to its limit, and what's written in parts is written whole", async () => {
  const { $writeAll } = await import(pathToFileURL(helpersModule).href);
  const blocked = () => Object.assign(new Error("would block"), { code: "EAGAIN" });
  let calls = 0;
  const written: string[] = [];
  const pauses: number[] = [];
  $writeAll(
    (bytes: Uint8Array) => {
      if (++calls <= 2) throw blocked();
      const n = Math.min(3, bytes.length);
      written.push(Buffer.from(bytes.subarray(0, n)).toString());
      return n;
    },
    "abcdefg",
    (ms: number) => pauses.push(ms),
    5,
  );
  expect([written, pauses]).toEqual([["abc", "def", "g"], [1, 1]]);
  const waited: number[] = [];
  expect(() => $writeAll(() => { throw blocked(); }, "a", (ms: number) => waited.push(ms), 3)).toThrow("would block");
  expect(waited).toEqual([1, 1, 1]);
  const other: number[] = [];
  expect(() => $writeAll(() => { throw new Error("closed"); }, "a", (ms: number) => other.push(ms), 3)).toThrow("closed");
  expect(other).toEqual([]);
});

// Where there's no stream, each `console.log` is a line: the held text is
// joined to the line, and values, which can't follow it on its line, are
// refused (#2320).
test("a browser's console has the held text joined to a line, and refuses values after it", () => {
  const lines: string[] = [];
  const context = { console: { log: (s: string) => lines.push(s), error: (s: string) => lines.push(`error ${s}`) }, queueMicrotask: () => {}, TextDecoder, TextEncoder };
  const runtime = readFileSync(helpersModule, "utf8").replace(/^export /gm, "");
  const refused = runInNewContext(
    `${runtime}
$print("partial ");
console.log("lib");
$print("tail\\n");
$print("x");
let said;
try { console.log("v", 1); } catch (error) { said = error.message; }
said;`,
    context,
  );
  expect(lines).toEqual(["partial lib", "tail"]);
  expect(refused).toBe("rust-js: stdout holds a line left unended, and on this host a console.log of values can't follow it but apart, so a line printed now would come out before it");
});
