// A switch's arms and what each returns: arms that end alike each their
// case's, an arm that goes on to its value returning it, a case's return
// its end, a function's last arms' returns, and an untagged enum's arms
// alike in JS one call. Split from lowering-switches.test.ts by concept
// (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = () => ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// ADR 0956: React's commitAttachRef, a `switch` of a case per tag, two
// arms of which end in the same value: each case's statements as written,
// not their tail shared after a smaller `switch` of an `if` chain's `else`.
test("arms of a switch that end alike are each their case's", async () => {
  const dir = fixture("switch-arms-sharing-a-tail");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

pub struct Fiber {
    pub tag: Cell<u8>,
    pub state_node: Cell<Option<&'static js::Unknown>>,
}

pub const HOST_COMPONENT: u8 = 5;
pub const FRAGMENT: u8 = 7;
pub const VIEW_TRANSITION_COMPONENT: u8 = 30;
pub const ENABLE_VIEW_TRANSITION: bool = true;

#[cfg_attr(rust_js, rust_js::link_name = "rustJsPublic", rust_js::nullable)]
fn get_public_instance(instance: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    let _ = instance;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "rustJsFragment")]
fn create_fragment_instance(fiber: &Fiber) -> &'static js::Unknown {
    let _ = fiber;
    unreachable!()
}

pub fn attach(finished_work: &Fiber) -> Option<&'static js::Unknown> {
    let instance_to_use = match finished_work.tag.get() {
        HOST_COMPONENT => get_public_instance(finished_work.state_node.get()),
        VIEW_TRANSITION_COMPONENT => {
            if ENABLE_VIEW_TRANSITION {
                get_public_instance(None)
            } else {
                finished_work.state_node.get()
            }
        }
        FRAGMENT => {
            if finished_work.state_node.get().is_none() {
                finished_work
                    .state_node
                    .set(Some(create_fragment_instance(finished_work)));
            }
            finished_work.state_node.get()
        }
        _ => finished_work.state_node.get(),
    };
    instance_to_use
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  switch (finishedWork.tag) {\n    case HOST_COMPONENT:\n");
  expect(js).toContain(
    "    case FRAGMENT:\n      finishedWork.stateNode ??= rustJsFragment(finishedWork);\n      instanceToUse = finishedWork.stateNode;\n      break;\n    default:\n      instanceToUse = finishedWork.stateNode;\n      break;\n  }\n",
  );
  const global = globalThis as Record<string, unknown>;
  global.rustJsPublic = (x: unknown) => ["public", x ?? null];
  global.rustJsFragment = () => "fragment";
  const lib = await import(join(dir, "lib.js"));
  const node = {};
  expect([
    lib.attach({ tag: 5, stateNode: node }),
    lib.attach({ tag: 30, stateNode: node }),
    lib.attach({ tag: 7, stateNode: null }),
    lib.attach({ tag: 7, stateNode: node }),
    lib.attach({ tag: 1, stateNode: node }),
  ]).toEqual([["public", node], ["public", null], "fragment", node, node]);
});

// ADR 0956, amended: React's unwindWork, `case SuspenseListComponent: {
// ..; if (..) { .. } return null; } default: return null;`, an arm whose
// statements go on to the value it ends with: that value its case's, as
// the program wrote it, not shared with `_`'s after the `switch`.
test("an arm of a switch that goes on to its value returns it, and `_`'s is its default", async () => {
  const dir = fixture("switch-arm-going-on-to-its-value");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn unwind(tag: u8, value: u32, log: &mut Vec<u32>) -> Option<u32> {
    match tag {
        5 => {
            log.push(value);
            None
        }
        25 => {
            if value > 2 {
                log.push(value);
            }
            None
        }
        _ => None,
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "    case 25:\n      if (value > 2) {\n        log.push(value);\n      }\n      return null;\n    default:\n      return null;\n  }\n}\n",
  );
  const lib = await import(join(dir, "lib.js"));
  const log: number[] = [];
  expect([5, 25, 25, 1].map((tag, i) => lib.unwind(tag, i + 1, log))).toEqual([null, null, null, null]);
  expect(log).toEqual([1, 3]);
});

// ADR 0905, amended: React's popToNextHostParent, `while
// (hydrationParentFiber) { switch (hydrationParentFiber.tag) { case
// HostComponent: ..; return; case HostRoot: ..; return; default: .. } }`.
// MIR joins the arms' `return`s into one after the `switch`, which printed
// each case's `break;` and a `return;` after it: each case of an arm the
// program wrote a `return` in ends with it, as written, and the `default`
// that goes on, with no `continue`.
test("a case of an arm that returns ends with its return", async () => {
  const dir = fixture("switch-arms-returning");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    pub parent: Option<Rc<Fiber>>,
}

pub fn pop_to_host(fiber: &Fiber, log: &mut Vec<u8>) {
    let mut parent = fiber.parent.clone();
    while let Some(current) = parent {
        match current.tag {
            5 | 13 => {
                log.push(1);
                return;
            }
            3 | 27 => {
                log.push(2);
                return;
            }
            _ => {
                parent = current.parent.clone();
            }
        }
    }
}

pub fn fiber(tag: u8, parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { tag, parent })
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "      case 27:\n        log.push(2);\n        return;\n      default:\n        parent = parent.parent;\n        break;\n    }\n  }\n}\n",
  );
  const lib = await import(join(dir, "lib.js"));
  const log: number[] = [];
  const root = lib.fiber(3, null);
  const host = lib.fiber(13, lib.fiber(1, root));
  for (const parent of [null, root, host, lib.fiber(1, lib.fiber(2, null))]) {
    lib.popToHost(lib.fiber(0, parent), log);
  }
  expect(log).toEqual([2, 1]);
});

// ADR 0742, amended (#435): at a function's end, MIR joins the arms'
// written `return`s into the function's own end, which the `_` arm goes
// on to too, and puts that arm before the one it tests apart, `if (tag
// !== 3 && tag !== 27) { log.push(3); return; } log.push(2); break bb;`.
// Each `break` is the `return` written after what runs just before it,
// `log.push(2)`, not after the `_` arm: the second arm's case ends with its
// `return;` as written, as the first's does, not a `break;` of the
// `switch`.
test("each arm's written return at a function's end is its case's return", async () => {
  const dir = fixture("switch-arms-returning-at-end");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    pub parent: Option<Rc<Fiber>>,
}

pub fn pop(fiber: &Fiber, log: &mut Vec<u8>) {
    if let Some(parent) = fiber.parent.clone() {
        match parent.tag {
            5 | 13 => {
                log.push(1);
                return;
            }
            3 | 27 => {
                log.push(2);
                return;
            }
            _ => {
                log.push(3);
            }
        }
    }
}

pub fn skip(fiber: &Fiber, log: &mut Vec<u8>) {
    if let Some(parent) = fiber.parent.clone() {
        match parent.tag {
            5 | 13 => {
                log.push(1);
                return;
            }
            3 | 27 => return,
            _ => {
                log.push(3);
            }
        }
    }
}

pub fn fiber(tag: u8, parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { tag, parent })
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "      case 5:\n      case 13:\n        log.push(1);\n        return;\n      case 3:\n      case 27:\n        log.push(2);\n        return;\n      default:\n        log.push(3);\n        break;\n    }\n  }\n}\n",
  );
  // An arm that only returns, its `break` straight after the `_` arm's
  // `if`: the `return` after that `if`'s test.
  expect(js).toContain("      case 3:\n      case 27:\n        return;\n      default:\n        log.push(3);\n");
  const lib = await import(join(dir, "lib.js"));
  const log: number[] = [];
  for (const tag of [5, 13, 3, 27, 0]) {
    lib.pop(lib.fiber(0, lib.fiber(tag, null)), log);
    lib.skip(lib.fiber(0, lib.fiber(tag, null)), log);
  }
  lib.pop(lib.fiber(0, null), log);
  lib.skip(lib.fiber(0, null), log);
  expect(log).toEqual([1, 1, 1, 1, 2, 2, 3, 3]);
});

// React port gap rd238: FragmentInstance's observers are `IntersectionObserver
// | ResizeObserver`, an untagged enum, each arm calling its member's method.
// Its arms alike in JS, the `match` is the one call upstream writes, and its
// test, which reads a global a host may lack, goes; a test that does
// something stays.
test("a match of an untagged enum whose arms are alike in JS is the one call", async () => {
  const withWeb = ["--", "--edition", "2024", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("webapi-alike-arms");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Element, IntersectionObserver, ResizeObserver};

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy)]
pub enum Observer {
    Intersection(&'static IntersectionObserver),
    Resize(&'static ResizeObserver),
}

pub fn observe_all(observers: &[Observer], child: &Element) {
    for observer in observers {
        match observer {
            Observer::Intersection(observer) => observer.observe(child),
            Observer::Resize(observer) => observer.observe(child, None),
        }
    }
}

pub fn disconnected(observer: Observer, ready: fn() -> bool) {
    #[allow(clippy::if_same_then_else)]
    if ready() { observer_disconnect(observer) } else { observer_disconnect(observer) }
}

#[allow(clippy::if_same_then_else)]
pub fn either(observer: Observer, wide: bool) {
    if wide { observer_disconnect(observer) } else { observer_disconnect(observer) }
}

pub fn partly(observers: &[Observer], wide: bool, ready: fn() -> bool) {
    for &observer in observers {
        if wide {
            observer_disconnect(observer);
        } else {
            observer_disconnect(observer);
            ready();
        }
        ready();
    }
}

fn observer_disconnect(observer: Observer) {
    match observer {
        Observer::Intersection(observer) => observer.disconnect(),
        Observer::Resize(observer) => observer.disconnect(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (let i = 0; i < observers.length; i++) {\n    const observer = observers[i];\n    observer.observe(child);\n  }\n");
  expect(js).toContain("function observerDisconnect(observer) {\n  observer.disconnect();\n}");
  expect(js).toContain("if (ready()) {");
  expect(js).not.toContain("instanceof");
  // An \`if\` whose branches are alike is its branch; alike in part, kept.
  expect(js).toContain("export function either(observer, wide) {\n  observerDisconnect(observer);\n}");
  expect(js).toContain("    if (wide) {\n      observerDisconnect(observer);\n    } else {\n      observerDisconnect(observer);\n      ready();\n    }\n");
  const lib = await import(join(dir, "lib.js"));
  const seen: string[] = [];
  const observer = (name: string) => ({ observe: (child: string) => void seen.push(`${name} ${child}`), disconnect: () => void seen.push(`${name} off`) });
  lib.observeAll([observer("a"), observer("b")], "c");
  let asked = 0;
  lib.disconnected(observer("d"), () => ++asked > 0);
  expect([seen, asked]).toEqual([["a c", "b c", "d off"], 1]);
  // Arms alike only in part are told apart, by `instanceof` of a class,
  // `false` of another realm's: refused (#555).
  writeFileSync(join(dir, "lib.rs"), `${readFileSync(join(dir, "lib.rs"), "utf8")}${`
pub fn watched(observers: &[Observer], child: &Element) {
    for observer in observers {
        match observer {
            Observer::Intersection(observer) => observer.observe(child),
            Observer::Resize(observer) => {
                observer.observe(child, None);
                observer.disconnect();
            }
        }
        observe_all(observers, child);
    }
}
`}`);
  const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "refused.js"), ...withWeb]);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("JS can't tell `Observer::Intersection`, a `IntersectionObserver`, from `Observer`'s other variants in every realm");
});
