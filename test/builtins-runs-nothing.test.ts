// A builtin that runs none of the program's code, `#[rust_js::runs_nothing]`
// (ADR 0867), or only what it's given, `#[rust_js::runs_only_given]` (ADR
// 0898): a `RefCell` borrowed across its call isn't counted, so a memo whose
// maker calls only such builtins reads its cache plainly. The marks of
// generated builtins are builtins/policy.ts' `RUNS`, each with its spec step;
// `RegExp`'s are written by hand (#2082). A constructor that runs only what
// it's given runs nothing given `None` where its options' type is an object a
// JS caller could hand over, but not given that object. One whose maker calls
// the program back, a JS callback, keeps the check, and its re-entry panics
// as Rust's.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const memo = (maker: string, signature = "key: &str", key = "key") => `use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

pub fn memo(${signature}) -> f64 {
    CACHE.with_borrow_mut(|cache| *cache.entry(${key}.to_string()).or_insert_with(|| ${maker}))
}
`;

async function compiled(name: string, source: string) {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

test("a memo whose maker runs only a regex reads its cache plainly", async () => {
  const { js, lib } = await compiled("runs-nothing-regex", memo(`js::reg_exp::new("^de", "i").test(key) as u8 as f64`));
  expect(js).not.toContain("$withBorrowMut");
  expect([lib.memo("de-DE"), lib.memo("en"), lib.memo("de-DE")]).toEqual([1, 0, 1]);
});

// A builtin's method is its own only of what a builtin made (#2111): of a
// JS caller's locale, a subclass's override may call back into the memo.
test("a memo whose maker calls a JS caller's locale's method keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-locale",
    memo("tag.maximize().to_string().len() as f64", "tag: &js::intl::Locale", "tag"),
  );
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo(new Intl.Locale("en"))).toBe(new Intl.Locale("en").maximize().toString().length);
  // The override's re-entry meets the held borrow, as Rust's would.
  class Hand extends Intl.Locale {
    maximize() {
      lib.memo(new Intl.Locale("de"));
      return super.maximize();
    }
  }
  expect(() => lib.memo(new Hand("fr"))).toThrow(/already/);
});

test("a memo whose maker calls a method of a locale it made, through a private function, reads its cache plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-locale-made",
    `use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

fn maximized(tag: &js::intl::Locale) -> f64 {
    tag.maximize().to_string().len() as f64
}

pub fn memo(key: &str) -> f64 {
    CACHE.with_borrow_mut(|cache| {
        *cache
            .entry(key.to_string())
            .or_insert_with(|| js::intl::locale::new(key, None).map_or(0.0, |tag| maximized(tag)))
    })
}
`,
  );
  expect(js).not.toContain("$withBorrowMut");
  expect(lib.memo("en")).toBe(new Intl.Locale("en").maximize().toString().length);
});

test("a private function also used as a value keeps its parameter's borrow checked", async () => {
  const { js } = await compiled(
    "runs-nothing-locale-escapes",
    `use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

fn maximized(tag: &js::intl::Locale) -> f64 {
    tag.maximize().to_string().len() as f64
}

pub fn measure() -> fn(&js::intl::Locale) -> f64 {
    maximized
}

pub fn memo(key: &str) -> f64 {
    CACHE.with_borrow_mut(|cache| {
        *cache
            .entry(key.to_string())
            .or_insert_with(|| js::intl::locale::new(key, None).map_or(0.0, |tag| maximized(tag)))
    })
}
`,
  );
  expect(js).toContain("$withBorrowMut");
});

test("a thread-local map a builtin made is read plainly while a cell is borrowed", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-thread-local-map",
    `use std::cell::RefCell;

thread_local! {
    static SEEN: &'static js::Map<String, f64> = js::map::new(None);
    static NOTES: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

pub fn note(key: &str) -> usize {
    NOTES.with_borrow_mut(|notes| {
        let key = key.to_string();
        if !SEEN.with(|seen| seen.has(&key)) {
            SEEN.with(|seen| seen.set(&key, 1.0));
            notes.push(1.0);
        }
        notes.len()
    })
}
`,
  );
  expect(js).not.toContain("$withBorrowMut");
  expect([lib.note("a"), lib.note("a"), lib.note("b")]).toEqual([1, 1, 2]);
});

test("a memo whose maker calls the program back keeps its borrow checked, and its re-entry panics", async () => {
  const { js, lib } = await compiled("runs-nothing-callback", memo("make()", "key: &str, make: &dyn Fn() -> f64"));
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo("a", () => 2)).toBe(2);
  expect(() => lib.memo("b", () => lib.memo("c", () => 3))).toThrow("already borrowed");
});

test("a memo whose maker makes a locale and a date format of no options reads its cache plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-intl-none",
    memo(
      `js::intl::locale::new(key, None).map_or(0.0, |tag| tag.to_string().len() as f64) + js::intl::date_time_format::new(None, None).resolved_options().locale.len() as f64`,
    ),
  );
  expect(js).not.toContain("$withBorrowMut");
  const runtime = new Intl.DateTimeFormat().resolvedOptions().locale.length;
  expect(lib.memo("de-DE")).toBe(5 + runtime);
  expect(lib.memo("not a tag!")).toBe(runtime);
});

test("a memo whose maker gives a locale a JS caller's options keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-intl-given",
    memo(
      `js::intl::locale::new(key, Some(options)).map_or(0.0, |tag| tag.to_string().len() as f64)`,
      "key: &str, options: &js::intl::LocaleOptions",
    ),
  );
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo("en", { region: "GB" })).toBe("en-GB".length);
});

// `js::optional_call((tag, Locale::get_week_info), ())`, `tag.getWeekInfo?.()`,
// runs what its method runs, as a plain call of it does: of a JS caller's
// locale, whose subclass may override it, counted (#2111); not a method that
// isn't marked, a formatter's `format`.
test("a memo whose maker calls a JS caller's locale's method optionally keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-optional-call",
    memo(
      "js::optional_call((tag, js::intl::Locale::get_week_info), ()).map_or(0.0, |info| info.first_day)",
      "tag: &js::intl::Locale",
      "tag",
    ),
  );
  expect(js).toContain("$withBorrowMut");
  const tag = new Intl.Locale("en-US") as any;
  const expected = tag.getWeekInfo?.()?.firstDay ?? 0;
  expect(lib.memo(tag)).toBe(expected);
  // A JS caller's subclass whose override calls back meets the held borrow.
  class Hand extends Intl.Locale {
    getWeekInfo() {
      lib.memo(new Intl.Locale("de"));
      return { firstDay: 1, weekend: [6, 7], minimalDays: 1 };
    }
  }
  expect(() => lib.memo(new Hand("fr"))).toThrow(/already/);
});

test("a memo whose maker calls a method of a locale it made optionally reads its cache plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-optional-call-made",
    memo(
      "js::intl::locale::new(key, None).map_or(0.0, |tag| js::optional_call((tag, js::intl::Locale::to_string), ()).map_or(0.0, |text| text.len() as f64))",
    ),
  );
  expect(js).not.toContain("$withBorrowMut");
  expect(lib.memo("en-US")).toBe(5);
});

test("a memo whose maker calls a method not marked optionally keeps its borrow checked", async () => {
  const { js } = await compiled(
    "runs-nothing-optional-call-given",
    memo(
      "js::optional_call((format, js::intl::DateTimeFormat::format), (Some(&now as &dyn js::intl::IntoDateOrNumber),)).map_or(0.0, |text| text.len() as f64)",
      "key: &str, format: &js::intl::DateTimeFormat, now: f64",
    ),
  );
  expect(js).toContain("$withBorrowMut");
});

// `optional_call_unchecked`'s value is asserted of another contract, so the
// method its path names may not be the binding's: its call is counted.
test("a memo whose maker calls a method optionally, unchecked, keeps its borrow checked", async () => {
  const { js } = await compiled(
    "runs-nothing-optional-call-unchecked",
    memo(
      "js::intl::locale::new(key, None).map_or(0.0, |tag| unsafe { js::optional_call_unchecked((tag, js::intl::Locale::to_string), ()) }.map_or(0.0, |text| text.len() as f64))",
    ),
  );
  expect(js).toContain("$withBorrowMut");
});

// A property read, `tag.region`, runs nothing only of a value a builtin
// made, at hand: of a JS caller's, whose getter may call back into the
// program, it's counted (#2106); of a regex's match `exec` made, passed to
// `and_then`'s closure, it isn't; of what a `Map` kept, a JS caller's value
// whatever made the map, it's counted.
test("a memo whose maker reads a JS caller's locale keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-given-locale",
    memo("tag.region().map_or(0.0, |region| region.len() as f64)", "tag: &js::intl::Locale", "tag"),
  );
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo(new Intl.Locale("en-GB"))).toBe(2);
});

test("a memo whose maker reads the match exec made reads its cache plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-exec-match",
    memo(`js::reg_exp::new("^(de)", "i").exec(key).and_then(|matched| matched.get(1)).map_or(0.0, |part| part.len() as f64)`),
  );
  expect(js).not.toContain("$withBorrowMut");
  expect([lib.memo("de-DE"), lib.memo("en")]).toEqual([2, 0]);
});

test("a memo whose maker reads what a map kept keeps its borrow checked", async () => {
  const { js } = await compiled(
    "runs-nothing-map-kept",
    memo(
      `{
            let kept: &js::Map<str, &js::intl::Locale> = js::map::new(None);
            kept.set(key, tag);
            kept.get(key).and_then(|tag| tag.region()).map_or(0.0, |region| region.len() as f64)
        }`,
      "key: &str, tag: &'static js::intl::Locale",
    ),
  );
  expect(js).toContain("$withBorrowMut");
});

test("a guard held over a JS caller's locale's read keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-guard-given-locale",
    `use std::cell::RefCell;

pub struct Seen {
    regions: RefCell<Vec<f64>>,
}

impl Seen {
    pub fn new() -> Seen {
        Seen { regions: RefCell::new(Vec::new()) }
    }

    pub fn note(&self, tag: &js::intl::Locale) -> usize {
        let mut regions = self.regions.borrow_mut();
        regions.push(tag.region().map_or(0.0, |region| region.len() as f64));
        regions.len()
    }
}
`,
  );
  expect(js).toContain("$borrowMut");
  const seen = lib.Seen.new();
  expect(lib.Seen.note(seen, new Intl.Locale("en-GB"))).toBe(1);
});

// Of the program's own struct, cast to a JS value, a property read is
// counted where JS may give one (#2109): an exported function takes it and
// the crate never builds one, so a JS caller's object, whose getter may
// call back, is what it reads. Of a struct only the crate constructs, which
// JS gives back as the crate gave it, it runs nothing (ADR 0262's
// difference 60).
const chunked = (built: string) => `use std::cell::RefCell;

pub struct Chunk {
    pub reason: String,
}

// SAFETY: a struct of the program's is a JS object, never \`undefined\`.
unsafe impl js::Defined for Chunk {}
${built}
#[derive(Default)]
pub struct Seen {
    notes: RefCell<Vec<f64>>,
}

impl Seen {
    pub fn new() -> Seen {
        Seen::default()
    }

    pub fn note(&self, chunk: &Chunk) -> usize {
        let mut notes = self.notes.borrow_mut();
        if js::truthy(js::get(js::unknown(chunk), "reason")) {
            notes.push(1.0);
        }
        notes.len()
    }
}
`;

test("a read of the program's own struct a JS caller gives, cast to a JS value, keeps its borrow checked", async () => {
  const { js, lib } = await compiled("runs-nothing-own-struct", chunked(""));
  expect(js).toContain("$borrowMut");
  const seen = lib.Seen.new();
  expect(lib.Seen.note(seen, { reason: "x" })).toBe(1);
  // A getter that notes again meets the held borrow, as Rust's would.
  const forged = {
    get reason() {
      return lib.Seen.note(seen, { reason: "y" });
    },
  };
  expect(() => lib.Seen.note(seen, forged)).toThrow(/already/);
});

test("a read of a struct only the crate constructs, cast to a JS value, is plain", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-built-struct",
    chunked(`
pub fn chunk(reason: String) -> Chunk {
    Chunk { reason }
}
`),
  );
  expect(js).not.toContain("$borrowMut");
  expect(lib.Seen.note(lib.Seen.new(), lib.chunk("x"))).toBe(1);
});

// A property read of a type parameter's value is its caller's to settle
// (#2109), as a drop of one is: a heap of tasks only the crate constructs
// reads `sortIndex` plainly under its queue's borrow; one of tasks a JS
// caller gives keeps the check.
const heap = (task: string, schedule: string) => `use std::cell::RefCell;
use std::rc::Rc;

pub trait Node {
    #[cfg_attr(rust_js, rust_js::link_name = "get sortIndex")]
    fn sort_index(&self) -> f64 {
        unreachable!()
    }
}

pub struct Task {
    pub sort_index: f64,
}

// SAFETY: a struct of the program's is a JS object, never \`undefined\`.
unsafe impl js::Defined for Task {}

impl Node for Rc<Task> {}

fn smallest<T: Node>(heap: &[T]) -> f64 {
    heap.iter().map(|node| node.sort_index()).fold(f64::INFINITY, f64::min)
}

thread_local! {
    static QUEUE: RefCell<Vec<Rc<Task>>> = const { RefCell::new(Vec::new()) };
}

pub fn schedule(${schedule}) -> f64 {
    QUEUE.with_borrow_mut(|queue| {
        queue.push(${task});
        smallest(queue)
    })
}
`;

test("a generic read of a task only the crate constructs is plain", async () => {
  const { js, lib } = await compiled("runs-nothing-generic-built", heap("Rc::new(Task { sort_index })", "sort_index: f64"));
  expect(js).not.toContain("$withBorrowMut");
  expect([lib.schedule(3), lib.schedule(1), lib.schedule(2)]).toEqual([3, 1, 1]);
});

test("a generic read of a task a JS caller gives keeps its borrow checked", async () => {
  const { js, lib } = await compiled("runs-nothing-generic-given", heap("task", "task: Rc<Task>"));
  expect(js).toContain("$withBorrowMut");
  expect(lib.schedule({ sortIndex: 3 })).toBe(3);
  const forged = {
    get sortIndex() {
      return lib.schedule({ sortIndex: 1 });
    },
  };
  expect(() => lib.schedule(forged)).toThrow(/already/);
});

test("a read of a JS value a caller gave, under a guard, keeps its borrow checked", async () => {
  const { js } = await compiled("runs-nothing-given-unknown", `use std::cell::RefCell;

pub struct Chunk {
    pub reason: String,
}

// SAFETY: a struct of the program's is a JS object, never \`undefined\`.
unsafe impl js::Defined for Chunk {}

pub struct Seen {
    notes: RefCell<Vec<f64>>,
}

impl Seen {
    pub fn new() -> Seen {
        Seen { notes: RefCell::new(Vec::new()) }
    }

    pub fn note(&self, chunk: &js::Unknown) -> usize {
        let mut notes = self.notes.borrow_mut();
        if js::truthy(js::get(chunk, "reason")) {
            notes.push(1.0);
        }
        notes.len()
    }
}
`);
  expect(js).toContain("$borrowMut");
});

// A variable is at hand only where every write of it is (#2110): a JS
// caller's locale assigned over one a builtin made, or written through a
// loan, keeps the memo's borrow checked.

test("a variable a JS caller's locale is assigned to keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-reassigned",
    memo(
      `{
            let mut locale = js::intl::locale::new("en-GB", None).unwrap();
            if given {
                locale = tag;
            }
            locale.region().map_or(0.0, |region| region.len() as f64)
        }`,
      "key: &str, tag: &'static js::intl::Locale, given: bool",
    ),
  );
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo("k", new Intl.Locale("en-GB"), true)).toBe(2);
});

test("a variable a closure assigns a JS caller's locale to keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-reassigned-in-closure",
    memo(
      `{
            let mut locale = js::intl::locale::new("en-GB", None).unwrap();
            let mut read = move || {
                if given {
                    locale = tag;
                }
                locale.region().map_or(0.0, |region| region.len() as f64)
            };
            read()
        }`,
      "key: &str, tag: &'static js::intl::Locale, given: bool",
    ),
  );
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo("k", new Intl.Locale("en-GB"), true)).toBe(2);
});

test("a variable lent mutably keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-lent",
    memo(
      `{
            let mut locale = js::intl::locale::new("en-GB", None).unwrap();
            let mut other = tag;
            if given {
                std::mem::swap(&mut locale, &mut other);
            }
            locale.region().map_or(0.0, |region| region.len() as f64)
        }`,
      "key: &str, tag: &'static js::intl::Locale, given: bool",
    ),
  );
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo("k", new Intl.Locale("en-GB"), true)).toBe(2);
});

test("a variable bound by ref mut keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-ref-mut",
    memo(
      `{
            let mut locale = js::intl::locale::new("en-GB", None).unwrap();
            match locale {
                ref mut slot if given => *slot = tag,
                _ => {}
            }
            locale.region().map_or(0.0, |region| region.len() as f64)
        }`,
      "key: &str, tag: &'static js::intl::Locale, given: bool",
    ),
  );
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo("k", new Intl.Locale("en-GB"), true)).toBe(2);
});


// A Rust field read of a struct JS may make or give is JS's own property
// read, which a getter may answer (#2113): of a JS caller's value cast to
// the program's struct, counted; of one the crate builds, which JS gives
// back as the crate gave it, plain (ADR 0262's difference 60).
const fielded = (note: string, built: string) => `use std::cell::RefCell;

pub struct Chunk {
    pub reason: String,
}

// SAFETY: a struct of the program's is a JS object, never \`undefined\`.
unsafe impl js::Defined for Chunk {}
${built}
#[derive(Default)]
pub struct Seen {
    notes: RefCell<Vec<f64>>,
}

impl Seen {
    pub fn new() -> Seen {
        Seen::default()
    }

${note}
}
`;

test("a field read of a JS caller's value cast to the program's struct keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-field-given",
    fielded(
      `    pub fn note(&self, value: &js::Unknown) -> usize {
        // SAFETY: the caller gives a chunk.
        let chunk: &Chunk = unsafe { js::cast(Some(value)) };
        let mut notes = self.notes.borrow_mut();
        if !chunk.reason.is_empty() {
            notes.push(1.0);
        }
        notes.len()
    }`,
      "",
    ),
  );
  expect(js).toContain("$borrowMut");
  const seen = lib.Seen.new();
  expect(lib.Seen.note(seen, { reason: "x" })).toBe(1);
  const forged = {
    get reason() {
      lib.Seen.note(seen, { reason: "y" });
      return "x";
    },
  };
  expect(() => lib.Seen.note(seen, forged)).toThrow(/already/);
});

test("a field read of a struct only the crate constructs is plain", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-field-built",
    fielded(
      `    pub fn note(&self, chunk: &Chunk) -> usize {
        let mut notes = self.notes.borrow_mut();
        if !chunk.reason.is_empty() {
            notes.push(1.0);
        }
        notes.len()
    }`,
      `
pub fn chunk(reason: String) -> Chunk {
    Chunk { reason }
}
`,
    ),
  );
  expect(js).not.toContain("$borrowMut");
  expect(lib.Seen.note(lib.Seen.new(), lib.chunk("x"))).toBe(1);
});

test("a field read in a helper, of a JS caller's value cast to the program's struct, keeps its caller's borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-field-helper",
    fielded(
      `    pub fn note(&self, value: &js::Unknown) -> usize {
        let mut notes = self.notes.borrow_mut();
        if has_reason(value) {
            notes.push(1.0);
        }
        notes.len()
    }`,
      `
pub fn has_reason(value: &js::Unknown) -> bool {
    // SAFETY: the caller gives a chunk.
    let chunk: &Chunk = unsafe { js::cast(Some(value)) };
    !chunk.reason.is_empty()
}
`,
    ),
  );
  expect(js).toContain("$borrowMut");
  const seen = lib.Seen.new();
  const forged = {
    get reason() {
      lib.Seen.note(seen, { reason: "y" });
      return "x";
    },
  };
  expect(() => lib.Seen.note(seen, forged)).toThrow(/already/);
});

// A cast of what's at hand is that value passed on (#2111): a locale the
// memo made, read as the program's struct, is read plainly.
test("a field read of a locale the memo made, cast to the program's struct, reads its cache plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-field-cast-made",
    `use std::cell::RefCell;
use std::collections::HashMap;

pub struct Named {
    pub language: String,
}

// SAFETY: a struct of the program's is a JS object, never \`undefined\`.
unsafe impl js::Defined for Named {}

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

pub fn memo(key: &str) -> f64 {
    CACHE.with_borrow_mut(|cache| {
        *cache.entry(key.to_string()).or_insert_with(|| {
            js::intl::locale::new(key, None).map_or(0.0, |tag| {
                // SAFETY: a locale has a \`language\`.
                let named: &Named = unsafe { js::cast(Some(js::unknown(tag))) };
                named.language.len() as f64
            })
        })
    })
}
`,
  );
  expect(js).not.toContain("$withBorrowMut");
  expect(lib.memo("de-DE")).toBe(2);
});

// The place a borrow is of runs before its guard is made: a field read of
// it, a JS caller's struct's cell, \`form.action.borrow()\`, can't meet that
// guard (#2113), so the guard is held plainly.
test("a borrow of a JS caller's struct's cell field holds its guard plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-field-own-cell",
    `use std::cell::RefCell;

pub struct Form {
    pub action: RefCell<String>,
}

// SAFETY: a struct of the program's is a JS object, never \`undefined\`.
unsafe impl js::Defined for Form {}

pub fn action(value: &js::Unknown) -> usize {
    // SAFETY: the caller gives a form.
    let form: &Form = unsafe { js::cast(Some(value)) };
    form.action.borrow().clone().len()
}
`,
  );
  expect(js).not.toContain("$borrow(");
  expect(lib.action({ action: "go" })).toBe(2);
});

