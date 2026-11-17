// Node's streams, `stream`'s `Writable`, `Readable` and `Duplex`, each event
// listened to as React's Node server streams listen, and `crypto`'s hash
// (ADR 0272, amended), compiled and run in Node. Split from
// test/node-bindings.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildNode } from "./node-crate";
import { buildReact, compiler, fixture, run } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0272, amended: `crypto`'s hash and `stream`'s `Writable`, as React's
// server stream config for Node hashes a key and writes its chunks, text
// and bytes, to the response; `util`'s `TextEncoder` is imported from
// `util`, as a host may have no global one.
// rd459: a Writable emits "drain", "error" and "close", each listener of
// its own type, as React's renderToPipeableStream listens to its
// destination: \`destination.on('drain', createDrainHandler(..))\`.
test("a Writable's drain, error and close are listened to", () => {
  const dir = fixture("node-stream-events");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;
use node::event;
use node::events::EventEmitterExt;
use node::stream::Writable;

pub fn watch(destination: &'static Writable, log: &'static dyn Fn(&str)) {
    destination.on(event::Drain, Box::new(move || log("drain")));
    destination.on(event::Error, Box::new(move |error: &JsError| log(&error.message())));
    destination.on(event::Close, Box::new(move || log("close")));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('destination.on("drain", () => {\n    log("drain");\n  });');
  expect(js).toContain('destination.on("error", (error) => {\n    log(error.message);\n  });');
  expect(js).toContain('destination.on("close", () => {\n    log("close");\n  });');
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { Writable } from "node:stream";
import { watch } from "./lib.js";
const seen = [];
const destination = new Writable({ highWaterMark: 1, write(chunk, _, done) { setTimeout(done, 1); } });
watch(destination, (said) => seen.push(said));
destination.write("abc");
destination.once("drain", () => {
  destination.destroy(new Error("boom"));
  destination.on("close", () => console.log(JSON.stringify(seen)));
});
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual(["drain", "boom", "close"]);
});

// rd462: a Readable its \`read\` pushes to, React's prerenderToNodeStream's
// prelude: \`new Readable({ read() { .. } })\`, \`readable.push(chunk)\`, its
// end \`readable.push(null)\`.
test("a Readable is pushed to by its read", () => {
  const dir = fixture("node-readable");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use node::stream::{Readable, ReadableOptions, readable};

pub fn counted(to: u32) -> &'static Readable {
    let count = Cell::new(0);
    let stream: Cell<Option<&'static Readable>> = Cell::new(None);
    let made = readable::new(Some(&ReadableOptions {
        read: Some(&|_size| {
            let stream = stream.get().unwrap();
            if count.get() < to {
                count.set(count.get() + 1);
                stream.push(Some(&count.get().to_string().as_str()));
            } else {
                stream.push(None);
            }
        }),
        ..Default::default()
    }));
    stream.set(Some(made));
    made
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { Readable } from "stream";');
  expect(js).toContain("const made = new Readable({");
  expect(js).toContain("stream$1.push(null);");
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { counted } from "./lib.js";
const seen = [];
const readable = counted(3);
readable.on("data", (chunk) => seen.push(chunk.toString()));
readable.on("end", () => console.log(JSON.stringify(seen)));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual(["1", "2", "3"]);
});

// rd601: a Readable emits what @types/node types its `on` of, each listener
// of its own type, as React's Flight client reads a Node stream:
// \`stream.on('data', chunk => ..); stream.on('error', ..); stream.on('end', onEnd)\`.
test("a Readable's data, end, error, close, pause, resume and readable are listened to", () => {
  const dir = fixture("node-readable-events");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use js::{JsError, Unknown};
use node::event;
use node::events::EventEmitterExt;
use node::stream::Readable;

pub fn read_all(stream: &'static Readable, log: &'static dyn Fn(&str)) {
    stream.on(event::Data, Box::new(move |chunk: Option<&Unknown>| log(&js::string(chunk))));
    stream.on(event::Error, Box::new(move |error: &JsError| log(&error.message())));
    stream.on(event::Pause, Box::new(move || log("pause")));
    stream.on(event::Resume, Box::new(move || log("resume")));
    stream.on(event::End, Box::new(move || log("end")));
    stream.on(event::Close, Box::new(move || log("close")));
}

pub fn when_readable(stream: &'static Readable, log: &'static dyn Fn(&str)) {
    stream.on(event::Readable, Box::new(move || log("readable")));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('stream.on("data", (chunk) => {\n    log(String(chunk));\n  });');
  expect(js).toContain('stream.on("end", () => {\n    log("end");\n  });');
  expect(js).toContain('stream.on("readable", () => {');
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { Readable } from "node:stream";
import { readAll, whenReadable } from "./lib.js";
const seen = [];
const log = (said) => seen.push(said);
const stream = new Readable({ read() {} });
readAll(stream, log);
stream.push("a");
stream.push("b");
stream.push(null);
stream.on("close", () => {
  const failing = new Readable({ read() {} });
  readAll(failing, log);
  failing.pause();
  failing.destroy(new Error("boom"));
  failing.on("close", () => {
    const paused = new Readable({ read() {} });
    whenReadable(paused, log);
    paused.push(null);
    paused.on("end", () => console.log(JSON.stringify(seen)));
    paused.read();
  });
});
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual(["resume", "a", "b", "end", "close", "pause", "resume", "boom", "close", "readable"]);
});

// rd588: a Duplex is a Readable, by Deref, and a Writable, by AsRef, each
// the object itself, as @types/node's `interface Duplex extends Readable,
// Writable`, React's Flight server's `debugChannel`; its events both's.
test("a Duplex is read from and written to, and emits both's events", () => {
  const dir = fixture("node-duplex");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
use node::event;
use node::events::EventEmitterExt;
use node::stream::{Duplex, Readable, Writable};

fn send(destination: &Writable, text: &str) -> bool {
    destination.write(text)
}

pub fn echo(duplex: &'static Duplex, log: &'static dyn Fn(&str)) {
    duplex.on(event::Data, Box::new(move |chunk: Option<&Unknown>| log(&js::string(chunk))));
    duplex.on(event::End, Box::new(move || log("end")));
    duplex.on(event::Finish, Box::new(move || log("finish")));
    duplex.on(event::Pipe, Box::new(move |_source: &Readable| log("pipe")));
    duplex.push(Some(&"read"));
    duplex.push(None);
    send(duplex.as_ref(), "written");
    duplex.as_ref().end();
}

pub fn piped(destination: &'static Writable, log: &'static dyn Fn(&str)) {
    destination.on(event::Unpipe, Box::new(move |_source: &Readable| log("unpipe")));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  duplex.push("read");\n  duplex.push(null);\n  send(duplex, "written");\n  duplex.end();');
  expect(js).toContain('duplex.on("pipe", (source) => {');
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { Duplex, Readable } from "node:stream";
import { echo, piped } from "./lib.js";
const seen = [];
const log = (said) => seen.push(said);
const duplex = new Duplex({ read() {}, write(chunk, _, done) { log("wrote " + chunk); done(); } });
echo(duplex, log);
duplex.on("close", () => {
  const source = Readable.from([]);
  const destination = new Duplex({ read() {}, write(chunk, _, done) { done(); } });
  piped(destination, log);
  source.pipe(destination);
  source.unpipe(destination);
  console.log(JSON.stringify(seen));
});
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual(["wrote written", "read", "finish", "end", "unpipe"]);
});

test("node's crypto hashes and a Writable is written to", () => {
  const dir = fixture("node-stream");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use node::crypto::{self, BinaryToTextEncoding};
use node::stream::Writable;
use node::util::{TextDecodeOptions, TextDecoderOptions, text_decoder, text_encoder};

pub fn create_fast_hash(input: &str) -> String {
    let hash = crypto::create_hash("md5");
    hash.update(input);
    hash.digest(BinaryToTextEncoding::Hex)
}

pub fn write_both(destination: &Writable, text: &str) -> bool {
    let bytes = text_encoder::new().encode(Some(text));
    let has_capacity = destination.write(text);
    destination.write(bytes) && has_capacity
}

pub fn close(destination: &Writable) {
    destination.end();
}

pub fn decoded(text: &str) -> String {
    let bytes = text_encoder::new().encode(Some(text));
    let decoder = text_decoder::new(Some("utf-8"), Some(TextDecoderOptions { ignore_bom: Some(true), ..Default::default() }));
    decoder.decode(Some(&bytes), Some(TextDecodeOptions { stream: Some(true) })) + &decoder.decode(None, None)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('import { createHash } from "crypto";');
  expect(js).toContain('const hash = createHash("md5");\n  hash.update(input);\n  return hash.digest("hex");');
  expect(js).toContain("const bytes = new TextEncoder().encode(text);");
  expect(js).toContain("destination.end();");
  expect(js).toContain('import { TextDecoder, TextEncoder } from "util";');
  expect(js).toContain('new TextDecoder("utf-8", { ignoreBOM: true });');
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { Writable } from "node:stream";
import { createFastHash, writeBoth, close, decoded } from "./lib.js";
const seen = [];
const destination = new Writable({ write(chunk, _, done) { seen.push(Buffer.from(chunk).toString()); done(); } });
const capacity = writeBoth(destination, "hi");
close(destination);
destination.on("finish", () => console.log(JSON.stringify([createFastHash("abc"), capacity, seen, decoded("é!")])));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual(["900150983cd24fb0d6963f7d28e17f72", true, ["hi", "hi"], "é!"]);
});
