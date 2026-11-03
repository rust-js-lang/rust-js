// How an object is laid out: across lines where its struct literal is
// written so, of two fields or more, or one its literal writes a comment
// in; on one line otherwise, where it fits (ADR 0835, amended). Each test
// is a program of its own, compiled.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// ADR 0835: an object of a struct literal written across lines, a line
// ending after its `{`, as rustfmt writes one too long for a line, is on
// lines of its own, as Prettier keeps an object a person wrote so; one
// written on one line is on one where it fits, and so is an object no
// literal writes, a range's or a tuple variant's. A `const`'s or a
// `static`'s, rustc's value, is as its literal is written.
test("an object is across lines where its struct literal is written so", () => {
  const dir = fixture("objects-as-written");
  writeFileSync(join(dir, "lib.rs"), `pub struct Ref {
    pub kind: &'static str,
    pub render: u32,
}

pub enum Event {
    Moved { dx: i32, dy: i32 },
    Placed(u32),
}

pub fn element(render: u32) -> Ref {
    let element_type = Ref {
        kind: "forward_ref",
        render,
    };
    element_type
}

pub fn short(render: u32) -> Ref {
    Ref { kind: "short", render }
}

pub struct Described {
    pub description: &'static str,
}

pub fn described() -> Described {
    Described {
        description: "one field, too long for rustfmt's line",
    }
}

pub fn commented() -> Described {
    Described {
        // Why it says so.
        description: "one field, a comment in it",
    }
}

pub fn block_commented() -> Described {
    Described {
        description: /* why */ "one field",
    }
}

pub struct Strategy {
    pub high_water_mark: Option<f64>,
    pub size: Option<f64>,
}

pub fn strategy() -> Strategy {
    Strategy {
        high_water_mark: Some(2048.0),
        size: None,
    }
}

pub const ONE_FIELD: Described = Described {
    description: "a const's one field",
};

pub fn events() -> Vec<Event> {
    vec![
        Event::Moved { dx: 1, dy: -2 },
        Event::Moved {
            dx: 3,
            dy: 4,
        },
        Event::Placed(5),
    ]
}

pub fn updated(base: Ref) -> Ref {
    Ref {
        kind: "updated",
        ..base
    }
}

pub fn span(from_the_start: bool) -> std::ops::Range<u32> {
    (if from_the_start {
        0
    } else {
        1
    })..2
}

pub fn placed(from_the_start: bool) -> Event {
    Event::Placed(if from_the_start {
        0
    } else {
        1
    })
}

pub const DEFAULT: Ref = Ref {
    kind: "default",
    render: 0,
};

pub static ONE_LINE: Ref = Ref { kind: "static", render: 1 };
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const elementType = {\n    kind: "forward_ref",\n    render,\n  };');
  expect(js).toContain('return { kind: "short", render };');
  // An object of one field is on one line where it fits, as a person
  // writes it, unless its literal writes a comment in it (ADR 0835,
  // amended, the user's decision); one of one field JS has, its `None`
  // left out, too.
  expect(js).toContain('return { description: "one field, too long for rustfmt\'s line" };');
  expect(js).toContain('return {\n    // Why it says so.\n    description: "one field, a comment in it",\n  };');
  expect(js).toContain('return {\n    description: /* why */ "one field",\n  };');
  expect(js).toContain("return { highWaterMark: 2048 };");
  expect(js).toContain('export const ONE_FIELD = { description: "a const\'s one field" };');
  expect(js).toContain('{ TAG: "Moved", dx: 1, dy: -2 },\n    {\n      TAG: "Moved",\n      dx: 3,\n      dy: 4,\n    },\n    { TAG: "Placed", _0: 5 },');
  expect(js).toContain('return {\n    ...base,\n    kind: "updated",\n  };');
  expect(js).toContain("return { start: fromTheStart ? 0 : 1, end: 2 };");
  expect(js).toContain('return { TAG: "Placed", _0: fromTheStart ? 0 : 1 };');
  expect(js).toContain('export const DEFAULT = {\n  kind: "default",\n  render: 0,\n};');
  expect(js).toContain('export const ONE_LINE = { kind: "static", render: 1 };');
});
