// Mutations of src/lower/jsx.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "undefined-children-written",
    breaks: "\`<Frame {..Default::default()} />\` is \`<Frame>{undefined}</Frame>\`",
    file: "src/lower/jsx.rs",
    find: "                Prop::Field(name, value) if name == \"children\" && matches!(value.kind, js::ExprKind::Undefined) => {}",
    replace: "                Prop::Field(name, value) if name == \"children\" && false && matches!(value.kind, js::ExprKind::Undefined) => {}",
    tests: ["test/jsx.test.ts", "-t", "named props and the rest from a base"],
  },
  {
    name: "rest-field-not-spread",
    breaks: "a `Rest` a component passes on is a `rest` prop, not `{...rest}`",
    file: "src/lower/jsx.rs",
    find: "                _ => vec![Prop::Spread(value)],",
    replace: "                _ => vec![Prop::Field(\"rest\".into(), value)],",
    tests: ["test/jsx.test.ts", "-t", "doesn't name as ...rest"],
  },
  {
    name: "passed-handler-wrapped",
    breaks: "an optional handler passed on to an element is wrapped, `(e) => { if (onClick != null) { onClick(e); } }`",
    file: "src/lower/jsx.rs",
    find: "    (given && called && handler != param).then(|| tested.clone())",
    replace: "    (false && given && called && handler != param).then(|| tested.clone())",
    tests: ["test/jsx.test.ts", "-t", "optional event handler on as it is"],
  },
  {
    name: "component-props-in-struct-order",
    breaks: "a component's props are in its struct's order, not as written: `<Frame title={title} tags={..}>` of `tags=.. title=..`",
    file: "src/lower/jsx.rs",
    find: "            && adt.adt_def.is_struct()\n            && fields.len() == adt.adt_def.non_enum_variant().fields.len()",
    replace: "            && false\n            && fields.len() == adt.adt_def.non_enum_variant().fields.len()",
    tests: ["test/jsx.test.ts","-t","a shared capture of a let reads alike, past an element.s key"],
  },
  {
    name: "props-read-before-constant-base",
    breaks: "a component's props are read into `const`s before its children where only a base's constant comes after them",
    file: "src/lower/jsx.rs",
    find: "                value.has_effects() && !self.reads_unchanging(children, out)\n",
    replace: "                true\n",
    tests: ["test/jsx.test.ts", "-t", "no dictionary"],
  },
  {
    name: "flattened-literal-spread",
    breaks: "a flattened struct made in JSX is spread, `{...{ href: \"/learn\" }}`, not its fields as attributes",
    file: "src/lower/jsx.rs",
    find: "        let js::ExprKind::Object(props) = value.kind else {\n",
    replace: "        if matches!(value.kind, js::ExprKind::Object(_)) {\n            return Ok(vec![Prop::Spread(value)]);\n        }\n        let js::ExprKind::Object(props) = value.kind else {\n",
    tests: ["test/jsx.test.ts", "-t", "flattened struct"],
  },
  {
    name: "flattened-chain-not-inlined",
    breaks: "a flattened struct's own flattened field is a prop, `html={..}`, not its fields",
    file: "src/lower/jsx.rs",
    find: "                    for attr in self.flattened_attrs(types[i].1, value, &shadowing, span)? {",
    replace: "                    attrs.push(Prop::Field(types[i].0.clone(), value));\n                    for attr in Vec::<Prop>::new() {",
    tests: ["test/jsx.test.ts", "-t", "chain of flattened"],
  },
  {
    name: "shadowed-name-given-silently",
    breaks: "a Rust caller's `href` of flattened props the props have too is dropped, not refused",
    file: "src/lower/jsx.rs",
    find: "                    if !matches!(value.kind, js::ExprKind::Undefined) {",
    replace: "                    if false {",
    tests: ["test/jsx.test.ts", "-t", "chain of flattened"],
  },
  {
    name: "props-in-companion-order",
    breaks: "a component's props are in its struct's order, not as they're written",
    file: "src/lower/jsx.rs",
    find: "        let sorted = order(&attrs);",
    replace: "        let sorted: Vec<usize> = (0..attrs.len()).collect();",
    tests: ["test/jsx.test.ts", "-t", "flattened props where they"],
  },
  {
    name: "reordered-effects-not-read-first",
    breaks: "props that do something, reordered as written, are made in that order, not Rust's",
    file: "src/lower/jsx.rs",
    find: "            if attrs.iter().filter(|attr| effects(attr)).count() > 1 {",
    replace: "            if false {",
    tests: ["test/jsx.test.ts", "-t", "flattened props where they"],
  },
  {
    name: "prop-written-in-macro",
    breaks: "a prop made by a macro, `vec![..]`, is placed by where that macro is defined",
    file: "src/lower/jsx.rs",
    find: "                .or_insert(self.thir[field.expr].span.source_callsite().lo());",
    replace: "                .or_insert(self.thir[field.expr].span.lo());",
    tests: ["test/jsx.test.ts", "-t", "no dictionary"],
  },
  {
    name: "negation-spilled",
    breaks: "a negation of what reads the same is spilled before a later child's statement: `const condition = !done`",
    file: "src/lower/jsx.rs",
    find: "            js::ExprKind::Unary(js::UnaryOp::Not, a) => self.reads_alike(a, out),\n",
    replace: "            js::ExprKind::Unary(js::UnaryOp::Not, a) => false && self.reads_alike(a, out),\n",
    tests: ["test/jsx.test.ts", "-t", "react.dev.s Preview tests its error in place"],
  },
  {
    name: "tag-as-children",
    breaks: "`react::tag(Comp)` is read as a DOM element taking `Comp` as children",
    file: "src/lower/jsx.rs",
    find: "            (\"$\", 1) => match args.next().expect(\"one\") {\n",
    replace: "            (\"$$\", 1) => match args.next().expect(\"one\") {\n",
    tests: ["test/jsx.test.ts", "-t", "tag that.s a value"],
  },
  {
    name: "unchanging-children-read-first",
    breaks: "a component's children that read only a state's value, declared first, make every prop a `const` before the JSX, beside a computed `className`",
    file: "src/lower/jsx.rs",
    find: "                value.has_effects() && !self.reads_unchanging(children, out)\n",
    replace: "                value.has_effects()\n",
    tests: ["test/jsx.test.ts","-t","flattened props where they're written, none given left out"],
  },
  {
    name: "field-read-hoisted",
    breaks: "a field of a plain Rust value nothing writes again is read into a `const` before a later child's statement, `const width = p.size === \"S\" ? ..`, where it stays in place",
    file: "src/lower/jsx.rs",
    find: "            js::ExprKind::Member(object, _) => {\n",
    replace: "            js::ExprKind::Member(object, _) if false => {\n",
    tests: [
      "test/jsx.test.ts",
      "-t",
      "field of what never changes in place"
    ]
  },
  {
    name: "getter-in-place",
    breaks: "a JS object's getter, `n.textContent`, is read after what writes it",
    file: "src/lower/jsx.rs",
    find: "                        ty::GenericArgKind::Type(t) => !self.refers_mutably(t) && !self.is_js_object(t),\n",
    replace: "                        ty::GenericArgKind::Type(t) => !self.refers_mutably(t),\n",
    tests: ["test/jsx.test.ts","-t","spreads a struct through a reference as the struct"],
  },
  {
    name: "rest-base-each-field",
    breaks: "an update of a props pattern's rest, `..props`, gives each of its fields, the props' own `href` among them, where it's `{...props}`",
    file: "src/lower/jsx.rs",
    find: "                    rest = Some(*object);\n",
    replace: "                    attrs.push(Prop::Field(name, Expr::member(*object, \"_\")));\n",
    tests: [
      "test/jsx.test.ts",
      "-t",
      "flattened props with a field of them set"
    ]
  },
  {
    name: "rest-spread-after-update",
    breaks: "a rest spread is after what the update gives, `className=\"link\" {...props}`, so the caller's `className` holds where the update's should",
    file: "src/lower/jsx.rs",
    find: "                            Prop::Spread(_) => written.get(&name).copied().or(first),\n",
    replace: "                            Prop::Spread(_) => written.get(&name).copied(),\n",
    tests: [
      "test/jsx.test.ts",
      "-t",
      "flattened props with a field of them set"
    ]
  },
  {
    name: "rest-spread-twice",
    breaks: "a rest a nested flattened struct's base gives too, `..props.html`, is spread again, `{...props} {...props}`",
    file: "src/lower/jsx.rs",
    find: "                                rest = Some(value)\n",
    replace: "                                attrs.push(Prop::Spread(value))\n",
    tests: [
      "test/jsx.test.ts",
      "-t",
      "flattened props with a field of them set"
    ]
  },
  {
    name: "children-and-unmade",
    breaks: "`(level == 1).then(|| jsx! { <b /> })` is `level === 1 ? <b /> : undefined`, not `level === 1 && <b />`",
    file: "src/lower/jsx.rs",
    find: "if !matches!(none.kind, js::ExprKind::Undefined) {",
    replace: "if true {",
    tests: ["test/jsx.test.ts","-t","tests an Option of an object by its truth"],
  },
  {
    name: "children-tuple-not-recursed",
    breaks: "only a first child, of the tuple of tuples `jsx!` makes, is `test && ..`: the next keep `? .. : undefined`",
    file: "src/lower/jsx.rs",
    find: ".map(|(item, &field)| self.shown_if(field, item))",
    replace: ".map(|(item, _)| item)",
    tests: ["test/jsx.test.ts","-t","an element's props are an ElementType, rendered as a tag"],
  },
  {
    name: "base-spread-last",
    breaks: "a base's spread is after what it's updated with, `<Panel wide {...props} />`, which then wins",
    file: "src/lower/jsx.rs",
    find: "                        Prop::Spread(_) => Some(BytePos(0)),",
    replace: "                        Prop::Spread(_) => None,",
    tests: ["test/jsx.test.ts","-t","updated from a reference are spread"],
  },
  {
    name: "unknown-props-unspread",
    breaks: "props that are a JS value of any shape, `{...props}` of a `js::Unknown`, are refused",
    file: "src/lower/jsx.rs",
    find: "            && !js_object(spread)\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "JS values of any shape"],
  },
  {
    name: "spread-of-none-refused",
    breaks: "JSX props spread of an \`Option\` of a JS value, \`undefined\` where it's none, is refused",
    file: "src/lower/jsx.rs",
    find: "            && !self.option_of(spread).is_some_and(js_object)\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "JS values of any shape"],
  },
  {
    name: "handler-of-binding-passed",
    breaks: "`onClick={() => alert()}` is `onClick={alert}`, which shows the event",
    file: "src/lower/jsx.rs",
    find: "    if calls_rust\n",
    replace: "    if true\n",
    tests: ["test/jsx.test.ts", "-t", "calls a function with nothing"],
  },
  {
    name: "handler-calling-nothing-kept",
    breaks: "`onClick={() => onClear()}`, not `onClick={onClear}`",
    file: "src/lower/jsx.rs",
    find: "        && args.is_empty()\n",
    replace: "        && args.is_empty()\n        && false\n",
    tests: ["test/jsx.test.ts", "-t", "calls a function with nothing"],
  },
  {
    name: "captured-import-spilled",
    breaks: "an import jsx! captures for a key that might do something is spilled, `const match = Fragment`, where it reads the same in place",
    file: "src/lower/jsx.rs",
    find: "                    || self.krate.imports[&self.module].values().any(|alias| alias == name)\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "react.dev.s Page tests its home and blog index in place"],
  },
  {
    name: "binding-callback-returned",
    breaks: "a component's callback of a binding that may give a value returns it, `onRecord={() => globalThis.record(5)}`",
    file: "src/lower/jsx.rs",
    find: "            _ if super::bindings::is_binding(self.tcx, def_id) => super::bindings::returns_undefined(self.tcx, def_id),\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "callback returns a call that gives undefined"],
  },
  {
    name: "rust-callback-kept",
    breaks: "a component's callback of a function of the crate's keeps its call a statement, `onSubmit={() => { submit(false); }}`",
    file: "src/lower/jsx.rs",
    find: "            _ => def_id.is_local(),\n",
    replace: "            _ => false,\n",
    tests: ["test/lowering.test.ts", "-t", "callback returns a call that gives undefined"],
  },
  {
    name: "closure-callback-kept",
    breaks: "a component's callback of a closure of the crate's keeps its call a statement, `onNext={() => { next(); }}`",
    file: "src/lower/jsx.rs",
    find: "                matches!(thir[args[0]].ty.peel_refs().kind(), ty::Closure(..))\n",
    replace: "                false\n",
    tests: ["test/lowering.test.ts", "-t", "callback returns a call that gives undefined"],
  },
  {
    name: "jsx-shown-if-ternary",
    breaks: "a child shown only if a test holds, `Option::map` to an element, is `t != null ? <p /> : undefined`, not the `t != null && <p />` a person writes",
    file: "src/lower/jsx.rs",
    find: "        if !matches!(none.kind, js::ExprKind::Undefined) {\n            return value;\n        }",
    replace: "        if !matches!(none.kind, js::ExprKind::Undefined) || child == child {\n            return value;\n        }",
    tests: ["test/react.test.ts", "-t", "React components are hand-written JSX, and React runs them"],
  },
  {
    name: "jsx-truthy-shown-tested",
    breaks: "JSX shows an Option of an object by `!!error && ..`, where the original writes `error && ..`",
    file: "src/lower/jsx.rs",
    find: "            js::ExprKind::Unary(js::UnaryOp::Not, not)\n                if let js::ExprKind::Unary(js::UnaryOp::Not, x) = &not.kind\n                    && self.maps_js_object(child) =>",
    replace: "            js::ExprKind::Unary(js::UnaryOp::Not, not)\n                if let js::ExprKind::Unary(js::UnaryOp::Not, x) = &not.kind\n                    && false =>",
    tests: ["test/jsx.test.ts", "-t", "JSX tests an Option of an object"],
  },
  {
    name: "component-child-shown-ternary",
    breaks: "a component's child shown only if a test holds is `test ? <el /> : undefined`, where an element's is `test && <el />`",
    file: "src/lower/jsx.rs",
    find: "                            self.shown_if(child, value)\n",
    replace: "                            value\n",
    tests: ["test/jsx.test.ts", "-t", "react.dev.s NavigationBar shows its options in place"],
  },
  {
    name: "key-last",
    breaks: "a component's `key` is last, not where it was written, `<Item value={path} key={path} />`",
    file: "src/lower/jsx.rs",
    find: "            \"key\" => {\n",
    replace: "            \"key\" if false => {\n",
    tests: ["test/jsx.test.ts", "-t", "a key is where it was written"],
  },
  {
    name: "key-after-base",
    breaks: "a `key` written before `{..base}` follows its spread, `<Panel {...props} key={match} wide />`",
    file: "src/lower/jsx.rs",
    find: "                    Prop::Spread(v) if v.span.is_none() => true,\n",
    replace: "                    Prop::Spread(v) if v.span.is_none() => false,\n",
    tests: ["test/jsx.test.ts", "-t", "props updated from a reference are spread"],
  },
  {
    "name": "jsx-spread-reference-refused",
    "breaks": "a struct spread through a reference, `{...&rest}`, is refused as a spread of a non-struct",
    "file": "src/lower/jsx.rs",
    "find": "        let spread = ty.peel_refs();",
    "replace": "        let spread = ty;",
    "tests": [
      "test/jsx.test.ts",
      "-t",
      "spreads a struct through a reference"
    ]
  },
  {
    "name": "jsx-key-alike-moves-children",
    "breaks": "a component's key that reads the same anywhere puts its children in `const`s first",
    "file": "src/lower/jsx.rs",
    "find": "        if !simple {",
    "replace": "        if true {",
    "tests": [
      "test/jsx.test.ts",
      "-t",
      "a key that reads alike keeps"
    ]
  },
  {
    "name": "referred-children-unspread",
    "breaks": "children given by reference are one array child, `{[a, b]}`, not each a child",
    "file": "src/lower/jsx.rs",
    "find": "                            let child = self.referred_children(child);",
    "replace": "                            let child = self.strip(child);",
    "tests": [
      "test/jsx.test.ts",
      "-t",
      "children declared any node"
    ]
  },
  {
    name: "mir-alike-unknown",
    breaks: "what MIR says reads alike doesn't, so JSX reads each attribute into a `const` first",
    file: "src/lower/jsx.rs",
    find: "                self.alike_local(name)\n",
    replace: "                false\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    "name": "jsx-mir-field-unplaced",
    "breaks": "a component's `key` from MIR goes last, `<Tag variant={tag} key={tag} />`, its fields' values placed nowhere",
    "file": "src/lower/jsx.rs",
    "find": "                        value.span = self.js_span(self.thir[f.expr].span);\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "attribute-value-unmapped",
    "breaks": "an attribute's constant value from MIR maps to no Rust line, `allow=` none in the source map",
    "file": "src/lower/jsx.rs",
    "find": "let mut value = value.or_at(self.js_span(value_span));",
    "replace": "let mut value = value;",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    name: "props-read-in-jsx-order",
    breaks: "props written out of the struct's order are made in the JSX's, not Rust's: `t` logged before `h`",
    file: "src/lower/jsx.rs",
    find: "        if !value.is_constant() {\n            let old = std::mem::replace(value, Expr::undefined());\n",
    replace: "        if false {\n            let old = std::mem::replace(value, Expr::undefined());\n",
    tests: ["test/jsx.test.ts", "-t", "flattened props where they"],
  },
];
