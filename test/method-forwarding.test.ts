// What the compiler writes of a method that only passes its parameters on
// to a function of the crate's (ADR 0605's forwarding family, ADR 0262's
// forwarding exception): a namespace's, of its own module's function, is
// that function (#566), and one its module names apart from that name is
// refused, as a call through the namespace would be the function's
// (#585); a constructor function's prototype's is the function itself,
// `X.prototype.m = f;` (rd715). Each test is a program of its own,
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

// The js crate, which the library tests compile against.
beforeAll(buildReact, 600_000);

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

// rd758 (#612): a namespace's method that's a function, its own module's
// (#566) or another's re-exported (rd292b), has no JS of its own, so its
// comments, above it, its doc, in its body and after code on its line,
// are before what it is: the function's declaration, after the function's
// own, or the re-export, where its `use`'s are, its first method's as
// they were, a blank line between kept, then the rest's, in the order
// written. One of no comments, none (ADR 1037, amended).
test("a namespace's forwarding method keeps its comments before the function it is", async () => {
  const { p, dir, read } = compiled("namespace-forwarding-comments", {
    "lib.rs": `mod other;

use other::batched;

/// Closes the response.
pub fn close(n: f64) -> f64 {
    n + 1.0
}

pub fn open(n: f64) -> f64 {
    n
}

#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Client;

#[cfg_attr(rust_js, rust_js::namespace)]
impl Client {
    /// Closes it.
    pub fn close(&self, n: f64) -> f64 {
        // Passed on.
        close(n) // As it is.
    }

    pub fn open(&self, n: f64) -> f64 {
        open(n)
    }

    // The batch.

    // Above batched.
    pub fn batched(&self, n: f64) -> f64 {
        // In batched.
        batched(n)
    }

    // Above flushed.
    pub fn flushed(&self, n: f64) -> f64 {
        // In flushed.
        other::flushed(n)
    }
}

pub fn inside(n: f64) -> f64 {
    Client.close(n) + Client.open(n) + Client.batched(n) + Client.flushed(n)
}
`,
    "other.rs": "pub fn batched(n: f64) -> f64 {\n    n * 2.0\n}\n\npub fn flushed(n: f64) -> f64 {\n    n * 3.0\n}\n",
  });
  expect(p.stderr).not.toContain("error");
  expect(p.code).toBe(0);
  const js = read("lib.js");
  expect(js).toContain(
    "\n/** Closes the response. */\n/** Closes it. */\n// Passed on.\n// As it is.\nexport function close(n) {\n  return n + 1;\n}\n\nexport function open(n) {\n",
  );
  expect(js).toContain(
    "\n// The batch.\n\n// Above batched.\n// In batched.\n// Above flushed.\n// In flushed.\nimport { batched, flushed } from \"./other.js\";\nexport { batched, flushed };\n",
  );
  const ran = run(["node", "--input-type=module", "--eval", `
    import * as lib from ${JSON.stringify(join(dir, "lib.js"))};
    console.log(JSON.stringify([lib.inside(1), lib.Client === undefined]));
  `]);
  expect(ran.trim()).toBe("[8,true]");
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

// #596: ReactFlightClient's `create_response` is
// `#[rust_js::nullable(bundler_config, ..)]`, and ReactNoopFlightClient
// calls it through the namespace's value. Its forwarding method is that
// function, so it says the same `#[rust_js::nullable(..)]`, and a call
// through the value gives `null` where the function's `None` is `null`:
// it panicked the compiler, and without the attribute gave `undefined`.
// One that says otherwise, or nothing, is refused, naming both lists.
test("a namespace's forwarding method says its function's nullable parameters", async () => {
  const source = (said: string, functionSaid = "#[cfg_attr(rust_js, rust_js::nullable(config))]\n") => `pub struct Response {
    pub n: f64,
    pub configured: bool,
}

${functionSaid}pub fn create_response(config: Option<&'static str>, n: f64) -> Response {
    Response {
        n,
        configured: config.is_some(),
    }
}

#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Client;

#[cfg_attr(rust_js, rust_js::namespace)]
impl Client {
${said}    pub fn create_response(&self, config: Option<&'static str>, n: f64) -> Response {
        create_response(config, n)
    }
}

pub fn through(client: &Client) -> f64 {
    client.create_response(None, 1.0).n + client.create_response(Some("c"), 2.0).n
}
`;
  const { p, dir, read } = compiled("namespace-forwarding-nullable", {
    "lib.rs": "pub mod client;\n",
    "client.rs": source("    #[cfg_attr(rust_js, rust_js::nullable(config))]\n"),
  });
  expect(p.stderr).not.toContain("error");
  expect(p.code).toBe(0);
  const js = read("client.js");
  expect(js).toContain("client.createResponse(null, 1).n");
  expect(js).not.toContain("createResponse$1");
  const ran = run(["node", "--input-type=module", "--eval", `
    import * as client from ${JSON.stringify(join(dir, "client.js"))};
    const given = [];
    const spy = { createResponse: (config, n) => (given.push(config), client.createResponse(config, n)) };
    console.log(JSON.stringify([client.through(spy), given]));
  `]);
  expect(ran.trim()).toBe('[3,[null,"c"]]');
  const nullable = "    #[cfg_attr(rust_js, rust_js::nullable(config))]\n";
  const cases: [string, string, string | undefined, string, string][] = [
    ["unsaid", "", undefined, "`config`", "none"],
    ["other", nullable, "", "none", "`config`"],
  ];
  for (const [name, said, functionSaid, theirs, method] of cases) {
    const refused = compiled(`namespace-forwarding-nullable-${name}`, { "lib.rs": "pub mod client;\n", "client.rs": source(said, functionSaid) }).p;
    expect(refused.code).not.toBe(0);
    expect(refused.stderr).toContain(
      `rust-js: \`client::Client::create_response\` is \`client::create_response\`, which it passes its parameters on to, so it says the same of them \`#[rust_js::nullable(..)]\`, as a call of it is that function's (the function's: ${theirs}; the method's: ${method})`,
    );
  }
});

// #596 (rd728): another crate's call of a library's forwarding method
// through the namespace's value, ReactNoopFlightClient's
// `client.create_response(..)` of ReactFlightClient's, compiles where the
// method says its function's `#[rust_js::nullable(..)]`, and gives `null`.
test("another crate calls a library's forwarding method through the namespace's value", async () => {
  const dir = fixture("namespace-forwarding-cross-crate");
  writeFileSync(join(dir, "dep.rs"), `pub struct Response {
    pub n: f64,
    pub configured: bool,
}

#[cfg_attr(rust_js, rust_js::nullable(config))]
pub fn create_response(config: Option<&'static str>, n: f64) -> Response {
    Response {
        n,
        configured: config.is_some(),
    }
}

#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Client;

#[cfg_attr(rust_js, rust_js::namespace)]
impl Client {
    #[cfg_attr(rust_js, rust_js::nullable(config))]
    pub fn create_response(&self, config: Option<&'static str>, n: f64) -> Response {
        create_response(config, n)
    }
}
`);
  const library = runSync([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", "--crate-type", "lib", `--emit=metadata=${join(dir, "libdep.rmeta")}`], root, 60_000);
  expect(library.stderr).not.toContain("error");
  writeFileSync(join(dir, "app.rs"), `pub fn through(client: &dep::Client) -> f64 {
    client.create_response(None, 1.0).n + client.create_response(Some("c"), 2.0).n
}
`);
  const app = runSync([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type", "lib", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir], root, 60_000);
  expect(app.stderr).not.toContain("error");
  expect(app.code).toBe(0);
  expect(readFileSync(join(dir, "app", "lib.js"), "utf8")).toContain("client.createResponse(null, 1).n");
  const ran = run(["node", "--input-type=module", "--eval", `
    import * as client from ${JSON.stringify(join(dir, "dep", "lib.js"))};
    import { through } from ${JSON.stringify(join(dir, "app", "lib.js"))};
    const given = [];
    const spy = { createResponse: (config, n) => (given.push(config), client.createResponse(config, n)) };
    console.log(JSON.stringify([through(spy), given]));
  `]);
  expect(ran.trim()).toBe('[3,[null,"c"]]');
});

/** `dep`, a library, and `app`, a crate that uses it, compiled in `dir`:
 * each one's JS. */
function withLibrary(dir: string, dep: string, app: string): { dep: string; app: string } {
  const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  return { dep: readFileSync(join(dir, "dep", "lib.js"), "utf8"), app: readFileSync(join(dir, "app", "lib.js"), "utf8") };
}

// #601 (ADR 0861, amended): a namespace's method that's the function it
// passes its parameters on to is that function, so whether the library has
// one changes neither the function's JS nor its contract, which the
// method's says too, each past its `self`. React's unbundled Flight client
// gives `serverConsumerManifest.serverModuleMap`, a nullable field of
// upstream's `null | ServerManifest`, `type ServerManifest = void` (rd645),
// so `undefined` where it's a `Some`, on to react-client's createResponse,
// which its contract says is `null` or a value: `?? null`, built with the
// method or without it.
test("a library's function and the namespace's method that's it say one contract", async () => {
  const dir = fixture("cross-crate-forwarded-nulled");
  const dep = `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable(bundler_config, server_reference_config))]
pub fn create_response(
    bundler_config: Option<&'static Unknown>,
    server_reference_config: Option<&'static Unknown>,
) -> bool {
    bundler_config.is_some() && server_reference_config.is_none()
}
`;
  const method = `
#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Client;

unsafe impl js::Defined for Client {}

#[cfg_attr(rust_js, rust_js::namespace)]
impl Client {
    #[cfg_attr(rust_js, rust_js::nullable(bundler_config, server_reference_config))]
    pub fn create_response(
        &self,
        bundler_config: Option<&'static Unknown>,
        server_reference_config: Option<&'static Unknown>,
    ) -> bool {
        create_response(bundler_config, server_reference_config)
    }
}
`;
  const app = `use js::Unknown;

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ServerManifest {
    #[cfg_attr(rust_js, rust_js::undefined)]
    Undefined,
}

pub struct ServerConsumerManifest {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub module_loading: Option<&'static Unknown>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub server_module_map: Option<&'static ServerManifest>,
}

pub fn create(manifest: &'static ServerConsumerManifest) -> bool {
    dep::create_response(manifest.module_loading, js::any_of(manifest.server_module_map))
}
`;
  const nulled = () => {
    const items = JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.items;
    return Object.fromEntries(items.map((item: { rust_path: string; nulled_params?: number[] }) => [item.rust_path, item.nulled_params]));
  };
  const alone = withLibrary(dir, dep, app);
  expect(nulled()).toEqual({ "dep::create_response": [0, 1] });
  const forwarded = withLibrary(dir, dep + method, app);
  expect(nulled()).toEqual({ "dep::create_response": [0, 1], "dep::Client::create_response": [1, 2] });
  expect(forwarded.app).toBe(alone.app);
  expect(alone.dep).toContain("  return bundlerConfig !== null && serverReferenceConfig === null;\n");
  expect(forwarded.dep).toContain("  return bundlerConfig !== null && serverReferenceConfig === null;\n");
  expect(alone.app).toContain("  return Dep.createResponse(manifest.moduleLoading, manifest.serverModuleMap ?? null);\n");
  // A call of the library's own that may give `undefined`: neither says
  // that parameter, whichever is called.
  withLibrary(dir, dep + method + "\npub fn loose(config: Option<&'static Unknown>) -> bool {\n    create_response(config, None)\n}\n", app);
  expect(nulled()).toEqual({ "dep::create_response": [1], "dep::Client::create_response": [2], "dep::loose": undefined });
  const { create } = await import(join(dir, "app", "lib.js"));
  expect([create({ moduleLoading: {}, serverModuleMap: undefined }), create({ moduleLoading: {}, serverModuleMap: null })]).toEqual([true, true]);
});
