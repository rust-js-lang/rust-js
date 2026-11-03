// What a crate's declarations (ADR 0196) say of a field whose cell is the
// property it holds (ADR 0772): what the cell holds, an `Option` of it
// declared as the `Option` would be, one a caller may leave out, named as
// the field's type writes it (rd414, rd415), and `| null` where a
// `js::null()` is written to it (ADR 0935, amended).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

test("a cell's Option field is declared as its Option, as written", () => {
  const dir = fixture("declarations-cells");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};

pub type Trace = Vec<(String, f64)>;

pub struct Info {
    pub size: Option<f64>,
    #[cfg_attr(rust_js, rust_js::optional)]
    pub byte_size: Cell<Option<f64>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub plain_stack: Option<Trace>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub stack: RefCell<Option<Trace>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub owner_stack: Cell<Option<Trace>>,
    pub count: Cell<u32>,
    pub made: Cell<Option<f64>>,
    pub set: Cell<Option<f64>>,
    pub replaced: Cell<Option<f64>>,
    pub queue: RefCell<Option<f64>>,
}

pub fn info() -> Info {
    Info {
        size: None,
        byte_size: Cell::new(None),
        plain_stack: None,
        stack: RefCell::new(None),
        owner_stack: Cell::new(None),
        count: Cell::new(0),
        made: Cell::new(js::null()),
        set: Cell::new(None),
        replaced: Cell::new(Some(1.0)),
        queue: RefCell::new(None),
    }
}

pub fn reset(info: &Info) -> Option<f64> {
    info.set.set(js::null());
    *info.queue.borrow_mut() = js::null();
    info.replaced.replace(js::null())
}

pub fn sized(info: &Info) -> f64 {
    info.byte_size.get().unwrap_or(0.0) + info.count.get() as f64
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain(`export interface Info {
    size?: number | undefined;
    byteSize?: number | undefined;
    plainStack?: Trace | null | undefined;
    stack?: Trace | null | undefined;
    ownerStack?: Trace | null | undefined;
    count: number;
    made?: number | null | undefined;
    set?: number | null | undefined;
    replaced?: number | null | undefined;
    queue?: number | null | undefined;
}`);
});
