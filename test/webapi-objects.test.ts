// The webapi crate's objects as JS has them: a named property, a weak
// collection's key. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, realpathSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

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
  expect(js).toContain("    { highWaterMark: 0 },\n");
  const lib = await import(join(dir, "lib.js"));
  const reader = lib.bytes(new Uint8Array([1, 2, 3])).getReader();
  const read = await reader.read();
  expect([...read.value]).toEqual([1, 2, 3]);
  expect((await reader.read()).done).toBe(true);
  await lib.bytes(new Uint8Array([4])).cancel("why");
  expect(lib.cancelled()).toBe(true);
  expect((await lib.values().getReader().read()).done).toBe(true);
});

// rd403: a ReadableStream reads from TypeScript's general
// \`UnderlyingSource\` too, of either kind, its \`type\` \`"bytes"\` or none as
// the program says when it runs, each function given the stream's
// \`ReadableStreamController\`, read by what the two controllers have alike:
// React's Flight client's \`new ReadableStream({ type: type, start(c) {
// controller = c; } })\`.
test("a stream of either kind reads from its general source, its controller either's", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-general-source");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;
use webapi::{ReadableStream, ReadableStreamController, UnderlyingSource, readable_stream};

thread_local! {
    static CONTROLLER: Cell<Option<&'static ReadableStreamController>> = const { Cell::new(None) };
}

pub fn start_readable_stream(r#type: Option<&'static str>) -> &'static ReadableStream {
    readable_stream::new(
        Some(&UnderlyingSource {
            r#type,
            start: Some(&|c| CONTROLLER.set(Some(c))),
            ..Default::default()
        }),
        None,
    )
}

pub fn enqueue(value: &'static Unknown) {
    CONTROLLER.get().unwrap().enqueue(Some(value));
}

pub fn close() {
    CONTROLLER.get().unwrap().close();
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`return new ReadableStream({
    type,
    start(c) {
      controller = c;
    },
  });`);
  const lib = await import(join(dir, "lib.js"));
  const values = lib.startReadableStream(undefined).getReader();
  lib.enqueue({ id: 1 });
  lib.close();
  expect((await values.read()).value).toEqual({ id: 1 });
  expect((await values.read()).done).toBe(true);
  const bytes = lib.startReadableStream("bytes").getReader({ mode: "byob" });
  lib.enqueue(new Uint8Array([7, 8]));
  const read = await bytes.read(new Uint8Array(4));
  expect([...read.value]).toEqual([7, 8]);
  expect(() => lib.enqueue({ id: 2 })).toThrow(TypeError);
  const declared = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declared).toContain("export function startReadableStream(type?: string | null | undefined): ReadableStream;");
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


// rd506: TypeScript's overloads of `getReader`, which WebIDL's one
// operation types as a union: `getReader()` a default reader, and
// `getReader({ mode: "byob" })` a BYOB one, which throws a `TypeError` of a
// stream that isn't a byte stream, a `Result`. React's Flight reply
// client's `try { binaryReader = stream.getReader({mode: 'byob'}); } catch
// (x) { return serializeReader(stream.getReader()); }`.
test("a stream's reader is of the mode its options say, a BYOB one a Result", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-get-reader");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{
    Byob, ReadableStream, ReadableStreamBYOBReader, ReadableStreamDefaultReader, ReadableStreamGetByobReaderOptions,
    ReadableStreamGetReaderOptions,
};

fn serialize_binary_reader(reader: &ReadableStreamBYOBReader) -> String {
    reader.release_lock();
    "binary".to_owned()
}

fn serialize_reader(reader: &ReadableStreamDefaultReader) -> String {
    reader.release_lock();
    "values".to_owned()
}

pub fn serialize_readable_stream(stream: &ReadableStream) -> String {
    let binary_reader = match stream.get_reader_with_options(ReadableStreamGetByobReaderOptions { mode: Byob }) {
        Ok(binary_reader) => binary_reader,
        Err(_) => return serialize_reader(stream.get_reader()),
    };
    serialize_binary_reader(binary_reader)
}

pub fn of_mode(stream: &ReadableStream, mode: Option<&str>) -> bool {
    stream.get_reader_with_options(ReadableStreamGetReaderOptions { mode }).is_ok()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  let binaryReader;
  try {
    binaryReader = stream.getReader({ mode: "byob" });
  } catch {
    return serializeReader(stream.getReader());
  }
  return serializeBinaryReader(binaryReader);`);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.serializeReadableStream(new ReadableStream({ type: "bytes" })), lib.serializeReadableStream(new ReadableStream())]).toEqual(["binary", "values"]);
  expect([lib.ofMode(new ReadableStream({ type: "bytes" }), "byob"), lib.ofMode(new ReadableStream(), undefined), lib.ofMode(new ReadableStream(), "byob")]).toEqual([true, true, false]);
});

// rd535: a `From` into an untagged enum is the value it's given (ADR 0214),
// so it runs nothing, webapi's whose body rust-js doesn't see too: a
// borrow held while each chunk is made a `BlobPart`, React's Flight reply
// client's `buffer.borrow().iter().map(|&chunk| BlobPart::from(chunk))`,
// is plain, and so is every other borrow of a cell of its type.
test("a borrow held across a From into an untagged enum is plain", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-untagged-from-held");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

use js::Uint8Array;
use webapi::BlobPart;

pub fn parts(buffer: Rc<RefCell<Vec<&'static Uint8Array>>>, chunk: &'static Uint8Array) -> usize {
    buffer.borrow_mut().push(chunk);
    let parts: Vec<BlobPart<'_>> = buffer.borrow().iter().map(|&chunk| BlobPart::from(chunk)).collect();
    parts.len()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  buffer.push(chunk);
  const parts = buffer.slice();
  return parts.length;`);
  expect(js).not.toContain("$borrow");
  const lib = await import(join(dir, "lib.js"));
  const buffer = [new Uint8Array([1])];
  expect([lib.parts(buffer, new Uint8Array([2])), buffer.length]).toEqual([2, 2]);
});

// rd505, rd397: `FormData`'s `append` and `set` take a value as TypeScript's
// first overload does, `string | Blob`, a form's own entry too, as React's
// Flight reply client copies a form's entries, `value.forEach((v, k) =>
// data.append(prefix + k, v))`; a Blob's file name is the form that gives
// one, so its two-argument call is the first's. Text is any text Rust
// has, `&part_json` of a `String` too, as a `&str` parameter takes it
// (rd540).
test("a form's append takes text or a Blob, an entry as it is", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-form-append");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Blob, FormData, form_data};

pub fn copied(value: &FormData, prefix: String) -> &'static FormData {
    let data = form_data::new(None, None);
    value.for_each(|original_value, original_key, _| {
        data.append(&(prefix.clone() + original_key), original_value);
    });
    data
}

pub fn parts(blob: &Blob, stream_prefix: String, part_json: String) -> &'static FormData {
    let data = form_data::new(None, None);
    data.append(&stream_prefix, &part_json);
    data.append(&stream_prefix, part_json);
    data.append("text", "C");
    data.append("blob", blob);
    data.append_with_blob("named", blob, "a.txt");
    data.set("text", blob);
    data.set_with_blob("blob", blob, "b.txt");
    data
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`value.forEach((originalValue, originalKey) => {
    data.append(prefix + originalKey, originalValue);
  });`);
  expect(js).toContain('data.append("blob", blob);');
  expect(js).toContain("data.append(streamPrefix, partJson);");
  expect(js).toContain('data.append("named", blob, "a.txt");');
  const lib = await import(join(dir, "lib.js"));
  const form = new FormData();
  form.append("a", "1");
  form.append("f", new File(["x"], "x.txt"));
  const copy: FormData = lib.copied(form, "_1_");
  expect([...copy.keys()]).toEqual(["_1_a", "_1_f"]);
  expect(copy.get("_1_a")).toBe("1");
  expect((copy.get("_1_f") as File).name).toBe("x.txt");
  const parts: FormData = lib.parts(new Blob(["y"]), "S1", "{}");
  expect([...parts.keys()]).toEqual(["S1", "S1", "text", "blob", "named"]);
  expect(parts.getAll("S1")).toEqual(["{}", "{}"]);
  expect([parts.get("text") instanceof Blob, (parts.get("blob") as File).name, (parts.get("named") as File).name]).toEqual([true, "b.txt", "a.txt"]);
});

// rd560: JS runs an iterable's `forEach` callback before `forEach` returns
// and keeps nothing of it, so webapi's `for_each` takes an `impl FnMut`, as
// `js::Map`'s and `js::Set`'s do, which may borrow and write a local, as
// React's ReactFlightActionServer writes `body.forEach((value, key) => {
// .. actionKey = key .. })`, where a `Box<dyn FnMut>`, `'static`, made the
// port loop `for (key, value) of body`.
test("an iterable's forEach takes a callback that may write a local", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-for-each-borrows");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{FormData, Headers, URLSearchParams, form_data};

pub fn action_key(body: &FormData) -> (Option<String>, &'static FormData) {
    let form_data = form_data::new(None, None);
    let mut found = None;
    body.for_each(|value, key, _| {
        if !key.starts_with("$ACTION_") {
            form_data.append(key, value);
        } else {
            found = Some(key.to_string());
        }
    });
    (found, form_data)
}

pub fn header_names(headers: &Headers) -> Vec<String> {
    let mut names = Vec::new();
    headers.for_each(|_, name, _| names.push(name.to_string()));
    names
}

pub fn total(params: &URLSearchParams) -> usize {
    let mut length = 0;
    params.for_each(|value, _, _| {
        length += value.len();
    });
    length
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  body.forEach((value, key) => {
    if (!key.startsWith("$ACTION_")) {
      formData.append(key, value);
    } else {
      found = key;
    }
  });`);
  expect(js).toContain("headers.forEach((_, name) => {");
  expect(js).toContain("params.forEach((value) => {");
  const lib = await import(join(dir, "lib.js"));
  const body = new FormData();
  body.append("a", "1");
  body.append("$ACTION_ID_x", "");
  body.append("f", new File(["x"], "x.txt"));
  const [found, copy] = lib.actionKey(body);
  expect(found).toBe("$ACTION_ID_x");
  expect([...copy.keys()]).toEqual(["a", "f"]);
  expect(lib.actionKey(new FormData())[0]).toBeUndefined();
  expect(lib.headerNames(new Headers({ b: "1", a: "2" }))).toEqual(["a", "b"]);
  expect(lib.total(new URLSearchParams("a=12&b=345"))).toBe(5);
  // As Node runs it, its own `FormData`, `Headers` and `URLSearchParams`.
  const program = `const lib = await import(${JSON.stringify(join(dir, "lib.js"))});
const body = new FormData();
body.append("a", "1");
body.append("$ACTION_ID_x", "");
const [found, copy] = lib.actionKey(body);
console.log(found, [...copy.keys()].join(), lib.headerNames(new Headers({ b: "1", a: "2" })).join(), lib.total(new URLSearchParams("a=12&b=345")));`;
  expect(run([node ?? "node", "--input-type=module", "--eval", program]).trim()).toBe("$ACTION_ID_x a a,b 5");
});

// rd574: what `forEach` gives its callback is JS's, as `get` gives it, so
// the callback may keep it: a `FormData`'s values, `File`s too, and keys,
// a `NodeList`'s nodes and the collection itself. React's Flight
// createBackingFormData keeps each entry, `formData.forEach(..)`.
test("an iterable's forEach gives values a callback may keep", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-for-each-kept");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{FormData, FormDataEntryValue, Headers, NodeList};

pub fn kept(body: &FormData) -> Vec<(&'static str, FormDataEntryValue<'static>)> {
    let mut entries = Vec::new();
    body.for_each(|value, key, _| entries.push((key, value)));
    entries
}

pub fn last_header(headers: &Headers) -> Option<(&'static str, &'static Headers)> {
    let mut last = None;
    headers.for_each(|value, _, all| last = Some((value, all)));
    last
}

pub fn nodes(list: &NodeList) -> Vec<&'static webapi::Node> {
    let mut nodes = Vec::new();
    list.for_each(|node, _, _| nodes.push(node));
    nodes
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("body.forEach((value, key) => {");
  expect(js).toContain("entries.push([key, value]);");
  const lib = await import(join(dir, "lib.js"));
  const body = new FormData();
  const file = new File(["x"], "x.txt");
  body.append("a", "1");
  body.append("f", file);
  const entries = lib.kept(body);
  expect(entries.map(([key]: [string]) => key)).toEqual(["a", "f"]);
  expect(entries[0][1]).toBe("1");
  expect(entries[1][1]).toBeInstanceOf(File);
  expect(entries[1][1].name).toBe("x.txt");
  const headers = new Headers({ a: "1", b: "2" });
  const [value, all] = lib.lastHeader(headers);
  expect(value).toBe("2");
  expect(all).toBe(headers);
  expect(lib.lastHeader(new Headers())).toBeUndefined();
  const list = { forEach: (f: (n: unknown, i: number, l: unknown) => void) => ["x", "y"].forEach((n, i) => f(n, i, list)) };
  expect(lib.nodes(list)).toEqual(["x", "y"]);
});

// rd663: what JS calls an observer's, a timer's or a request's callback
// with is JS's too, as `forEach`'s arguments are (rd574), so the callback
// may keep it: a `MutationObserver`'s records and the observer itself, an
// idle callback's deadline, `getAsString`'s text, and a `js::Map`'s
// `forEach`'s map.
test("an observer's or a timer's callback is given values it may keep", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-callback-kept");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use webapi::{
    DataTransferItem, IdleDeadline, MutationObserver, MutationRecord, ResizeObserver,
    ResizeObserverEntry, mutation_observer, resize_observer, window_global,
};

type Kept = Vec<(&'static MutationRecord, &'static MutationObserver)>;

thread_local! {
    static RECORDS: RefCell<Kept> = const { RefCell::new(Vec::new()) };
    static DEADLINE: Cell<Option<&'static IdleDeadline>> = const { Cell::new(None) };
    static TEXT: Cell<Option<&'static str>> = const { Cell::new(None) };
}

pub fn observe() -> &'static MutationObserver {
    mutation_observer::new(Box::new(|records, observer| {
        RECORDS.with_borrow_mut(|kept| kept.extend(records.iter().map(|&record| (record, observer))))
    }))
}

pub fn records() -> Kept {
    RECORDS.with_borrow(|kept| kept.clone())
}

pub fn idle() -> f64 {
    window_global::request_idle_callback(Box::new(|deadline| DEADLINE.set(Some(deadline))), None)
}

pub fn deadline() -> Option<&'static IdleDeadline> {
    DEADLINE.get()
}

pub fn read(item: &DataTransferItem) -> Option<&'static str> {
    item.get_as_string(Some(Box::new(|text| TEXT.set(Some(text)))));
    TEXT.get()
}

pub fn resized() -> &'static ResizeObserver {
    resize_observer::new(Box::new(|entries: &[&ResizeObserverEntry], _: &ResizeObserver| {
        assert!(!entries.is_empty());
    }))
}

pub fn map_of(map: &js::Map<str, f64>) -> Option<&'static js::Map<str, f64>> {
    let mut kept = None;
    map.for_each(|_, _, all| kept = Some(all));
    kept
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return new MutationObserver((records, observer) => {");
  expect(js).toContain("return requestIdleCallback((deadline) => {");
  expect(js).toContain("map.forEach((_, _$1, all) => {");
  const host = globalThis as Record<string, unknown>;
  const names = ["MutationObserver", "ResizeObserver", "requestIdleCallback"];
  const saved = names.map((name) => [name, host[name]] as const);
  class Observer {
    constructor(readonly callback: (entries: unknown[], observer: Observer) => void) {}
  }
  const idle = { timeRemaining: () => 5 };
  host.MutationObserver = Observer;
  host.ResizeObserver = Observer;
  host.requestIdleCallback = (f: (deadline: unknown) => void) => (f(idle), 7);
  try {
    const lib = await import(join(dir, "lib.js"));
    const observer = lib.observe();
    const [added, changed] = [{ type: "childList" }, { type: "attributes" }];
    observer.callback([added, changed], observer);
    const records = lib.records();
    expect(records.length).toBe(2);
    expect(records[0][0]).toBe(added);
    expect(records[1][0]).toBe(changed);
    expect(records[1][1]).toBe(observer);
    expect(lib.idle()).toBe(7);
    expect(lib.deadline()).toBe(idle);
    expect(lib.read({ getAsString: (f: (text: string) => void) => f("dropped") })).toBe("dropped");
    const resized = lib.resized();
    expect(() => resized.callback([], resized)).toThrow("assertion failed: !entries.is_empty()");
    resized.callback([{}], resized);
    const map = new Map([["a", 1]]);
    expect(lib.mapOf(map)).toBe(map);
    expect(lib.mapOf(new Map())).toBeUndefined();
  } finally {
    for (const [name, value] of saved) {
      if (value === undefined) delete host[name];
      else host[name] = value;
    }
  }
});

// rd663's follow-up: a shared listener, `webapi::listener(..)`'s, is a
// callback JS calls too, so it's given its event as JS gives it, which it
// may keep, `&'static dyn Fn(&'static E)`, as every other callback's is; a
// closure that types its parameter, and a listener typed of any borrow,
// compile as they did.
test("a shared listener is given an event it may keep", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-listener-kept");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use webapi::{Event, EventTarget, EventTargetExt, HTMLElement, PointerEvent, listener};

thread_local! {
    static LAST: Cell<Option<&'static Event>> = const { Cell::new(None) };
    static CLICKED: Cell<Option<&'static PointerEvent>> = const { Cell::new(None) };
}

pub fn listen(target: &EventTarget) -> &'static dyn Fn(&'static Event) {
    let kept = listener(|event| LAST.set(Some(event)));
    target.add_event_listener_named("custom", kept, None);
    kept
}

pub fn last() -> Option<&'static Event> {
    LAST.get()
}

pub fn clicked(button: &HTMLElement) {
    button.set_onclick(Some(&listener(|event: &PointerEvent| CLICKED.set(Some(event)))));
}

pub fn click() -> Option<&'static PointerEvent> {
    CLICKED.get()
}

pub fn once(target: &'static EventTarget, given: &'static dyn Fn(&Event)) -> &'static dyn Fn(&'static Event) {
    let shared = listener(move |event: &Event| {
        target.remove_event_listener_named("custom", given, None);
        given(event);
    });
    target.add_event_listener_named("custom", shared, None);
    shared
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  function kept(event) {\n    LAST = event;\n  }\n  target.addEventListener("custom", kept);\n  return kept;');
  expect(js).toContain("  button.onclick = (event) => {\n    CLICKED = event;\n  };");
  const lib = await import(join(dir, "lib.js"));
  const added: [string, Function][] = [];
  const removed: [string, Function][] = [];
  const host = {
    addEventListener: (type: string, f: Function) => void added.push([type, f]),
    removeEventListener: (type: string, f: Function) => void removed.push([type, f]),
  };
  const kept = lib.listen(host);
  expect(added).toEqual([["custom", kept]]);
  const event = { type: "custom" };
  kept(event);
  expect(lib.last()).toBe(event);
  const button: { onclick?: Function } = {};
  lib.clicked(button);
  const click = { type: "click" };
  button.onclick!(click);
  expect(lib.click()).toBe(click);
  const seen: unknown[] = [];
  const given = (e: unknown) => void seen.push(e);
  const shared = lib.once(host, given);
  shared(event);
  expect([seen, removed]).toEqual([[event], [["custom", given]]]);
});

// rd506's audit: of an untagged enum of one class and kinds JS tells by
// their value, a form entry's `File | string`, the class is what's left,
// each other tested, `typeof entry === "string"`, so no `instanceof File`
// runs where a host has no `File` global, Node 18 or a test's environment,
// which would throw a ReferenceError where Rust runs on (the
// coordinator's decision on the user's behalf).
test("a union of one class is told apart by the others' tests, no instanceof of a global", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-lone-class");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{FormData, FormDataEntryValue};

pub fn kind(data: &FormData, name: &str) -> u32 {
    match data.get(name) {
        Some(FormDataEntryValue::File(_)) => 1,
        Some(FormDataEntryValue::Str(_)) => 2,
        None => 0,
    }
}

pub fn file_name(data: &FormData, name: &str) -> Option<String> {
    if let Some(FormDataEntryValue::File(file)) = data.get(name) {
        return Some(file.name());
    }
    None
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("instanceof");
  expect(js).toContain('typeof get === "string"');
  const lib = await import(join(dir, "lib.js"));
  const form = (entries: Record<string, unknown>) => ({ get: (name: string) => entries[name] ?? null });
  const file = { name: "a.txt" };
  const data = form({ f: file, s: "text" });
  const File = globalThis.File;
  // @ts-ignore: a host without the global.
  delete globalThis.File;
  try {
    expect([lib.kind(data, "f"), lib.kind(data, "s"), lib.kind(data, "x")]).toEqual([1, 2, 0]);
    expect([lib.fileName(data, "f"), lib.fileName(data, "s"), lib.fileName(data, "x")]).toEqual(["a.txt", undefined, undefined]);
  } finally {
    globalThis.File = File;
  }
});

// #555: the one class beside kinds JS tells by the value alone is what
// they aren't wherever it's tested, a derived `==`'s too, `!Array.isArray(a)`,
// never `a instanceof File`, `false` of another realm's file and a
// ReferenceError where the host has no `File`.
test("a union's one class is what the others aren't in a derived ==, no instanceof", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-lone-class-eq");
  writeFileSync(join(dir, "lib.rs"), `use webapi::File;

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(PartialEq)]
pub enum Entry<'a> {
    File(&'a File),
    Items(Vec<u32>),
}

pub fn same(a: &Entry, b: &Entry) -> bool {
    a == b
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("instanceof");
  expect(js).toContain("!Array.isArray(a)\n    ? !Array.isArray(b) && a === b");
  const lib = await import(join(dir, "lib.js"));
  // A file of no class this host has, as another realm's is to its own.
  const file = { name: "a.txt" };
  expect([lib.same(file, file), lib.same(file, { name: "a.txt" }), lib.same([1, 2], [1, 2]), lib.same(file, [1])]).toEqual([true, false, true, false]);
});

// rd687 (P0): a `match` of an untagged enum whose arms each do the same
// JS, FizzTestUtils' `source.firstChild` of a `Document | Element`, is that
// JS once, no `instanceof Document`: Jest's node environment, whose DOM is a
// JSDOM window's, has no `Document` global, and the test threw a
// ReferenceError where Rust runs on. Arms that differ are still told apart.
test("an untagged enum's arms that each do the same are that JS, no instanceof of a global", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-alike-arms");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Document, Element, Node};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum DocumentOrElement<'a> {
    Document(&'a Document),
    Element(&'a Element),
}

pub fn first(source: DocumentOrElement<'_>) -> Option<&'static Node> {
    match source {
        DocumentOrElement::Document(source) => source.first_child(),
        DocumentOrElement::Element(source) => source.first_child(),
    }
}

pub fn count(source: DocumentOrElement<'_>) -> f64 {
    let scripts = match source {
        DocumentOrElement::Document(source) => source.query_selector_all("script"),
        DocumentOrElement::Element(source) => source.query_selector_all("script"),
    };
    scripts.length()
}

pub fn named(source: DocumentOrElement<'_>) -> String {
    match source {
        DocumentOrElement::Document(_) => "document".to_string(),
        DocumentOrElement::Element(element) => element.tag_name(),
    }
}

pub struct Fragment {
    pub first_child: Option<&'static Node>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ElementOrFragment<'a> {
    Element(&'a Element),
    Fragment(&'a Fragment),
}

// The same JS, \`source.firstChild\`, but a getter's read in one arm and a
// field's in the other: what's known of one isn't of the other.
pub fn first_of(source: ElementOrFragment<'_>) -> Option<&'static Node> {
    match source {
        ElementOrFragment::Element(element) => element.first_child(),
        ElementOrFragment::Fragment(fragment) => fragment.first_child,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function first(source) {\n  return source.firstChild;\n}");
  expect(js).toContain('  const scripts = source.querySelectorAll("script");\n');
  // Arms that differ are told by each node's `nodeType` (#555).
  expect(js).not.toContain("instanceof");
  expect(js).toContain('return source.nodeType === 9 ? "document" : source.tagName;');
  expect(js).toContain("return source.nodeType === 1 ? source.firstChild : source.firstChild;");
  const lib = await import(join(dir, "lib.js"));
  // A host without the global, as Jest's node environment is.
  expect(typeof (globalThis as Record<string, unknown>).Document).toBe("undefined");
  const child = { nodeType: 1 };
  const element = { firstChild: child, querySelectorAll: () => ({ length: 2 }) };
  expect([lib.first(element), lib.count(element)]).toEqual([child, 2]);
});

// #555 (P0): an untagged enum of DOM node classes whose arms differ is
// told by each node's `nodeType`, `source.nodeType === 9`, as React tells
// one. `source instanceof Document` was `false` of another realm's
// document, a frame's `contentDocument` or a JSDOM window's in a browser
// test, so its `tagName` was read, `undefined`, where Rust returns
// "document"; and it threw a ReferenceError where the host has no
// `Document` global. Two happy-dom windows are two realms: the nodes are
// one's, and the globals none, then the other's.
test("an untagged enum's DOM nodes are told by their nodeType, in any realm and with no global", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-node-types");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Document, Element, Text};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum DocumentOrElement<'a> {
    Document(&'a Document),
    Element(&'a Element),
}

pub fn named(source: DocumentOrElement<'_>) -> String {
    match source {
        DocumentOrElement::Document(_) => "document".to_string(),
        DocumentOrElement::Element(element) => element.tag_name(),
    }
}

pub fn tag(source: DocumentOrElement<'_>) -> Option<String> {
    match source {
        DocumentOrElement::Element(element) => Some(element.tag_name()),
        DocumentOrElement::Document(_) => None,
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ElementOrText<'a> {
    Element(&'a Element),
    Text(&'a Text),
}

pub fn content(node: ElementOrText<'_>) -> String {
    match node {
        ElementOrText::Text(text) => text.data(),
        ElementOrText::Element(element) => element.tag_name(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("instanceof");
  expect(js).toContain('return source.nodeType === 9 ? "document" : source.tagName;');
  expect(js).toContain("  if (source.nodeType === 1) {\n    return source.tagName;\n  }\n");
  // A `Text` may be a `CDATASection`, whose `nodeType` is 4.
  expect(js).toContain("node.nodeType === 3 || node.nodeType === 4 ? node.data : node.tagName");
  const lib = await import(join(dir, "lib.js"));
  const happyDom = Bun.resolveSync("happy-dom", realpathSync(join(root, "node_modules/@happy-dom/global-registrator")));
  const { Window } = await import(happyDom);
  const frame = new Window();
  const page = new Window();
  const doc = frame.document;
  const section = doc.createElement("section");
  const text = doc.createTextNode("hi");
  const told = () => [lib.named(doc), lib.named(section), lib.tag(doc), lib.tag(section), lib.content(text), lib.content(section)];
  const rust = ["document", "SECTION", undefined, "SECTION", "hi", "SECTION"];
  // A host without the globals, as Jest's node environment is.
  const globals = globalThis as Record<string, unknown>;
  expect(typeof globals.Document).toBe("undefined");
  expect(told()).toEqual(rust);
  // Another realm's classes: a page's, of a frame's nodes.
  Object.assign(globals, { Document: page.Document, Element: page.Element, Text: page.Text });
  try {
    expect(doc instanceof page.Document).toBe(false);
    expect(told()).toEqual(rust);
  } finally {
    for (const name of ["Document", "Element", "Text"]) delete globals[name];
  }
  await frame.happyDOM.close();
  await page.happyDOM.close();

  // Classes no `nodeType` tells apart, which only `instanceof` did: an
  // `HTMLElement` and an `SVGElement` are each 1, and a `Blob` has none.
  const refused = (name: string, source: string, message: string) => {
    const at = fixture(name);
    writeFileSync(join(at, "lib.rs"), source);
    const built = Bun.spawnSync([compiler, join(at, "lib.rs"), "-o", join(at, "lib.js"), ...withWeb]);
    expect(built.exitCode).not.toBe(0);
    expect(built.stderr.toString()).toContain(message);
    expect(built.stderr.toString()).toContain("lib.rs:");
  };
  refused(
    "webapi-node-types-alike",
    `use webapi::{HTMLElement, SVGElement};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Drawn<'a> {
    Html(&'a HTMLElement),
    Svg(&'a SVGElement),
}

pub fn is_svg(drawn: Drawn<'_>) -> bool {
    matches!(drawn, Drawn::Svg(_))
}
`,
    "`Svg` and `Html` are each an object whose `nodeType` is the number 1, which JS can't tell apart in an untagged enum in every realm",
  );
  refused(
    "webapi-node-types-untold",
    `use webapi::{Blob, Element};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Dropped<'a> {
    Element(&'a Element),
    Blob(&'a Blob),
}

pub fn size(dropped: Dropped<'_>) -> f64 {
    match dropped {
        Dropped::Blob(blob) => blob.size(),
        Dropped::Element(_) => 0.0,
    }
}
`,
    "JS can't tell `Dropped::Blob`, a `Blob`, from `Dropped`'s other variants in every realm: `instanceof Blob` is `false` of a `Blob` of another realm",
  );
  // Its element tested first, by its `nodeType`, the blob what's left: no
  // `instanceof` made, nothing refused.
  const first = fixture("webapi-node-types-element-first");
  writeFileSync(join(first, "lib.rs"), `use webapi::{Blob, Element};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Dropped<'a> {
    Element(&'a Element),
    Blob(&'a Blob),
}

pub fn size(dropped: Dropped<'_>) -> f64 {
    match dropped {
        Dropped::Element(_) => 0.0,
        Dropped::Blob(blob) => blob.size(),
    }
}
`);
  run([compiler, join(first, "lib.rs"), "-o", join(first, "lib.js"), ...withWeb]);
  expect(readFileSync(join(first, "lib.js"), "utf8")).toContain("return dropped.nodeType === 1 ? 0 : dropped.size;");
});

// React DOM's compareDocumentPositionForEmptyFragment tests a position
// against `Node.DOCUMENT_POSITION_CONTAINED_BY`: webapi's constants are
// WebIDL's `const unsigned short`, `u16`, each read by its JS name, a
// widening `as u32` the constant itself (ADRs 0031, 0994, amended, rd618).
test("a webapi constant is read by its JS name, as WebIDL's integer", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-constants");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Node, node};

pub fn contained(position: u32) -> bool {
    position & node::DOCUMENT_POSITION_CONTAINED_BY as u32 != 0
}

pub fn disconnected() -> f64 {
    node::DOCUMENT_POSITION_DISCONNECTED as f64
}

pub fn kind(of: &Node) -> &'static str {
    match of.node_type() as u16 {
        node::ELEMENT_NODE => "element",
        node::TEXT_NODE => "text",
        _ => "other",
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return (position & Node.DOCUMENT_POSITION_CONTAINED_BY) >>> 0 !== 0;\n");
  expect(js).toContain("  return Node.DOCUMENT_POSITION_DISCONNECTED;\n");
  expect(js).toContain("    case Node.ELEMENT_NODE:\n");
  const Node = { DOCUMENT_POSITION_CONTAINED_BY: 16, DOCUMENT_POSITION_DISCONNECTED: 1, ELEMENT_NODE: 1, TEXT_NODE: 3 };
  const lib = await import(join(dir, "lib.js"));
  const saved = (globalThis as Record<string, unknown>).Node;
  (globalThis as Record<string, unknown>).Node = Node;
  try {
    expect([lib.contained(20), lib.contained(4), lib.disconnected()]).toEqual([true, false, 1]);
    expect([lib.kind({ nodeType: 1 }), lib.kind({ nodeType: 3 }), lib.kind({ nodeType: 8 })]).toEqual(["element", "text", "other"]);
  } finally {
    (globalThis as Record<string, unknown>).Node = saved;
  }
});
