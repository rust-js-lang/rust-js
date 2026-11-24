// A library's data in its consumer: a struct it builds the program's own,
// a JS-made struct's cell field replaced, quiet RefCell fields, a nullable
// parameter given null, and a closure capturing nothing made into a field.
// Split from cross-crate-items.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiled, js } from "./crates";
import { buildReact, compiler, fixture } from "./support";

beforeAll(buildReact, 600_000);

// A struct a library makes values of is the program's own data in its
// consumer too, as in the library: React DOM's getParent, of a fiber the
// reconciler makes, is `inst != null ? inst.return : null`, as upstream's,
// not a getter's `inst?.return` (rd221, ADR 1022, amended). A struct no
// code makes is JS's still, its field read once.
test("a struct a library builds is the program's own data in its consumer", async () => {
  const dir = fixture("cross-crate-built");
  const { app } = compiled(dir, `use std::cell::Cell;
use std::rc::Rc;

use js::Unknown;

pub struct Fiber {
    pub parent: Cell<Option<Rc<Fiber>>>,
    pub key: Option<&'static Unknown>,
}

pub struct Props {
    pub key: Option<&'static Unknown>,
}

pub fn fiber(parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber {
        parent: Cell::new(parent),
        key: None,
    })
}
`, `use std::rc::Rc;

use dep::Fiber;
use js::Unknown;

pub fn fiber_of(node: &Unknown) -> Option<Rc<Fiber>> {
    // Safety: a node's key holds its fiber.
    unsafe { js::cast(js::get(node, "fiber")) }
}

pub fn reparent(fiber: &Fiber, parent: Rc<Fiber>) {
    fiber.parent.replace(Some(parent));
}

pub fn fiber_keyed(fiber: &Fiber) -> bool {
    match fiber.key {
        Some(key) => js::truthy(Some(key)),
        None => false,
    }
}

pub fn keyed(given: &Unknown) -> bool {
    // Safety: what's given is props.
    let props: &dep::Props = unsafe { js::cast(Some(given)) };
    let key = props.key;
    key.is_some() && js::truthy(key)
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.built).toHaveLength(1);
  expect(app).toContain("export function reparent(fiber, parent) {\n  fiber.parent = parent;\n}");
  expect(app).toContain("  const key = props.key;\n");
  // The fiber's field, plain data, read where it's read: its truth, which
  // `None`'s `undefined` has none of.
  expect(app).toContain("  return !!fiber.key;\n");
  const { reparent, keyed, fiberKeyed } = await import(join(dir, "app", "lib.js"));
  const fiber = { parent: null, key: "k" };
  reparent(fiber, fiber);
  expect([fiber.parent === fiber, keyed({ key: 0 }), fiberKeyed(fiber)]).toEqual([true, false, true]);
});

// React's useActionState writes the form action a server action's
// `$$FORM_ACTION` gives, shared's `ReactCustomFormAction`, a struct JS
// makes: `metadata.action.replace(Some(..))` is upstream's `metadata.action
// = permalink`, the write alone, as `*borrow_mut() = ..` is, no getter's
// read of the old value nothing reads (rd418, ADR 1022, amended). One whose
// old value is read reads it once.
test("a replace of a JS-made struct's cell field whose old value nothing reads is the write alone", async () => {
  const dir = fixture("cross-crate-replace-unread");
  const { app } = compiled(dir, `use std::cell::RefCell;

pub struct FormAction {
    pub name: Option<String>,
    #[cfg_attr(rust_js, rust_js::optional)]
    pub action: RefCell<Option<String>>,
}

pub fn named(action: &FormAction) -> Option<String> {
    action.action.borrow().clone()
}
`, `use std::marker::PhantomData;

use dep::FormAction;
use js::JsObject;

pub struct ServerAction(PhantomData<JsObject>);

impl ServerAction {
    #[cfg_attr(rust_js, rust_js::link_name = "$$FORM_ACTION")]
    fn form_action(&self, prefix: &str) -> &'static FormAction {
        let _ = prefix;
        unreachable!()
    }
}

pub fn overridden(action: &ServerAction, permalink: Option<String>) -> &'static FormAction {
    let metadata = action.form_action("p");
    if let Some(permalink) = &permalink {
        metadata
            .action
            .replace(Some(js::concat_text(permalink.as_str())));
    }
    metadata
}

pub fn cleared(action: &ServerAction) {
    let metadata = action.form_action("p");
    metadata.action.take();
}

pub fn swapped(action: &ServerAction, permalink: &str) -> Option<String> {
    let metadata = action.form_action("p");
    metadata.action.replace(Some(permalink.to_owned()))
}
`);
  expect(app).toContain('  if (permalink != null) {\n    metadata.action = "" + permalink;\n  }\n');
  expect(app).toContain('  const metadata = action.$$FORM_ACTION("p");\n  metadata.action = undefined;\n}');
  expect(app).not.toContain("const old = metadata.action;\n    metadata.action =");
  expect(app).toContain("  const old = metadata.action;\n  metadata.action = permalink;\n  return old ?? undefined;\n");
  const { overridden, cleared, swapped } = await import(join(dir, "app", "lib.js"));
  // An action whose `action` a getter answers, which counts its reads.
  let reads = 0;
  let stored: string | undefined = "/old";
  const metadata = {
    name: "form",
    get action() {
      reads++;
      return stored;
    },
    set action(v: string | undefined) {
      stored = v;
    },
  };
  const server = { $$FORM_ACTION: () => metadata };
  expect(overridden(server, "/new")).toBe(metadata);
  expect([stored, reads]).toEqual(["/new", 0]);
  cleared(server);
  expect(stored).toBeUndefined();
  expect(reads).toBe(0);
  stored = "/kept";
  expect([swapped(server, "/next"), stored, reads]).toEqual(["/kept", "/next", 1]);
});

// A library of types alone, none of whose bodies asks what its `RefCell`s
// are, says each struct's field no borrow of which can meet another is
// quiet all the same (ADR 0863, amended), so a consumer's closure writes it
// in place, not through its one cell, `$fieldCell(metadata, "action")`
// (rd418).
test("a library none of whose bodies borrows a RefCell says its quiet fields", async () => {
  const dir = fixture("cross-crate-quiet-unasked");
  const { app } = compiled(dir, `use std::cell::RefCell;

pub struct FormAction {
    pub action: RefCell<Option<String>>,
}
`, `use dep::FormAction;

fn each(f: &dyn Fn(&str), given: &str) {
    f(given)
}

pub fn renamed(make: &dyn Fn() -> &'static FormAction, permalink: Option<String>) {
    each(
        &move |_| {
            let metadata = make();
            if let Some(permalink) = &permalink {
                *metadata.action.borrow_mut() = Some(permalink.clone());
            }
        },
        "p",
    );
}
`);
  const facts = JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.fields;
  expect(facts).toEqual([expect.objectContaining({ facts: ["quiet"] })]);
  expect(app).not.toContain("$fieldCell");
  expect(app).toContain("      metadata.action = permalink;\n");
  const { renamed } = await import(join(dir, "app", "lib.js"));
  const metadata: { action?: string } = {};
  renamed(() => metadata, "/new");
  expect(metadata.action).toBe("/new");
});

// rd613 (ADR 0861, amended): a library's function whose own nullable
// parameter each of its crate's calls gives `null` or a value tests it
// `!== null`, as its contract says (`nulled_params`); a consumer's call
// gives what may be `undefined` `?? null`, and what's `null` or a value
// already as it is. Its use as a value is refused.
test("a library's nullable parameter its calls give null is given null by its consumer", async () => {
  const dir = fixture("cross-crate-nulled-params");
  const { dep, app } = compiled(dir, `pub struct ModuleLoading {
    pub prefix: String,
}

#[cfg_attr(rust_js, rust_js::nullable(module_loading))]
pub fn prepare(module_loading: Option<&ModuleLoading>, chunk: &str) -> String {
    if let Some(module_loading) = module_loading {
        return module_loading.prefix.clone() + chunk;
    }
    String::new()
}
`, `use dep::ModuleLoading;

pub fn given(module_loading: Option<&ModuleLoading>) -> String {
    dep::prepare(module_loading, "a.js")
}

#[cfg_attr(rust_js, rust_js::nullable(module_loading))]
pub fn passed_on(module_loading: Option<&ModuleLoading>) -> String {
    dep::prepare(module_loading, "b.js") + &dep::prepare(None, "c.js")
}

pub struct Holder<'a> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub loading: Option<&'a ModuleLoading>,
}

pub fn held(holder: &Holder) -> String {
    dep::prepare(holder.loading, "e.js")
}

#[cfg_attr(rust_js, rust_js::nullable(module_loading))]
pub fn relayed(module_loading: Option<&ModuleLoading>) -> String {
    dep::prepare(module_loading, "f.js")
}

pub fn relays(module_loading: Option<&ModuleLoading>) -> String {
    relayed(module_loading)
}
`);
  expect(JSON.parse(readFileSync(join(dir, "dep", "lib.manifest.json"), "utf8")).library.items[0].nulled_params).toEqual([0]);
  expect(dep).toContain("  if (moduleLoading !== null) {\n");
  expect(app).toContain('  return Dep.prepare(moduleLoading ?? null, "a.js");\n');
  expect(app).toContain('  return Dep.prepare(moduleLoading, "b.js") + Dep.prepare(null, "c.js");\n');
  // A nullable field's, `null` or a value already, as it is.
  expect(app).toContain('  return Dep.prepare(holder.loading, "e.js");\n');
  // An own nullable parameter a call of the crate's gives what may be
  // `undefined`, `relays`'s, isn't `null` or a value: given `?? null`.
  expect(app).toContain('  return Dep.prepare(moduleLoading ?? null, "f.js");\n');
  const { given, passedOn, relays } = await import(join(dir, "app", "lib.js"));
  expect([given(undefined), given(null), given({ prefix: "p/" })]).toEqual(["", "", "p/a.js"]);
  expect([passedOn(null), passedOn({ prefix: "p/" })]).toEqual(["", "p/b.js"]);
  expect([relays(undefined), relays(null), relays({ prefix: "p/" })]).toEqual(["", "", "p/f.js"]);
  writeFileSync(join(dir, "valued.rs"), `use dep::ModuleLoading;

pub fn each(loadings: &[Option<&ModuleLoading>]) -> Vec<String> {
    loadings.iter().copied().map(|loading| (loading, "d.js")).map(|(l, c)| dep::prepare(l, c)).collect()
}

pub fn as_value() -> fn(Option<&ModuleLoading>, &str) -> String {
    dep::prepare
}
`);
  const p = Bun.spawnSync([compiler, join(dir, "valued.rs"), "-o", join(dir, "valued", "lib.js"), "--dependency",
    join(dir, "dep", "lib.manifest.json"), "--", "--crate-name", "valued", "--extern", `dep=${join(dir, "libdep.rmeta")}`,
    "-L", dir, ...js], { stderr: "pipe" });
  const stderr = p.stderr.toString();
  expect(p.exitCode).not.toBe(0);
  expect(stderr).toContain("whose nullable parameter is `null` at each of its calls, given as a value");
  expect(stderr.match(/given as a value/g)).toHaveLength(1);
});

// A closure capturing nothing that a library's generic function makes into
// a struct's field is a constant there, `f: const ZeroSized`, in the MIR a
// consumer compiles its own of the function from, cloning what isn't its
// own clone (ADR 0914): its body is found as the struct's operand. arrayvec's `ScopeExitGuard` in `extend_from_iter`
// refused its probe with "this closure, from its MIR" (#795).
test("a library's closure capturing nothing, made into a struct's field, is compiled by its consumer", async () => {
  const dir = fixture("cross-crate-zero-sized-closure");
  const { app } = compiled(dir, `struct Guard<T, Data, F: FnMut(&Data, &mut T)> {
    value: T,
    data: Data,
    f: F,
}

impl<T, Data, F: FnMut(&Data, &mut T)> Drop for Guard<T, Data, F> {
    fn drop(&mut self) {
        (self.f)(&self.data, &mut self.value)
    }
}

pub struct Tally {
    pub len: u32,
}

impl Tally {
    pub fn cloned_all<T: Clone>(&mut self, items: &[T]) -> Vec<T> {
        let len = self.len;
        let mut guard = Guard {
            value: &mut self.len,
            data: len,
            f: move |&len, self_len: &mut &mut u32| **self_len = len,
        };
        let mut cloned = Vec::new();
        for item in items {
            cloned.push(item.clone());
            guard.data += 1;
        }
        cloned
    }
}
`, `pub fn tally() -> (u32, Vec<Vec<u32>>, Vec<Vec<u32>>) {
    let mut tally = dep::Tally { len: 1 };
    let original = vec![vec![4], vec![5]];
    let mut copies = tally.cloned_all(&original);
    copies[0].push(6);
    (tally.len, original, copies)
}
`);
  // The consumer's own, as what it clones isn't itself.
  expect(app).toContain("  clonedAll(tally, items, TClone) {\n");
  const { tally } = await import(join(dir, "app", "lib.js"));
  expect(tally()).toEqual([3, [[4], [5]], [[4, 6], [5]]]);
});
