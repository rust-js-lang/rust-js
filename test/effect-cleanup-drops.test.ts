// An effect's cleanup that holds a value with a destructor (#1881, ADR
// 1020 amended): what the effect gives back React calls once, as its
// `Cleanup` says, its `FnOnce` member, so what the cleanup took is dropped
// as that call ends, as Rust's `call_once` drops it. An RAII guard the
// cleanup drops, `move || drop(listening)`, as the deep-chain harness's
// primitives stops listening to the window; one that takes a guard's text
// alone, whose guard the effect drops; and one rustc infers `Fn` that
// holds its guard whole. Each log is native Rust's, of the same closures
// given to an `effect` that calls the cleanup once after `mounted`.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => [join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];

const program = `use react::{JSX, jsx, use_effect};

/// Says when it's dropped.
pub struct Guard(pub &'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

/// Its cleanup drops the guard it holds.
pub fn Dropping() -> JSX::Element {
    use_effect(
        || {
            println!("listening");
            let guard = Guard("dropping");
            move || {
                println!("cleanup dropping");
                drop(guard);
                println!("after dropping");
            }
        },
        Some(&()),
    );
    jsx! { <p>{"dropping"}</p> }
}

/// Its cleanup drops one guard and reads the other's text, which is all it
/// takes of it: that guard is the effect's, dropped as the effect returns,
/// as Rust's closure takes the place it reads.
pub fn Both() -> JSX::Element {
    use_effect(
        || {
            let first = Guard("first");
            let second = Guard("second");
            move || {
                drop(first);
                println!("cleanup reads {}", second.0);
            }
        },
        Some(&()),
    );
    jsx! { <p>{"both"}</p> }
}

/// Its cleanup, which rustc infers \`Fn\`, holds its guard whole, dropped
/// as React's one call of it ends, as its \`call_once\` drops it.
pub fn Holding() -> JSX::Element {
    use_effect(
        || {
            let guard = Guard("holding");
            move || {
                let held = &guard;
                println!("cleanup {}", held.0);
            }
        },
        Some(&()),
    );
    jsx! { <p>{"holding"}</p> }
}
`;

test("an effect's cleanup drops what it holds as React calls it", () => {
  const dir = fixture("effect-cleanup-drops");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, ...flags(dir)]);
  writeFileSync(join(dir, "driver.test.jsx"), `import { expect, test } from "bun:test";
import { createElement } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { Both, Dropping, Holding } from "./lib.jsx";

const said = [];
console.log = (line) => said.push(line);

test("mounted then unmounted, each cleanup once, each guard dropped as it ends", () => {
  const container = document.createElement("div");
  const root = createRoot(container);
  flushSync(() => root.render(createElement("div", null, createElement(Dropping), createElement(Both), createElement(Holding))));
  expect(said).toEqual(["listening", "dropped second"]);
  flushSync(() => root.unmount());
  expect(said).toEqual(["listening", "dropped second", "cleanup dropping", "dropped dropping", "after dropping", "dropped first", "cleanup reads second", "cleanup holding", "dropped holding"]);
});
`);
  const ran = Bun.spawnSync(["bun", "test", "--preload", "./test/happydom.js", join(dir, "driver.test.jsx")], { cwd: root, stderr: "pipe" });
  const output = ran.stdout.toString() + ran.stderr.toString();
  expect([ran.exitCode, output.match(/(\d+) pass/)?.[1], output.includes("(fail)") ? output : ""]).toEqual([0, "1", ""]);
  expect(readFileSync(join(dir, "lib.jsx"), "utf8")).toContain("return () => {");
});

// What a binding gives back that it doesn't call once, `use_memo`'s value,
// isn't: a closure it returns that holds a guard stays refused, as JS would
// keep it and never drop what it took.
test("a closure a memo gives back that holds a value with a destructor is refused", () => {
  const dir = fixture("effect-cleanup-memo");
  writeFileSync(join(dir, "lib.rs"), `use react::{JSX, jsx, use_memo};

pub struct Guard(pub &'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn Memo() -> JSX::Element {
    let kept = use_memo(
        || {
            let guard = Guard("memo");
            move || drop(guard)
        },
        (),
    );
    let _ = kept;
    jsx! { <p>{"memo"}</p> }
}
`);
  const refused = Bun.spawnSync([compiler, ...flags(dir)], { cwd: root, stderr: "pipe", timeout: 60_000 });
  expect([refused.exitCode === 0, refused.stderr.toString().includes("a closure that holds a value with a destructor")]).toEqual([false, true]);
});
