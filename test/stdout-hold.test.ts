// While std's runtime holds a line left unended, `console.log` is wrapped,
// so a crate's whole line, `println!`'s (ADR 0087), comes after it: the held
// text joined to the line, one `console.log`, never a stream's write apart,
// which Bun's `console.log`, writing the descriptor itself, can overtake
// through a pipe (#2320, #2347). Unwrapped once nothing's held. A
// `console.log` of values while text is held writes what's held first where
// the host's `console.log` writes through the stream, Node's, and is refused
// where it doesn't, Bun's, as held bytes that aren't text are. Each host
// run here with a stream and a console that record what they're given.
import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { helpersModule } from "./packages-of";
import { fixture } from "./support";

const probe = `
const calls = [];
process.stdout.write = (chunk) => (calls.push(["write", typeof chunk === "string" ? chunk : "bytes " + Buffer.from(chunk).toString("hex")]), true);
const original = (...args) => calls.push(["log", ...args]);
console.log = original;
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
try {
  console.log("values", 1);
  said.values = "logged";
} catch (error) {
  said.values = error.message;
}
rt.$flushStdout();
rt.$printBytes([0xff], 0);
try {
  console.log("after bytes");
  said.bytes = "logged";
} catch (error) {
  said.bytes = error.message;
}
rt.$flushStdout();
process.stderr.write(JSON.stringify({ calls, said }));
`;

function run(host: string): { calls: string[][]; said: Record<string, unknown> } {
  const dir = fixture(`stdout-hold-${host}`);
  const file = join(dir, "probe.mjs");
  writeFileSync(file, probe);
  const ran = Bun.spawnSync([host, file, pathToFileURL(helpersModule).href], { cwd: dir, timeout: 60_000 });
  return JSON.parse(ran.stderr.toString());
}

const refused = "rust-js: stdout holds a line left unended, and on this host";

test("a whole line printed while stdout holds text is joined to it, one console.log, and the wrapper goes once nothing's held", () => {
  for (const host of ["node", "bun"]) {
    const { calls, said } = run(host);
    expect([host, said.wrapped, said.unwrapped, said.unwrappedByItsLine]).toEqual([host, true, true, true]);
    // Joined: no write of "partial " apart from the line.
    expect([host, calls.slice(0, 2)]).toEqual([host, [["log", "partial lib"], ["write", "held "]]]);
  }
});

test("a console.log of values, or after held bytes that aren't text, writes what's held first on Node and is refused on Bun", () => {
  const node = run("node");
  expect(node.said).toMatchObject({ values: "logged", bytes: "logged" });
  expect(node.calls).toContainEqual(["write", "x"]);
  expect(node.calls).toContainEqual(["write", "bytes ff"]);
  const bun = run("bun");
  expect(bun.said.values).toBe(`${refused} a console.log of values can't follow it but apart, so a line printed now would come out before it`);
  expect(bun.said.bytes).toBe(`${refused} what's held isn't text, which console.log can't write, so a line printed now would come out before it`);
});
