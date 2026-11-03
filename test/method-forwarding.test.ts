// What the compiler writes of a method that only passes its parameters on
// to a function of the crate's (ADR 0605's forwarding family, ADR 0262's
// forwarding exception): a namespace's, of its own module's function, is
// that function (#566), and one its module names apart from that name is
// refused, as a call through the namespace would be the function's
// (#585); a constructor function's prototype's is the function itself,
// `X.prototype.m = f;` (rd715). Each test is a program of its own,
// compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compiler, fixture, root, run } from "./support";

/** The crate `lib`, of the files `files`, compiled with its declarations. */
function compiled(name: string, files: Record<string, string>) {
  const dir = fixture(name);
  for (const [file, text] of Object.entries(files)) writeFileSync(join(dir, file), text);
  const p = runSync([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type", "lib"], root, 60_000);
  const read = (file: string) => readFileSync(join(dir, file), "utf8");
  return { p, dir, read };
}

const client = `pub struct Response {
    pub n: f64,
}

pub fn close(response: &Response) -> f64 {
    response.n + 1.0
}

pub fn create_response(n: f64) -> Response {
    Response { n }
}

#[cfg_attr(rust_js, rust_js::namespace)]
pub struct ReactFlightClient;

#[cfg_attr(rust_js, rust_js::namespace)]
impl ReactFlightClient {
    pub fn close(&self, response: &Response) -> f64 {
        close(response)
    }

    pub fn create_response(&self, n: f64) -> Response {
        create_response(n)
    }
}

pub fn inside(response: &Response) -> f64 {
    ReactFlightClient.close(response)
}
`;

const markup = `use crate::client::{ReactFlightClient, close as close_flight, create_response as create_flight_response};

pub fn render(n: f64) -> f64 {
    let response = create_flight_response(n);
    close_flight(&response) + ReactFlightClient.close(&response)
}

pub fn through(client: &ReactFlightClient, n: f64) -> f64 {
    let response = client.create_response(n);
    client.close(&response)
}

pub fn given(n: f64) -> f64 {
    through(&ReactFlightClient, n)
}
`;

// #566: React's ReactNoopFlightClient calls the Flight client's functions
// through the instance its factory makes, a namespace, and ReactMarkupServer
// imports them renamed, `close as closeFlight`. A namespace method that only
// passes its parameters on to its own module's function of its name is that
// function: it prints nothing of its own, its callers keep their imports and
// aliases, and the namespace's value, the module, holds the function itself.
test("a namespace's method that only passes its parameters on to its module's function is that function", async () => {
  const { p, dir, read } = compiled("namespace-own-forwarding", {
    "lib.rs": "pub mod client;\npub mod markup;\n",
    "client.rs": client,
    "markup.rs": markup,
  });
  expect(p.stderr).not.toContain("error");
  expect(p.code).toBe(0);
  const js = read("client.js");
  expect(js).not.toContain("close$1");
  expect(js).not.toContain("createResponse$1");
  expect(js).toContain("export function inside(response) {\n  return close(response);\n}");
  const markupJs = read("markup.js");
  expect(markupJs).toContain('import { close as closeFlight, createResponse as createFlightResponse } from "./client.js";');
  expect(markupJs).toContain("return closeFlight(response) + closeFlight(response);");
  expect(markupJs).toContain("const response = client.createResponse(n);\n  return client.close(response);");
  expect(read("client.d.ts")).not.toContain("close$1");
  const ran = run(["node", "--input-type=module", "--eval", `
    import * as client from ${JSON.stringify(join(dir, "client.js"))};
    import * as markup from ${JSON.stringify(join(dir, "markup.js"))};
    console.log(JSON.stringify([client.inside({ n: 1 }), markup.render(2), markup.through(client, 3), markup.given(4)]));
  `]);
  expect(ran.trim()).toBe("[2,6,4,5]");
});

// #585: a namespace's method its module names apart from a function of its
// name, `close$1`, was called through the namespace's value as `.close`,
// that function: a wrong answer. Refused, naming both and what to write;
// one of a private function's name too, which the method's export loses.
test("a namespace's method its module names apart is refused", async () => {
  const doing = client.replace("        close(response)\n", "        close(response) * 10.0\n")
    .replace("pub fn create_response(n: f64)", "fn create_response(n: f64)")
    .replace("        create_response(n)\n", "        create_response(n * 2.0)\n");
  const { p } = compiled("namespace-renamed-refused", {
    "lib.rs": "pub mod client;\npub mod markup;\n",
    "client.rs": doing,
    "markup.rs": markup.replace(", close as close_flight, create_response as create_flight_response", "")
      .replace("    let response = create_flight_response(n);\n    close_flight(&response) + ReactFlightClient.close(&response)\n", "    n\n"),
  });
  expect(p.code).not.toBe(0);
  expect(p.stderr).toContain(
    "rust-js: the namespace's method `client::ReactFlightClient::close` and `client::close` are both `close` in JS, and a call through the namespace reads its module's `close`, which would be `client::close`; give one another name with `#[rust_js::name = \"..\"]`, or have the method only pass its parameters on to `client::close`, which it then is",
  );
  expect(p.stderr).toContain("`client::ReactFlightClient::create_response` and `client::create_response` are both `createResponse` in JS");
});

// rd715: React's ReactDOMRoot writes `ReactDOMHydrationRoot.prototype.
// unstable_scheduleHydration = scheduleHydration;`. A constructor
// function's prototype method that only passes its parameters on to a
// function of the crate's, its result that function's, is that function,
// which reads no `this`; one that does more, or gives another result,
// stays a function of its own.
test("a prototype's method that only passes its parameters on is the function it calls", async () => {
  const { p, dir, read } = compiled("prototype-forwarding", {
    "lib.rs": `pub struct Seen {
    pub targets: std::cell::RefCell<Vec<f64>>,
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct HydrationRoot {
    #[cfg_attr(rust_js, rust_js::name = "_internalRoot")]
    pub internal_root: f64,
}

#[cfg_attr(rust_js, rust_js::class)]
impl HydrationRoot {
    pub fn new(internal_root: f64) -> Self {
        HydrationRoot { internal_root }
    }

    #[cfg_attr(rust_js, rust_js::name = "unstable_scheduleHydration")]
    pub fn unstable_schedule_hydration(&self, seen: &Seen, target: Option<f64>) {
        schedule_hydration(seen, target);
    }

    pub fn twice(&self, n: f64) -> f64 {
        doubled(n)
    }

    pub fn offset(&self, n: f64) -> f64 {
        doubled(n) + self.internal_root
    }

    pub fn note(&self, n: f64) {
        doubled(n);
    }
}

fn schedule_hydration(seen: &Seen, target: Option<f64>) {
    if let Some(target) = target {
        seen.targets.borrow_mut().push(target);
    }
}

fn doubled(n: f64) -> f64 {
    n * 2.0
}

pub fn run(n: f64) -> (f64, f64, Vec<f64>) {
    let root = HydrationRoot::new(n);
    let seen = Seen {
        targets: std::cell::RefCell::new(Vec::new()),
    };
    root.unstable_schedule_hydration(&seen, Some(n));
    root.unstable_schedule_hydration(&seen, None);
    (root.twice(n), root.offset(n), seen.targets.into_inner())
}
`,
  });
  expect(p.stderr).not.toContain("error");
  expect(p.code).toBe(0);
  const js = read("lib.js");
  expect(js).toContain("HydrationRoot.prototype.unstable_scheduleHydration = scheduleHydration;\n");
  expect(js).toContain("HydrationRoot.prototype.twice = doubled;\n");
  expect(js).toContain("HydrationRoot.prototype.offset = function (n) {");
  // Of `()`, its result isn't the function's.
  expect(js).toContain("HydrationRoot.prototype.note = function (n) {");
  const ran = run(["node", "--input-type=module", "--eval", `
    import { run } from ${JSON.stringify(join(dir, "lib.js"))};
    console.log(JSON.stringify(run(3)));
  `]);
  expect(ran.trim()).toBe("[6,9,[3]]");
});

// rd715: a prototype's property is read where its module loads, so the
// function it's given is its own module's, a declaration hoisted there:
// not another module's, which a cycle may not have loaded yet, and not a
// function its module would make the variable it calls (ADR 0610), which
// the load would read before it's set.
test("a prototype's method passes its parameters on to a declaration its module's load can read", async () => {
  const { p, dir, read } = compiled("prototype-forwarding-loaded", {
    "lib.rs": `pub mod other {
    pub fn tripled(n: f64) -> f64 {
        n * 3.0
    }
}

thread_local! {
    static RUN: fn(f64) -> f64 = if std::hint::black_box(true) { other::tripled } else { |n| n };
}

fn run_now(n: f64) -> f64 {
    RUN.with(|f| f(n))
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Root {
    pub n: f64,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Root {
    pub fn new(n: f64) -> Self {
        Root { n }
    }

    pub fn thrice(&self, n: f64) -> f64 {
        other::tripled(n)
    }

    pub fn run(&self, n: f64) -> f64 {
        run_now(n)
    }
}

pub fn ran(n: f64) -> f64 {
    let root = Root::new(n);
    root.thrice(n) + root.run(n)
}
`,
  });
  expect(p.stderr).not.toContain("error");
  expect(p.code).toBe(0);
  const js = read("lib.js");
  expect(js).toContain("Root.prototype.thrice = function (n) {");
  expect(js).toContain("Root.prototype.run = runNow;\n");
  expect(js).toContain("function runNow(n) {");
  const ran = run(["node", "--input-type=module", "--eval", `
    import { ran } from ${JSON.stringify(join(dir, "lib.js"))};
    console.log(ran(2));
  `]);
  expect(ran.trim()).toBe("12");
});
