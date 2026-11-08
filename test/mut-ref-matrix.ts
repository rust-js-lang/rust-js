// The `&mut` matrix (ADR 1086): a program for each pointee and holder of
// a `&mut`, each running every operation on one, that rust-js must run as
// native Rust does. Each operation asks the `&mut`'s form, `MutRefForm`,
// and twelve wrong answers were two of them asking it apart (#614's
// family), so each pointee meets each holder and each operation here.

/** What a `&mut` points at: its type, what's declared for it, a first
 * value and another, and how a place of it is changed in place and has a
 * field written, as a person would write each. */
type Pointee = {
  ty: string;
  decl: string;
  init: string;
  alt: string;
  change: (place: string) => string;
  field?: (place: string) => string;
  /** A field holder of its own, `name` the operation's: a flattened
   * state's, its class's field, through its guard and lent to a function. */
  held?: (name: string, op: string) => { items: string; main: string };
  /** The operations rust-js refuses of it, each with why, so the list
   * only shrinks: each is left out of its programs; `<op>@<holder>` of one
   * held so alone. */
  refused?: Record<string, string>;
};

const pointees: Record<string, Pointee> = {
  number: { ty: "i32", decl: "", init: "1", alt: "5", change: (p) => `${p} += 10;` },
  string: { ty: "String", decl: "", init: `String::from("a")`, alt: `String::from("b")`, change: (p) => `${p}.push_str("!");` },
  vec: { ty: "Vec<i32>", decl: "", init: "vec![1, 2]", alt: "vec![3]", change: (p) => `${p}.push(9);` },
  struct: {
    ty: "Point",
    decl: `#[derive(Debug, Clone, Default, PartialEq)]
struct Point {
    x: i32,
    name: String,
}
`,
    init: `Point { x: 1, name: String::from("p") }`,
    alt: `Point { x: 5, name: String::from("q") }`,
    change: (p) => `${p}.x += 10;`,
    field: (p) => `${p}.x = 7;`,
  },
  copy_struct: {
    ty: "Pair",
    decl: `#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Pair {
    a: i32,
    b: i32,
}
`,
    init: "Pair { a: 1, b: 2 }",
    alt: "Pair { a: 5, b: 6 }",
    change: (p) => `${p}.a += 10;`,
    field: (p) => `${p}.b = 7;`,
  },
  enum_with_unit: {
    ty: "Switch",
    decl: `#[derive(Debug, Clone, Default, PartialEq)]
enum Switch {
    #[default]
    Off,
    On(i32),
}
`,
    init: "Switch::On(1)",
    alt: "Switch::Off",
    change: (p) => `match &mut ${p} {
        Switch::On(n) => *n += 10,
        off => *off = Switch::On(0),
    }`,
    field: (p) => `if let Switch::On(n) = &mut ${p} {
        *n = 7;
    }`,
  },
  enum_without_unit: {
    ty: "Shape",
    decl: `#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Circle(i32),
    Rect { w: i32, h: i32 },
}

impl Default for Shape {
    fn default() -> Self {
        Shape::Circle(0)
    }
}
`,
    init: "Shape::Circle(1)",
    alt: "Shape::Rect { w: 2, h: 3 }",
    change: (p) => `match &mut ${p} {
        Shape::Circle(r) => *r += 10,
        Shape::Rect { w, .. } => *w += 10,
    }`,
    field: (p) => `if let Shape::Circle(r) = &mut ${p} {
        *r = 7;
    }`,
  },
  flattened_state: {
    ty: "Status",
    decl: `#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { tries: u32 },
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done { value: String },
}

impl Default for Status {
    fn default() -> Self {
        Status::Pending { tries: 0 }
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Task {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: std::cell::RefCell<Status>,
    pub name: String,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Task {
    pub fn new(state: Status, name: String) -> Self {
        Task {
            state: std::cell::RefCell::new(state),
            name,
        }
    }
}
`,
    init: "Status::Pending { tries: 1 }",
    alt: `Status::Done { value: String::from("v") }`,
    change: (p) => `match &mut ${p} {
        Status::Pending { tries } => *tries += 10,
        Status::Done { value } => value.push('!'),
    }`,
    field: (p) => `if let Status::Pending { tries } = &mut ${p} {
        *tries = 7;
    }`,
    // Given on to generic code, or to a trait's default method, from its
    // class's guard: refused, as a flattened state's borrows are its
    // object's own keys, which count none (#855's first run).
    refused: {
      "passed_to_generic@field": "a flattened state lent past its guard, its borrows uncounted",
      "panic_in_callee@field": "a flattened state lent past its guard, its borrows uncounted",
      "self_replaced@field": "a flattened state lent past its guard, its borrows uncounted",
    },
    // Through its guard, and lent to a function whose `&mut` its box is,
    // `{ value: task }`, the class's object (#658).
    held: (name, op) => ({
      items: `fn lent_${name}(r: &mut Status) {
    ${op}
}
`,
      main: `let task = Task::new(Status::Pending { tries: 1 }, String::from("t"));
    {
        let mut state = task.state.borrow_mut();
        let r = &mut *state;
        ${op}
    }
    println!("{:?} {}", task.state.borrow(), task.name);
    lent_${name}(&mut task.state.borrow_mut());
    println!("{:?} {}", task.state.borrow(), task.name);`,
    }),
  },
  iterator: {
    ty: "std::ops::Range<i32>",
    decl: "",
    init: "1..4",
    alt: "7..9",
    change: (p) => `${p}.next();`,
  },
  // The space's other values (#855): a `bool` and an `f64`, boxes as a
  // number is; an `Option`, whose `None` is a value of its own; a tuple,
  // an array JS changes in place; and a `Box`, its value.
  boolean: { ty: "bool", decl: "", init: "true", alt: "false", change: (p) => `${p} = !${p};` },
  float: { ty: "f64", decl: "", init: "1.5", alt: "-2.25", change: (p) => `${p} += 0.5;` },
  option: {
    ty: "Option<i32>",
    decl: "",
    init: "Some(1)",
    alt: "None",
    change: (p) => `match &mut ${p} {
        Some(n) => *n += 10,
        none => *none = Some(0),
    }`,
    field: (p) => `if let Some(n) = &mut ${p} {
        *n = 7;
    }`,
  },
  tuple: {
    ty: "(i32, String)",
    decl: "",
    init: `(1, String::from("t"))`,
    alt: `(5, String::from("u"))`,
    change: (p) => `${p}.0 += 10;`,
    field: (p) => `${p}.1 = String::from("w");`,
  },
  boxed: { ty: "Box<i32>", decl: "", init: "Box::new(1)", alt: "Box::new(5)", change: (p) => `**&mut ${p} += 10;` },
};

/** Each operation through `r`, a `&mut` of the pointee, `alt` its other
 * value as the holder writes it, and `change` what changes one in place. */
type Op = (r: string, alt: string, change: (place: string) => string, field?: (place: string) => string) => string | undefined;

const operations: Record<string, Op> = {
  changed: (r, _, change) => change(`(*${r})`),
  whole_write: (r, alt) => `*${r} = ${alt};`,
  field_write: (r, _, __, field) => field?.(`(*${r})`),
  replace: (r, alt, change) => `let mut old = std::mem::replace(${r}, ${alt});
        ${change("old")}
        println!("replaced {old:?}");`,
  take: (r, _, change) => `let mut old = std::mem::take(${r});
        ${change("old")}
        println!("taken {old:?}");`,
  swap: (r, alt, change) => `let mut other = ${alt};
        std::mem::swap(${r}, &mut other);
        ${change("other")}
        println!("swapped {other:?}");`,
  cell_replace: (r, alt, change) => `let mut old = std::cell::Cell::from_mut(&mut *${r}).replace(${alt});
        ${change("old")}
        println!("cell {old:?}");`,
  panic_after_write: (r, alt, change) => `let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            *${r} = ${alt};
            ${change(`(*${r})`)}
            panic!("after the write");
        }));
        println!("caught {}", caught.is_err());`,
  // The space's other operations (#855): a reborrow changed; the `&mut`
  // given on, to the pointee's own method, to generic code, and to generic
  // code that panics after its change, which a box copies back as it
  // unwinds (#487); `*self` written whole by a trait's default method; and
  // a closure that captures it, called.
  reborrow: (r, _, change) => `let s = &mut *${r};
        ${change("(*s)")}`,
  passed_on: (r) => `Change::change(&mut *${r});`,
  passed_to_generic: (r) => `changed_by(&mut *${r});`,
  panic_in_callee: (r) => `let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| changed_then_panics(&mut *${r})));
        println!("caught {}", caught.is_err());`,
  self_replaced: (r) => `${r}.reset();`,
  closure_captures: (r, _, change) => `let mut by = || {
            ${change(`(*${r})`)}
        };
        by();
        by();`,
};

/** Where the `&mut` is held, and how the program makes it: `op` is the
 * operation's statements through `r`. A generic holder has no field to
 * write. */
type Holder = { generic: boolean; program: (p: Pointee, name: string, op: string) => { items: string; main: string } };

const holders: Record<string, Holder> = {
  local: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut v: ${p.ty} = ${p.init};
    {
        let r = &mut v;
        ${op}
    }
    println!("{v:?}");`,
    }),
  },
  parameter: {
    generic: false,
    program: (p, name, op) => ({
      items: `fn ${name}(r: &mut ${p.ty}) {
    ${op}
}
`,
      main: `let mut v: ${p.ty} = ${p.init};
    ${name}(&mut v);
    println!("{v:?}");`,
    }),
  },
  field: {
    generic: false,
    program: (p, name, op) =>
      p.held
        ? p.held(name, op)
        : {
            items: `#[derive(Debug)]
struct Holder${camel(name)} {
    f: ${p.ty},
    n: u32,
}
`,
            main: `let mut h = Holder${camel(name)} { f: ${p.init}, n: 3 };
    {
        let r = &mut h.f;
        ${op}
    }
    println!("{h:?}");`,
          },
  },
  item: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut items: Vec<${p.ty}> = vec![${p.init}, ${p.init}];
    {
        let r = &mut items[1];
        ${op}
    }
    println!("{items:?}");`,
    }),
  },
  result_payload: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut res: Result<${p.ty}, String> = Ok(${p.init});
    if let Ok(r) = &mut res {
        ${op}
    }
    println!("{res:?}");`,
    }),
  },
  closure_capture: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut v: ${p.ty} = ${p.init};
    let r = &mut v;
    let mut run = move || {
        ${op}
    };
    run();
    println!("{v:?}");`,
    }),
  },
  // The space's other holders (#855): an `Option`'s payload, a tuple's
  // field, a `Box`'s value, a `RefCell`'s guard, each item stepped by
  // `iter_mut`, a method's field of `self`, and a map's value.
  option_payload: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut opt: Option<${p.ty}> = Some(${p.init});
    if let Some(r) = &mut opt {
        ${op}
    }
    println!("{opt:?}");`,
    }),
  },
  tuple_field: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut t: (u8, ${p.ty}) = (3, ${p.init});
    {
        let r = &mut t.1;
        ${op}
    }
    println!("{t:?}");`,
    }),
  },
  boxed_value: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut b: Box<${p.ty}> = Box::new(${p.init});
    {
        let r = &mut *b;
        ${op}
    }
    println!("{b:?}");`,
    }),
  },
  cell_guard: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let cell = std::cell::RefCell::new(${p.init});
    {
        let mut guard = cell.borrow_mut();
        let r = &mut *guard;
        ${op}
    }
    println!("{:?}", cell.borrow());`,
    }),
  },
  iter_mut: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut items: Vec<${p.ty}> = vec![${p.init}, ${p.alt}];
    for r in items.iter_mut() {
        ${op}
    }
    println!("{items:?}");`,
    }),
  },
  method_self: {
    generic: false,
    program: (p, name, op) => ({
      items: `#[derive(Debug)]
struct Owner${camel(name)} {
    f: ${p.ty},
    n: u32,
}

impl Owner${camel(name)} {
    fn ${name}(&mut self) {
        let r = &mut self.f;
        ${op}
    }
}
`,
      main: `let mut o = Owner${camel(name)} { f: ${p.init}, n: 3 };
    o.${name}();
    println!("{o:?}");`,
    }),
  },
  map_value: {
    generic: false,
    program: (p, _, op) => ({
      items: "",
      main: `let mut m: std::collections::BTreeMap<u8, ${p.ty}> = std::collections::BTreeMap::new();
    m.insert(1, ${p.init});
    if let Some(r) = m.get_mut(&1) {
        ${op}
    }
    println!("{m:?}");`,
    }),
  },
  generic_fn: {
    generic: true,
    program: (p, name, op) => ({
      items: `fn ${name}<T: std::fmt::Debug + Default + Change>(r: &mut T) {
    ${op}
}
`,
      main: `let mut v: ${p.ty} = ${p.init};
    ${name}(&mut v);
    println!("{v:?}");`,
    }),
  },
  trait_default: {
    generic: true,
    program: (p, name, op) => ({
      items: `trait Ops${camel(name)}: std::fmt::Debug + Default + Change + Sized {
    // Named apart from the pointee's own, a \`Range\`'s \`take\`.
    fn ${name}_by_default(&mut self) {
        let r = self;
        ${op}
    }
}

impl Ops${camel(name)} for ${p.ty} {}
`,
      main: `let mut v: ${p.ty} = ${p.init};
    v.${name}_by_default();
    println!("{v:?}");`,
    }),
  },
};

/** `whole_write`'s `WholeWrite`, a type's name of an operation's. */
const camel = (name: string) => name.replace(/(?:^|_)(\w)/g, (_, c: string) => c.toUpperCase());

/** The pointee changed in place, as generic code asks it of a trait. */
const change = (p: Pointee) => `trait Change {
    fn change(&mut self);
    fn other() -> Self;
    fn reset(&mut self)
    where
        Self: Sized,
    {
        *self = Self::other();
    }
}

fn changed_by<U: Change>(x: &mut U) {
    x.change();
}

fn changed_then_panics<U: Change>(x: &mut U) {
    x.change();
    panic!("after the change");
}

impl Change for ${p.ty} {
    fn change(&mut self) {
        ${p.change("(*self)")}
    }

    fn other() -> Self {
        ${p.alt}
    }
}
`;

/** Each program of the matrix, by its name, `<pointee>__<holder>`. */
export function programs(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [pointee, p] of Object.entries(pointees)) {
    for (const [holder, h] of Object.entries(holders)) {
      const items: string[] = [];
      const calls: string[] = [];
      for (const [operation, op] of Object.entries(operations)) {
        // Through a `&mut` of a type parameter, its alt is given, and it's
        // changed by its trait; it has no field to write.
        const alt = h.generic ? "Change::other()" : p.alt;
        const changed = h.generic ? (place: string) => `${place.replace(/^\(\*(\w+)\)$/, "$1")}.change();` : p.change;
        const left =
          (h.generic && operation === "field_write") || operation in (p.refused ?? {}) || `${operation}@${holder}` in (p.refused ?? {});
        const body = left ? undefined : op("r", alt, changed, p.field);
        if (body === undefined) continue;
        const made = h.program(p, operation, body);
        items.push(made.items);
        calls.push(`println!("-- ${operation}");\n    {\n    ${made.main}\n    }`);
      }
      out[`${pointee}__${holder}`] = `// The \`&mut\` matrix (test/mut-ref-matrix.ts, ADR 1086): a \`&mut\` of a
// ${pointee.replaceAll("_", " ")}, held ${holder.replaceAll("_", " ")}, through each operation.
${p.decl}
${change(p)}
${items.join("\n")}
fn main() {
    ${calls.join("\n    ")}
}
`;
    }
  }
  return out;
}

/** What each push runs of the matrix, where its owner changes: the forms
 * that meet most, a box, a generic box, an object and a flattened state's
 * box, each held where #614's family went wrong; and each the matrix found
 * wrong, its fix's regression test (#636, #638). */
export const sample = [
  "number__generic_fn",
  "struct__closure_capture",
  "enum_with_unit__result_payload",
  "flattened_state__field",
  "iterator__parameter",
  "iterator__item",
  "vec__trait_default",
  // And of the space #855 grew it to, a guard's, an `iter_mut` item's, a
  // method's field of `self`, a map's value and an `Option`'s payload.
  "option__cell_guard",
  "boolean__iter_mut",
  "tuple__method_self",
  "float__map_value",
  "boxed__option_payload",
];
