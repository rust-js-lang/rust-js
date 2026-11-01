// The webapi crate's objects as JS has them: a named property, a weak
// collection's key. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React DOM's Fizz runtime reads and writes `element.dataset[name]`:
// DOMStringMap's IDL is all unnamed specials, a getter, a setter and a
// deleter of a name, each `this[name]` (#429).
test("a DOMStringMap's named properties are get, set and delete, dataset[name]", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-named-property");
  writeFileSync(join(dir, "lib.rs"), `use webapi::HTMLElement;

pub fn moved(node: &HTMLElement, from: &str, to: &str) -> Option<String> {
    let dataset = node.dataset();
    let value = dataset.get(from)?;
    dataset.set(to, &value);
    dataset.delete(from);
    Some(value)
}

pub fn cleared(node: &HTMLElement) -> impl Fn() + '_ {
    move || node.dataset().delete("x")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const value = dataset[from];");
  expect(js).toContain("dataset[to] = value;");
  expect(js).toContain("delete dataset[from];");
  const lib = await import(join(dir, "lib.js"));
  const dataset: Record<string, string> = { precedence: "high" };
  expect(lib.moved({ dataset }, "precedence", "p")).toBe("high");
  expect(dataset).toEqual({ p: "high" });
  expect(lib.moved({ dataset }, "missing", "q")).toBeUndefined();
  expect(dataset).toEqual({ p: "high" });
  // A closure of a delete gives what Rust's does, nothing: not `true`.
  expect(lib.cleared({ dataset: { x: "1" } })()).toBeUndefined();
});

// React DOM's Fizz config caches each Blob's data URL by the Blob,
// `WeakMap<Blob, Thenable<string>>`: every web API object is an object,
// TypeScript's `WeakKey`, which a weak collection holds (#446, ADR 0677).
test("a web API object is a WeakKey, a WeakMap's key", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-weak-key");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use webapi::{Blob, Element};

thread_local! {
    static SIZES: Cell<Option<&'static js::WeakMap<Blob, f64>>> = const { Cell::new(None) };
}

pub fn size_of(blob: &'static Blob) -> f64 {
    let sizes = match SIZES.get() {
        Some(sizes) => sizes,
        None => {
            let sizes = js::weak_map::new();
            SIZES.set(Some(sizes));
            sizes
        }
    };
    if let Some(size) = sizes.get(blob) {
        return size;
    }
    let size = blob.size();
    sizes.set(blob, size);
    size
}

pub fn seen(element: &'static Element, set: &js::WeakSet<Element>) -> bool {
    let was = set.has(element);
    set.add(element);
    was
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("new WeakMap()");
  const lib = await import(join(dir, "lib.js"));
  const blob = { size: 3 };
  expect([lib.sizeOf(blob), lib.sizeOf(blob)]).toEqual([3, 3]);
  const set = new WeakSet();
  const element = {};
  expect([lib.seen(element, set), lib.seen(element, set)]).toEqual([false, true]);
});

// React's server stream config for Node reads `const {read, written} =
// textEncoder.encodeInto(text, view)`: what a function gives has each member
// TypeScript's DOM requires, though WebIDL's dictionary doesn't say
// `required`, so it isn't an `Option` (ADR 0102, amended).
test("a dictionary a function gives has the members TypeScript's DOM requires", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-given-dictionary");
  writeFileSync(join(dir, "lib.rs"), `use webapi::text_encoder;

pub fn encoded(text: &str) -> f64 {
    let view = js::uint8_array::new(8);
    let result = text_encoder::new().encode_into(text, view);
    result.read * 100.0 + result.written
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return result.read * 100 + result.written;");
  const { encoded } = await import(join(dir, "lib.js"));
  expect(encoded("héllo")).toBe(506);
});

// rd452: a ReadableStream reads from TypeScript's \`UnderlyingByteSource\`
// or \`UnderlyingDefaultSource\`, which WebIDL takes as an \`object\`: React's
// Fizz \`new ReadableStream({ type: 'bytes', pull, cancel }, { highWaterMark:
// 0 })\`, each function lent, its controller the stream's (the user's
// decision).
test("a byte stream reads from its byte source's pull", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-byte-source");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Uint8Array;
use webapi::{
    QueuingStrategy, ReadableByteStreamController, ReadableStream, ReadableStreamDefaultController,
    UnderlyingByteSource, UnderlyingDefaultSource, readable_stream,
};

thread_local! {
    static CANCELLED: Cell<bool> = const { Cell::new(false) };
}

pub fn bytes(chunk: &'static Uint8Array) -> &'static ReadableStream {
    readable_stream::new(
        Some(&UnderlyingByteSource {
            r#type: "bytes",
            auto_allocate_chunk_size: None,
            start: None,
            pull: Some(&move |controller: &'static ReadableByteStreamController| {
                controller.enqueue(chunk);
                controller.close();
            }),
            cancel: Some(&|_reason| CANCELLED.set(true)),
        }),
        Some(QueuingStrategy {
            high_water_mark: Some(0.0),
        }),
    )
}

pub fn values() -> &'static ReadableStream {
    readable_stream::new(
        Some(&UnderlyingDefaultSource {
            start: Some(&|controller: &'static ReadableStreamDefaultController| controller.close()),
            ..Default::default()
        }),
        None,
    )
}

pub fn cancelled() -> bool {
    CANCELLED.get()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`new ReadableStream(
    {
      type: "bytes",
      pull(controller) {
        controller.enqueue(chunk);
        controller.close();
      },`);
  expect(js).toContain("{\n      highWaterMark: 0,\n    },");
  const lib = await import(join(dir, "lib.js"));
  const reader = lib.bytes(new Uint8Array([1, 2, 3])).getReader();
  const read = await reader.read();
  expect([...read.value]).toEqual([1, 2, 3]);
  expect((await reader.read()).done).toBe(true);
  await lib.bytes(new Uint8Array([4])).cancel("why");
  expect(lib.cancelled()).toBe(true);
  expect((await lib.values().getReader().read()).done).toBe(true);
});

// rd453: a record a binding takes, TypeScript's \`Record<string, V>\`, is
// what vouches it's one, a \`Dict<V>\` or a struct of \`Option<V>\`s each
// \`None\` of which is written \`None\`, so it's left out: React's
// Fizz \`new Headers(headersDescriptor)\` of its \`{ Link }\`, a
// \`HeadersInit\`'s record member, which webapi left out.
test("a struct that vouches it's a record is one, new Headers(descriptor)", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-record");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Headers, Record, URLSearchParams, headers, url_search_params};

pub struct HeadersDescriptor {
    #[cfg_attr(rust_js, rust_js::name = "Link")]
    pub link: Option<String>,
}

unsafe impl Record for HeadersDescriptor {
    type Value = String;
}

pub fn linked(link: Option<String>) -> &'static Headers {
    let descriptor = match link {
        Some(link) => HeadersDescriptor { link: Some(link) },
        None => HeadersDescriptor { link: None },
    };
    headers::new(Some(&descriptor))
}

pub fn query(params: &js::Dict<String>) -> &'static URLSearchParams {
    url_search_params::new(Some(params))
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return new Headers(descriptor);");
  expect(js).toContain("return new URLSearchParams(params);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.linked("</a.css>; rel=preload").get("link")).toBe("</a.css>; rel=preload");
  expect([...lib.linked(undefined).keys()]).toEqual([]);
  expect(lib.query({ q: "x", n: "1" }).toString()).toBe("q=x&n=1");
});

