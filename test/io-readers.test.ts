// A known byte source's place is given its runtime dictionary in the box
// generic code gives it: a handle on the place, `{ get value() { .. }, set
// value(value) { .. } }`, where the dictionary writes the box, else a plain
// box, `{ value: e }`, which reads the same (#1329). Which writes it is
// `ByteSource::writes_its_box` (recognition/io.rs), a fact of the runtime's
// dictionaries: each is called here with a box whose setter throws, which
// throws exactly where the compiler gives a known one's place a handle; a
// file's, its reader's and writer's, read for what they assign, as no test
// opens the OS file their box holds.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, helpersModule, run } from "./support";

beforeAll(buildCompiler, 600_000);

const program = `use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Write};

pub fn main() {
    let mut b = [0u8; 2];
    let mut slice_src: &[u8] = &[1, 2, 3];
    let _ = slice_src.read(&mut b);
    let mut empty_src = io::empty();
    let _ = empty_src.read(&mut b);
    let mut repeat_src = io::repeat(5);
    let _ = repeat_src.read(&mut b);
    let mut cursor_src = Cursor::new(vec![1u8, 2, 3]);
    let _ = cursor_src.read(&mut b);
    let mut deque_src = VecDeque::from(vec![1u8, 2, 3]);
    let _ = deque_src.read_exact(&mut b);
    let mut buf_src = BufReader::new(io::empty());
    let _ = buf_src.read(&mut b);
    let mut file_src = File::open("x").unwrap();
    let _ = file_src.read(&mut b);
    let mut file_out = File::create("y").unwrap();
    let _ = file_out.write_all(b"x");
}
`;

// Each method a dictionary may hold, and what it's given past its box.
const given: Record<string, () => unknown[]> = {
  read: () => [[0, 0]],
  readBuf: () => [{ buf: [0, 0], filled: 0, init: 0 }],
  readExact: () => [[0, 0]],
  readToEnd: () => [[]],
  readToString: () => [{ value: "" }],
  fillBuf: () => [],
  consume: () => [1],
  readUntil: () => [10, []],
  skipUntil: () => [10],
  readLine: () => [{ value: "" }],
  seek: () => [{ TAG: "Start", _0: 0n }],
  streamPosition: () => [],
  seekRelative: () => [0n],
};

const WROTE = Symbol("wrote");

/** Whether a method of `dictionary` writes the box it's given `value` in. */
function writesItsBox(dictionary: Record<string, unknown>, value: () => unknown): boolean {
  return Object.entries(given).some(([name, args]) => {
    const method = dictionary[name];
    if (typeof method !== "function") return false;
    const held = value();
    const box = {
      get value() {
        return held;
      },
      set value(_: unknown) {
        throw WROTE;
      },
    };
    try {
      method.call(dictionary, box, ...args());
    } catch (e) {
      return e === WROTE;
    }
    return false;
  });
}

test("a known byte source's place is a handle exactly where its runtime dictionary writes its box", async () => {
  const dir = fixture("io-readers");
  const input = join(dir, "main.rs");
  writeFileSync(input, program);
  run([compiler, input, "-o", join(dir, "main.js")]);
  const js = readFileSync(join(dir, "main.js"), "utf8");
  const handled = (name: string) => new RegExp(`set value\\(value\\) \\{\\s*${name} = value;`).test(js);
  const rt = await import(helpersModule);
  const sources: [string, Record<string, unknown>, () => unknown][] = [
    ["sliceSrc", rt.$sliceReader, () => [1, 2, 3]],
    ["emptySrc", rt.$emptyReader, () => undefined],
    ["repeatSrc", rt.$repeatReader, () => ({ byte: 5 })],
    ["cursorSrc", rt.$cursorReader({ asRef: (v: unknown) => v }), () => ({ inner: [1, 2, 3], pos: 0n })],
    ["dequeSrc", rt.$dequeReader, () => [1, 2, 3]],
    ["bufSrc", rt.$bufReader(rt.$emptyReader), () => ({ inner: undefined, buf: [0, 0, 0, 0], pos: 0, filled: 0, initialized: 0 })],
  ];
  const seen = sources.map(([name, dictionary, value]) => [name, handled(name), writesItsBox(dictionary, value)]);
  expect(seen.filter(([, h, w]) => h !== w)).toEqual([]);
  // The writer is found, so the probe reaches what it writes.
  expect(seen.find(([name]) => name === "sliceSrc")).toEqual(["sliceSrc", true, true]);
  // A file's, whose place is an OS file no test opens, writes no box either,
  // its reader's nor its writer's: their methods read their box's `value`
  // alone.
  expect([handled("fileSrc"), handled("fileOut")]).toEqual([false, false]);
  const assigns = (dictionary: Record<string, unknown>) =>
    Object.values(dictionary).some((m) => typeof m === "function" && /\b\w+\.value\s*=[^=]/.test(String(m)));
  expect([assigns(rt.$fileReader), assigns(rt.$fileWriter)]).toEqual([false, false]);
  expect(assigns(rt.$sliceReader)).toBe(true);
});
