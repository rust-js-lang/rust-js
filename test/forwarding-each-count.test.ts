// A private function that only calls a thread-local's field, of a trailing
// `Option` known where it runs, is that field where the crate fills it
// with its own functions alone, which read no `arguments.length`: its call
// of each count (#1876) is the call of all to them, so the value is taken
// apart as alien-signals writes it, `const { link, unlink } =
// createReactiveSystem(0)` (ADR 0610, amended, #1890). One filled with a
// JS function, `Date.UTC`, which counts what it's given, stays a function
// of each count. In Bun and in Chromium.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";
import { buildReact, compiler, fixture, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

const flags = ["--", "--crate-type=lib", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const program = `use std::rc::Rc;

pub struct ReactiveSystem {
    pub link: Rc<dyn Fn(u32, u32) -> u32>,
    pub unlink: Rc<dyn Fn(u32, Option<u32>) -> Option<u32>>,
}

pub fn create_reactive_system(base: u32) -> ReactiveSystem {
    ReactiveSystem {
        link: Rc::new(move |dep, sub| base + dep + sub),
        unlink: Rc::new(|link, sub| sub.map(|sub| sub + link)),
    }
}

type Utc = dyn Fn(i32, Option<i32>, Option<i32>, Option<i32>, Option<i32>, Option<i32>, Option<i32>) -> f64;

pub struct Clock {
    pub utc: Rc<Utc>,
}

thread_local! {
    static SYSTEM: ReactiveSystem = create_reactive_system(0);
    static CLOCK: Clock = Clock {
        utc: Rc::new(js::date::utc),
    };
}

fn link(dep: u32, sub: u32) -> u32 {
    SYSTEM.with(|system| (system.link)(dep, sub))
}

fn unlink(link: u32, sub: Option<u32>) -> Option<u32> {
    SYSTEM.with(|system| (system.unlink)(link, sub))
}

fn utc(
    year: i32,
    month: Option<i32>,
    day: Option<i32>,
    hours: Option<i32>,
    minutes: Option<i32>,
    seconds: Option<i32>,
    ms: Option<i32>,
) -> f64 {
    CLOCK.with(|clock| (clock.utc)(year, month, day, hours, minutes, seconds, ms))
}

pub fn run(sub: Option<u32>, month: Option<i32>) -> Vec<f64> {
    vec![
        f64::from(link(1, 2)),
        unlink(3, sub).map_or(-1.0, f64::from),
        utc(2020, month, None, None, None, None, None),
    ]
}
`;

let compiled: string | undefined;
const compile = () => {
  if (compiled) return compiled;
  const dir = fixture("forwarding-each-count");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags]);
  compiled = readFileSync(join(dir, "lib.js"), "utf8");
  return compiled;
};

// Each `None` and `Some`: `Date.UTC(2020)` is 2020's first moment, and
// `Date.UTC(2020, undefined)` would be `NaN`.
const expected = [
  [[null, null], [3, -1, Date.UTC(2020)]],
  [[4, 1], [3, 7, Date.UTC(2020, 1)]],
];

test("a thread-local's field of the crate's own functions is that field, its call of each count the call of all", async () => {
  const js = compile();
  expect(js).toContain("const { link, unlink } = createReactiveSystem(0);\n");
  expect(js).not.toContain("function unlink(");
  // A field of a JS function, which counts what it's given: each count.
  expect(js).toContain("function utc(");
  expect(js).toContain("month != null");
  const dir = fixture("forwarding-each-count-run");
  writeFileSync(join(dir, "lib.js"), js);
  const lib = await import(join(dir, "lib.js"));
  for (const [[sub, month], result] of expected) {
    expect(lib.run(sub ?? undefined, month ?? undefined)).toEqual(result);
  }
}, underLoad(120_000));

test("a thread-local's field taken apart, and one of a JS function kept, run as Rust does in Chromium", async () => {
  const js = compile();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    const results = await page.evaluate(async ([js, cases]) => {
      const url = URL.createObjectURL(new Blob([js], { type: "text/javascript" }));
      const lib = await import(url);
      return cases.map(([[sub, month]]) => lib.run(sub ?? undefined, month ?? undefined));
    }, [js, expected] as const);
    expect(results).toEqual(expected.map(([, result]) => result));
  } finally {
    await browser.close();
  }
}, underLoad(120_000));
