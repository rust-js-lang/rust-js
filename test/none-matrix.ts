// The `None` matrix (#855): an `Option` written to each kind of place, each
// with its `NoneForm` (#819, `representation/nones.rs`), by each writer, and
// read there by each reader, of each payload, `Some` and `None`: a program a
// place and payload, which rust-js must run as native Rust does. A place's
// `None` is `undefined` or `null` by its form; a writer stores by the
// place's form and a reader trusts what its form says, so a writer and a
// reader of two forms that disagree is a wrong answer: #819's cluster.

/** What an `Option` holds: its type, a value of it, and how two are told
 * apart in print. A nullable place of an `Option<i32>` is `Nested` (ADR
 * 1017), of a `()` `Value` (rd645). */
type Payload = { ty: string; some: string };

const payloads: Record<string, Payload> = {
  number: { ty: "i32", some: "7" },
  string: { ty: "String", some: `String::from("s")` },
  unit: { ty: "()", some: "()" },
  option: { ty: "Option<i32>", some: "None" },
  point: { ty: "Pt", some: "Pt { x: 3 }" },
};

/** Where the `Option` is held: what a program declares for it, and the
 * statements that write `w`, an expression of `Option<P>`, and then run
 * `read(place)`, of an expression that reads it there, `name` the block's
 * own. */
type Place = {
  items: (p: Payload, name: string) => string;
  block: (p: Payload, name: string, w: string, read: (place: string) => string) => string;
  /** The payloads rust-js refuses of the place by design, each with why:
   * a compile error that says so, not a gap, so no program of them. */
  refuses?: Record<string, string>;
};

const nullable = "#[cfg_attr(rust_js, rust_js::nullable)]";

const places: Record<string, Place> = {
  local: {
    items: () => "",
    block: (p, _, w, read) => `let place: Option<${p.ty}> = ${w};
        ${read("place")}`,
  },
  field: {
    items: (p, name) => `#[derive(Debug)]
struct Plain${camel(name)} {
    v: Option<${p.ty}>,
    n: u8,
}
`,
    block: (_, name, w, read) => `let mut h = Plain${camel(name)} { v: None, n: 1 };
        h.v = ${w};
        ${read("h.v")}
        println!("{} {:?}", h.n, h);`,
  },
  nullable_field: {
    items: (p, name) => `#[derive(Debug)]
struct Nullable${camel(name)} {
    ${nullable}
    v: Option<${p.ty}>,
    n: u8,
}
`,
    block: (_, name, w, read) => `let mut h = Nullable${camel(name)} { v: None, n: 1 };
        h.v = ${w};
        ${read("h.v")}
        println!("{} {:?}", h.n, h);`,
  },
  optional_field: {
    refuses: {
      unit: "an `Option<()>` nullable field's `Some(())` is `undefined`, so it can't be absent for `None` (rd645)",
      option: "a nested nullable field's `None` is absent already, its `Some(None)` `null` (ADR 1017)",
    },
    items: (p, name) => `#[derive(Debug)]
struct Optional${camel(name)} {
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    v: Option<${p.ty}>,
    n: u8,
}
`,
    block: (_, name, w, read) => `let h = Optional${camel(name)} { v: ${w}, n: 1 };
        ${read("h.v")}
        println!("{} {:?}", h.n, h);`,
  },
  result: {
    items: (p, name) => `fn made_${name}(v: Option<${p.ty}>) -> Option<${p.ty}> {
    v
}
`,
    block: (_, name, w, read) => `${read(`made_${name}(${w})`)}`,
  },
  nullable_result: {
    items: (p, name) => `${nullable}
fn made_${name}(v: Option<${p.ty}>) -> Option<${p.ty}> {
    v
}
`,
    block: (_, name, w, read) => `${read(`made_${name}(${w})`)}`,
  },
  param: {
    items: () => "",
    block: (p, name, w, read) => `fn read_${name}(v: Option<${p.ty}>) {
            ${read("v")}
        }
        read_${name}(${w});`,
  },
  nullable_param: {
    items: () => "",
    block: (p, name, w, read) => `#[cfg_attr(rust_js, rust_js::nullable(v))]
        fn read_${name}(v: Option<${p.ty}>) {
            ${read("v")}
        }
        read_${name}(${w});`,
  },
  thread_local: {
    items: (p, name) => `thread_local! {
    ${nullable}
    static CELL_${name.toUpperCase()}: std::cell::RefCell<Option<${p.ty}>> = const { std::cell::RefCell::new(None) };
}
`,
    block: (_, name, w, read) => `CELL_${name.toUpperCase()}.with(|cell| *cell.borrow_mut() = ${w});
        CELL_${name.toUpperCase()}.with(|cell| {
            let held = cell.borrow();
            ${read("(*held)")}
        });`,
  },
  vec_item: {
    items: () => "",
    block: (p, _, w, read) => `let items: Vec<Option<${p.ty}>> = vec![None, ${w}];
        ${read("items[1]")}
        println!("{items:?}");`,
  },
  cell: {
    items: () => "",
    block: (p, _, w, read) => `let cell: std::cell::RefCell<Option<${p.ty}>> = std::cell::RefCell::new(None);
        *cell.borrow_mut() = ${w};
        ${read("(*cell.borrow())")}`,
  },
  mut_ref: {
    items: () => "",
    block: (p, _, w, read) => `let mut held: Option<${p.ty}> = Some(Default::default());
        {
            let r = &mut held;
            *r = ${w};
            ${read("(*r)")}
        }
        println!("{held:?}");`,
  },
};

/** How the `Option` is made, given `x`, the `Some` or the `None` it is. */
const writers: Record<string, (x: string, p: Payload) => string> = {
  literal: (x) => x,
  through_a_local: (x, p) => `{
            let o: Option<${p.ty}> = ${x};
            o
        }`,
  from_a_nullable_field: (x) => `Source { v: ${x} }.v`,
  from_a_function: (x) => `given(${x})`,
  from_a_nullable_function: (x) => `given_nullable(${x})`,
  taken: (x) => `{
            let mut o = ${x};
            o.take()
        }`,
  mapped: (x) => `${x}.map(|v| v)`,
  filtered: (x) => `${x}.filter(|_| true)`,
  from_a_cell: (x) => `std::cell::Cell::new(${x}).take()`,
  popped: (x, p) => `{
            let mut items: Vec<${p.ty}> = Vec::new();
            if let Some(v) = ${x} {
                items.push(v);
            }
            items.pop()
        }`,
};

/** How the `Option` is read where it's held, `o` a place of it. */
const readers: Record<string, (o: string) => string> = {
  debug: (o) => `println!("debug {:?}", &${o});`,
  is_some: (o) => `println!("is_some {} {}", ${o}.is_some(), ${o}.is_none());`,
  matched: (o) => `match &${o} {
            Some(v) => println!("some {v:?}"),
            None => println!("none"),
        }`,
  equals_none: (o) => `println!("== {}", ${o} == None);`,
  question: (o) => `println!("? {:?}", asked(${o}.clone()));`,
  map_or: (o) => `println!("map_or {}", ${o}.as_ref().map_or(0, |_| 1));`,
  if_let: (o) => `if let Some(v) = &${o} {
            println!("if let {v:?}");
        } else {
            println!("if let none");
        }`,
  to_a_nullable_param: (o) => `nulled(${o}.clone());`,
  to_a_plain_field: (o) => `println!("plain {}", Plain { v: ${o}.clone() }.v.is_none());`,
};

/** `taken_by_a_local`'s `TakenByALocal`. */
const camel = (name: string) => name.replace(/(?:^|_)(\w)/g, (_, c: string) => c.toUpperCase());

/** What each program has besides its place: its payload's type, where
 * it's the crate's, and what writers and readers call. */
const prelude = (p: Payload) => `#[derive(Debug, Clone, Default, PartialEq)]
struct Pt {
    x: i32,
}

struct Source {
    ${nullable}
    v: Option<${p.ty}>,
}

#[derive(Debug)]
struct Plain {
    v: Option<${p.ty}>,
}

fn given(v: Option<${p.ty}>) -> Option<${p.ty}> {
    v
}

${nullable}
fn given_nullable(v: Option<${p.ty}>) -> Option<${p.ty}> {
    v
}

fn asked(o: Option<${p.ty}>) -> Option<String> {
    let v = o?;
    Some(format!("{v:?}"))
}

#[cfg_attr(rust_js, rust_js::nullable(v))]
fn nulled(v: Option<${p.ty}>) {
    println!("nulled {}", v.is_none());
}
`;

/** Each program of the matrix, by its name, `<place>__<payload>__<writer>`:
 * a writer's `Some` and `None` each read by every reader, so what's refused
 * or wrong is one place, payload and writer's. */
export function programs(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [place, at] of Object.entries(places)) {
    for (const [payload, p] of Object.entries(payloads)) {
      if (payload in (at.refuses ?? {})) continue;
      for (const [writer, write] of Object.entries(writers)) {
        const items: string[] = [];
        const blocks: string[] = [];
        for (const [x, which] of [[`Some(${p.some})`, "some"], ["None", "none"]] as const) {
          const name = `${writer}_${which}`;
          items.push(at.items(p, name));
          const read = (o: string) => Object.values(readers).map((r) => r(o)).join("\n        ");
          blocks.push(`println!("-- ${which}");\n    {\n        ${at.block(p, name, write(x, p), read)}\n    }`);
        }
        out[`${place}__${payload}__${writer}`] = `// The \`None\` matrix (test/none-matrix.ts, #855): an \`Option<${p.ty}>\`
// held as ${place.replaceAll("_", " ")}, written ${writer.replaceAll("_", " ")}, and read each way.
${prelude(p)}
${items.join("\n")}
fn main() {
    ${blocks.join("\n    ")}
}
`;
      }
    }
  }
  return out;
}

/** What each push runs of the matrix, where a `None`'s form changes: each
 * nullable place with a payload whose `None` and `Some` JS may confuse. */
export const sample = [
  "nullable_field__option__from_a_function",
  "nullable_field__unit__literal",
  "optional_field__number__from_a_nullable_field",
  "nullable_result__string__through_a_local",
  "nullable_param__point__from_a_nullable_function",
  "thread_local__option__taken",
];
