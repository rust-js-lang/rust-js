// Mutations of src/lower/classes.rs (ADR 0093): a JS class (ADR 0761).
import type { Mutation } from "../../mutations";

const file = "src/lower/classes.rs";
const base = [
  "test/classes.test.ts",
  "-t",
  "React's Component is a constructor function",
];
const refusals = ["test/classes.test.ts", "-t", "what a class is refused"];

export const mutations: Mutation[] = [
  {
    name: "receiver-booleans-unrenamed",
    breaks:
      "a class's method's `this.below(x) || ..` is `!!this.below(x) || ..`, its `bool` paths left of the receiver's name",
    file: "src/lower/classes.rs",
    find: '            Some(rest) => format!("this.{rest}"),\n',
    replace: "            Some(_) => path,\n",
    tests: ["test/corpus.test.ts", "-t", "^class_bool_methods.rs$"],
    snapshots: true,
  },
  {
    name: "class-receiver-left-to-tidy",
    breaks:
      "a class's method's `self` stays a parameter through tidy, so it's no `this`, its `let point` read as another variable",
    file,
    find: "    if !matches!(\n        member(tcx, function),\n",
    replace: "    if matches!(\n        member(tcx, function),\n",
    tests: ["test/corpus.test.ts", "-t", "^class_local_named_as_self.rs$"],
  },
  {
    name: "class-unmarked",
    breaks:
      "a struct marked `#[rust_js::class]` is no class: its impl an object of functions, `Component.new(..)`, which `class X extends Component` can't run",
    file,
    find: "    tcx.def_kind(adt) == DefKind::Struct && marked(tcx, adt)\n",
    replace:
      "    tcx.def_kind(adt) == DefKind::Struct && marked(tcx, adt) && false\n",
    tests: base,
  },
  {
    name: "class-method-a-static",
    breaks:
      "a class's `&self` method is the constructor's own, `Component.setState`, so `this.setState` of an instance is `undefined`",
    file,
    find: "        DefKind::AssocFn if tcx.associated_item(def_id).is_method() => Member::Method,\n",
    replace: "",
    tests: base,
  },
  {
    name: "class-self-not-this",
    breaks:
      "a method's `self` stays the variable of its parameter, which no one declares, not `this`",
    file,
    find: "    if js::read_as_this(body, &receiver) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "class_methods"],
  },
  {
    name: "class-constructor-returns",
    breaks:
      "a constructor's early `return { .. }` sets `this`'s fields and goes on, so the rest overwrites them",
    file,
    find: "            if Some(i + 1) != last {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "class_methods"],
  },
  {
    name: "class-const-screaming",
    breaks:
      "a class's `const IS_REACT_COMPONENT` is the prototype's `IS_REACT_COMPONENT`, not `isReactComponent`, which React reads",
    file,
    find: "            true => crate::lower::camel_case(&named.to_lowercase()),\n",
    replace: "            true => named,\n",
    tests: base,
  },
  {
    name: "class-trait-allowed",
    breaks:
      "a class's `#[derive(Clone)]` compiles, and its clone is a plain object, no instance of its prototype",
    file,
    find: "                    && !tcx.associated_item_def_ids(id).is_empty()\n",
    replace: "                    && false\n",
    tests: refusals,
  },
  {
    name: "class-literal-allowed",
    breaks:
      "a class's struct literal outside its constructor compiles, a plain object without its prototype's methods",
    file,
    find: "                && is_class(tcx, adt.adt_def.did())\n                && !(in_constructor",
    replace: "                && false\n                && !(in_constructor",
    tests: refusals,
  },
  {
    name: "class-by-value-allowed",
    breaks:
      "a class's method of `self` by value compiles, its receiver no reference",
    file,
    find: "                            if !matches!(receiver.kind(), ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Adt(..))) {\n",
    replace: "                            if false {\n",
    tests: refusals,
  },
  {
    name: "class-field-method-clash-allowed",
    breaks:
      "a class's method named as its field compiles, and `this.count` hides `count()`",
    file,
    find: "                    if matches!(member(tcx, item), Some(Member::Method | Member::Getter | Member::Const))\n",
    replace: "                    if false\n",
    tests: refusals,
  },
  {
    name: "class-generic-allowed",
    breaks: "a generic class compiles, one prototype for each of its types",
    file,
    find: "                if tcx.generics_of(id).own_params.iter().any(|p| p.kind.is_ty_or_const()) {\n",
    replace: "                if false {\n",
    tests: refusals,
  },
  {
    name: "class-unmade-allowed",
    breaks:
      "a class of no `fn new` compiles, no constructor function to make it",
    file,
    find: "                if constructor_of(tcx, id).is_none() {\n",
    replace: "                if false {\n",
    tests: refusals,
  },
  {
    name: "class-impl-unmarked-allowed",
    breaks:
      "a class's impl not marked `#[rust_js::class]` compiles, the attribute on one half of the class",
    file,
    find: '                    (true, false) => refuse(span, "struct\'s impl is marked `#[rust_js::class]` too"),\n',
    replace: "",
    tests: refusals,
  },
  {
    name: "class-handle-this-unnamed",
    breaks:
      "a handle a class's method hands on, `bump(&self.reads)`, reads `this` in its accessors, the handle's own, so the field is never set (#359)",
    file,
    find: "    if !kept {\n        return;\n    }\n",
    replace: "    if kept || !kept {\n        return;\n    }\n",
    tests: ["test/classes.test.ts", "-t", "Cell fields"],
  },
  {
    name: "getter-a-method",
    breaks:
      "a class's method marked `rust_js::getter` is a prototype method, `position.leftDiagonal` a function, not its value (#357)",
    file,
    find: "        DefKind::AssocFn if tcx.associated_item(def_id).is_method() && marked_getter(tcx, def_id) => Member::Getter,\n",
    replace: "",
    tests: ["test/classes.test.ts", "-t", "getter is its prototype"],
  },
  {
    name: "getter-of-no-class-allowed",
    breaks:
      "`rust_js::getter` on a plain struct's method compiles, a getter of no prototype (#357)",
    file,
    find: "            _ if marked_getter(tcx, id) && class_of(tcx, id).is_none() => {\n",
    replace: "            _ if false => {\n",
    tests: ["test/classes.test.ts", "-t", "what a getter is refused"],
  },
  {
    name: "getter-of-any-shape-allowed",
    breaks:
      "a getter of `&mut self`, of arguments or of no `self` compiles, which no read of a property calls (#357)",
    file,
    find: "                        && !(member(tcx, item) == Some(Member::Getter) && reads_as_getter(tcx, item))\n",
    replace: "                        && false\n",
    tests: ["test/classes.test.ts", "-t", "what a getter is refused"],
  },
  {
    name: "class-display-no-member",
    breaks:
      "a class's `Display`'s `fmt` is no prototype method, its `self` no `this` (#358)",
    file: "src/lower/classes.rs",
    find: "    if shown_class(tcx, def_id).is_some() || trait_member_class(tcx, def_id).is_some() {\n",
    replace: "    if trait_member_class(tcx, def_id).is_some() {\n",
    tests: ["test/classes.test.ts", "-t", "Display is its prototype"],
  },
  {
    name: "class-display-refused",
    breaks:
      "a class's `impl Display` is refused as any trait impl with items is (#358)",
    file: "src/lower/classes.rs",
    find: "            DefKind::Impl { of_trait: true } if displayed_class(tcx, id).is_none() => {\n",
    replace: "            DefKind::Impl { of_trait: true } => {\n",
    tests: ["test/classes.test.ts", "-t", "Display is its prototype"],
  },
  {
    name: "class-instance-not-this",
    breaks:
      "a constructor that makes its instance, `let fiber = Fiber { .. }; ..; fiber`, is refused, React's `FiberNode` its object form",
    file,
    find: "    instance_as_this(body)?;\n",
    replace: "",
    tests: ["test/classes.test.ts", "-t", "constructor's statements set this"],
  },
  {
    name: "class-instance-set-again-allowed",
    breaks:
      "a constructor whose instance is set again, `c = C { .. }`, is `this = ..`, no JS",
    file,
    find: "    if set_again {\n",
    replace: "    if false {\n",
    tests: refusals,
  },
  {
    name: "class-constructor-none-set",
    breaks:
      "a constructor's `None` field is `this.actualDuration = undefined;`, where React's sets none until its flag's on",
    file,
    find: "    js::undefined_keys::left_out(&mut props, layout);\n",
    replace: "",
    tests: ["test/classes.test.ts", "-t", "constructor's statements set this"],
  },
  {
    name: "class-constructor-handle-on-this",
    breaks:
      "a handle on a constructor's instance's field, `bump(&fiber.reads)`, reads the handle's `this`, `{ get value() { return this.reads; } }`",
    file,
    find: "        Member::Constructor => set_this(&mut function.body).map(|()| this_outside_accessors(class, &mut function.body)),\n",
    replace: "        Member::Constructor => set_this(&mut function.body),\n",
    tests: ["test/corpus.test.ts", "-t", "^class_constructor_statements.rs$"],
  },
  {
    name: "class-instance-return-kept",
    breaks:
      "a constructor's early `return fiber;` gives the instance, not `return;`",
    file,
    find: "                stmt.kind = StmtKind::Return(None);\n",
    replace:
      "                stmt.kind = StmtKind::Return(Some(Expr::this()));\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^class_constructor_statements.rs$"],
  },
  {
    name: "class-identity-impls-refused",
    breaks:
      "a class's `PartialEq` of `ptr::eq` and `Hash` of `ptr::hash`, React's fibers in a `Set`, are refused as a trait's methods",
    file,
    find: "                    && !identity\n",
    replace: "",
    tests: ["test/classes.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "class-home-the-structs",
    breaks:
      "a class whose struct is another module's is declared nowhere, `createFiber(): any`",
    file,
    find: "    Some(tcx.parent_module_from_def_id(constructor_of(tcx, adt)?.as_local()?))\n",
    replace: "    Some(tcx.parent_module_from_def_id(adt.as_local()?))\n",
    tests: ["test/classes.test.ts", "-t", "constructor's statements set this"],
  },
  {
    name: "object-self-a-parameter",
    breaks:
      "an object's method's `self` is a parameter, `function containsNode(scopeInstance, fiber)`, where its callers give it as `this`, `scope.containsNode(fiber)` (ADR 0943)",
    file,
    find: "        Some(Member::Method | Member::Getter | Member::Own)\n",
    replace: "        Some(Member::Method | Member::Getter)\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "an object's methods are its literal's own",
    ],
  },
  {
    name: "object-method-a-prototypes",
    breaks:
      "an object's impl's method is any impl's, `ScopeInstance.containsNode(scope, fiber)`, its literal holding none (ADR 0943)",
    file,
    find: "    if object_of_impl(tcx, imp).is_some() {\n",
    replace: "    if object_of_impl(tcx, imp).is_some() && false {\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "an object's methods are its literal's own",
    ],
  },
  {
    name: "object-by-value-unchecked",
    breaks:
      "an object's method of `self` by value, which JS's `this` can't be, compiles (ADR 0943)",
    file,
    find: "                    if member(tcx, item) != Some(Member::Own) {\n",
    replace: "                    if true {\n",
    tests: ["test/classes.test.ts", "-t", "what an object is refused"],
  },
  {
    name: "object-generic-allowed",
    breaks:
      "a `#[rust_js::object]` of a type parameter compiles, its methods one function for every value, which a JS caller gives no dictionary (ADR 0943)",
    file,
    find: "                if generic {\n",
    replace: "                if false {\n",
    tests: ["test/classes.test.ts", "-t", "what an object is refused"],
  },
  {
    name: "object-unmarked-impl-unchecked",
    breaks:
      "an object's impl unmarked compiles, its methods any impl's, its literal holding none (ADR 0943)",
    file,
    find: '                    (true, false) => refuse(span, "struct\'s impl is marked `#[rust_js::object]` too"),\n',
    replace: "",
    tests: ["test/classes.test.ts", "-t", "what an object is refused"],
  },
  {
    name: "method-self-in-function-undeclared",
    breaks:
      "a `with_this` function in a class's method reads `self` through a variable no one declares, a `ReferenceError`, not `const counter = this;` (ADR 0972)",
    file,
    find: "        body.insert(0, StmtKind::Const(receiver.clone(), Expr::this()).at(js::Span::NONE));\n",
    replace: "",
    tests: ["test/js-functions.test.ts", "-t", "in a class's method"],
  },
  {
    name: "constructor-as-unknown-constructed",
    breaks:
      "a constructor held as a value of any shape, `js::new(js::unknown(&fake), ())`, compiles: a second way to hold one beside `Class::of`",
    file,
    find: '                ("new", true, _) => {\n',
    replace: '                ("new", true, _) if false => {\n',
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "a constructor held as a value of any shape is refused",
    ],
  },
  {
    name: "constructor-as-unknown-prototype",
    breaks:
      'a constructor\'s prototype read of it as a value of any shape, `js::get(js::unknown(&fake), "prototype")`, compiles, beside `Class::of(&fake).prototype()`',
    file,
    find: '                ("get", true, _) if prototype() => {\n',
    replace: '                ("get", true, _) if prototype() && false => {\n',
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "a constructor held as a value of any shape is refused",
    ],
  },
  {
    name: "class-as-unknown-constructed",
    breaks:
      "a `js::Class` constructed as a value of any shape, `js::new(js::unknown(class), ())`, compiles, beside `js::new(class, ..)`",
    file,
    find: '                ("new", _, true) => {\n',
    replace: '                ("new", _, true) if false => {\n',
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "a constructor held as a value of any shape is refused",
    ],
  },
  {
    name: "held-class-as-unknown-named-new",
    breaks:
      "a class held, an untyped `js::Class`, constructed as a value of any shape is told to be given to `js::new` as it is, which rustc refuses, not to its constructor's binding, `call_static`",
    file,
    find: '                ("new", _, true) if is_untyped_class(tcx, held) => {\n',
    replace: '                ("new", _, true) if false => {\n',
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "a constructor held as a value of any shape is refused",
    ],
  },
  {
    name: "class-of-a-closure",
    breaks:
      "`Class::of` of a closure compiles, an arrow `new` throws a `TypeError` of where it runs",
    file,
    find: "            if is_class_of(tcx, did) && matches!(",
    replace: "            if is_class_of(tcx, did) && false && matches!(",
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "a constructor held as a value of any shape is refused",
    ],
  },
  {
    name: "class-trait-of-any-impls",
    breaks:
      "a trait a plain struct implements too is a trait of classes, its class's impl no dictionary, which the plain struct's callers need (rd283)",
    file: "src/lower/classes.rs",
    find: "        && impls.all(|imp| class_of_impl(tcx, imp).is_some())\n",
    replace: "        && impls.any(|imp| class_of_impl(tcx, imp).is_some())\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-trait-bound-allowed",
    breaks:
      "a trait of classes bounds a type parameter, whose generic code is given no dictionary (rd283)",
    file: "src/lower/classes.rs",
    find: "                && is_class_trait(tcx, bound.def_id())\n",
    replace: "                && false\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-trait-dyn-allowed",
    breaks:
      "a trait of classes is a `dyn`'s, which no dictionary is made for (rd283)",
    file: "src/lower/classes.rs",
    find: "                ty::Dynamic(predicates, ..) => predicates.principal_def_id().filter(|&t| is_class_trait(tcx, t)),\n",
    replace:
      "                ty::Dynamic(predicates, ..) => predicates.principal_def_id().filter(|_| false),\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-trait-rest-overridden",
    breaks:
      "a class's own method of a default that takes rest arguments is given them spread, where it takes a slice (rd283)",
    file: "src/lower/classes.rs",
    find: "                        .is_some_and(|item| crate::lower::bindings::takes_rest(tcx, item))\n",
    replace: "                        .is_some_and(|_| false)\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-trait-method-hidden",
    breaks:
      "a trait of classes' method named as the class's field is on its prototype, where the instance's own property hides it (rd283)",
    file: "src/lower/classes.rs",
    find: "                    if fields.contains(&named) && !own.is_some_and(|own| field_accessor(tcx, own).is_some()) {\n",
    replace: "                    if false {\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-defaults-unshared",
    breaks:
      "each class takes its own copy of a trait's default that prints alike, not `B.prototype.render = A.prototype.render = function` (rd283)",
    file: "src/lower/classes.rs",
    find: "                .any(|f| f.name == function.name && f.params == function.params && f.body == function.body)\n",
    replace: "                .any(|_| false)\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-defaults-shared-unlike",
    breaks:
      "a trait default a class's own impl writes otherwise is shared with another's, so `unmount` is one function (rd283)",
    file: "src/lower/classes.rs",
    find: "                .any(|f| f.name == function.name && f.params == function.params && f.body == function.body)\n",
    replace: "                .any(|f| f.name == function.name)\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_methods.rs$"],
    snapshots: true,
  },
  {
    name: "class-default-self-unread",
    breaks:
      "a trait default on a class's prototype reads its `self` as a variable, not `this` (rd283)",
    file: "src/lower/classes.rs",
    find: "            && js::read_as_this(&mut body, &receiver)\n",
    replace: "            && false\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "namespace-methods-an-objects",
    breaks:
      "a `#[rust_js::namespace]`'s methods are its type's object's, given `self`, not its module's functions (ADR 1037)",
    file: "src/lower/classes.rs",
    find: "        return method.then_some(Member::Namespace);\n",
    replace: "        return None;\n",
    tests: [
      "test/namespaces.test.ts",
      "-t",
      "a namespace's methods are its module's functions",
    ],
  },
  {
    name: "namespace-receiver-kept",
    breaks:
      "a namespace's method keeps its `self` parameter, so a call through the value gives its arguments one place off (ADR 1037)",
    file: "src/lower/classes.rs",
    find: "        params.remove(0);\n        return Ok(());\n",
    replace: "        return Ok(());\n",
    tests: [
      "test/namespaces.test.ts",
      "-t",
      "a namespace's methods are its module's functions",
    ],
  },
  {
    name: "namespace-by-value-allowed",
    breaks: "a namespace's method of `self` by value isn't refused (ADR 1037)",
    file: "src/lower/classes.rs",
    find: "                        matches!(sig.inputs()[0].kind(), ty::Ref(_, _, ty::Mutability::Not))\n",
    replace: "                        !sig.inputs().is_empty()\n",
    tests: ["test/namespaces.test.ts", "-t", "what a namespace is refused"],
  },
  {
    name: "field-accessor-reborrow-refused",
    breaks:
      "a method that gives a field, `&self.internal_root` reborrowed where it's given back, is no field accessor: a prototype method, `$fieldCell(this, ..)` (rd304)",
    file: "src/lower/classes.rs",
    find: "                && matches!(borrow.kind, Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not))) => {}\n",
    replace: "                && false => {}\n",
    tests: ["test/classes-traits.test.ts", "-t", "prototype's"],
  },
  {
    name: "field-accessor-of-any-path",
    breaks:
      "a method that gives what isn't a field of its `self`, `&OTHER.f`, or of a field reached through another reference, is read as a field of `self` (rd304)",
    file: "src/lower/classes.rs",
    find: "                    [deref] if is_self && matches!(deref.kind, Adjust::Deref(DerefAdjustKind::Builtin)) => true,\n",
    replace: "                    [_] => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_field_borrows.rs$"],
  },
  {
    name: "field-accessor-held-allowed",
    breaks:
      "a trait of classes' method that gives a field is held as a function, `let given = <Root as RootType>::internal_root;`, though it's no function JS has (rd304)",
    file: "src/lower/classes.rs",
    find: "                && !fields_given(tcx, typing_env, (method, args)).is_empty()\n",
    replace: "                && fields_given(tcx, typing_env, (method, args)).is_empty() && false\n",
    tests: ["test/classes-traits.test.ts", "-t", "bounds no type parameter"],
  },
  {
    name: "field-accessor-name-clash-refused",
    breaks:
      "a trait of classes' method that gives a field, named as that field, `fn items(&self) -> &RefCell<..> { &self.items }`, is refused as a method the field hides, though it's no method (rd304)",
    file: "src/lower/classes.rs",
    find: "                    if fields.contains(&named) && !own.is_some_and(|own| field_accessor(tcx, own).is_some()) {\n",
    replace: "                    if fields.contains(&named) {\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_field_borrows.rs$"],
  },
  {
    name: "class-trait-unknown-self-any",
    breaks:
      "a default's call of a trait of classes' method, of a `Self` not known, may borrow anything, so a borrow held across `self.internal_root()` is counted and checked, `$borrow($fieldCell(this, ..))` (rd304)",
    file: "src/lower/classes.rs",
    find: "        Ok(None) => (implementations(tcx, callee).into_iter().flatten())\n",
    replace: "        Ok(None) => (implementations(tcx, callee).into_iter().flatten().filter(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_field_borrows.rs$"],
  },
];
