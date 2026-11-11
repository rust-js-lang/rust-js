// Mutations of src/lower/classes/member_functions.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const refusals = ["test/classes.test.ts", "-t", "what a class is refused"];

export const mutations: Mutation[] = [
  {
    name: "receiver-booleans-unrenamed",
    breaks:
      "a class's method's `this.below(x) || ..` is `!!this.below(x) || ..`, its `bool` paths left of the receiver's name",
    file: "src/lower/classes/member_functions.rs",
    find: '            Some(rest) => format!("this.{rest}"),\n',
    replace: "            Some(_) => path,\n",
    tests: ["test/corpus.test.ts", "-t", "^class_bool_methods.rs$"],
    snapshots: true,
  },
  {
    name: "namespace-receiver-kept",
    breaks:
      "a namespace's method keeps its `self` parameter, so a call through the value gives its arguments one place off (ADR 1037)",
    file: "src/lower/classes/member_functions.rs",
    find: "        params.remove(0);\n        return Ok(());\n",
    replace: "        return Ok(());\n",
    tests: [
      "test/namespaces.test.ts",
      "-t",
      "a namespace's methods are its module's functions",
    ],
  },
  {
    name: "subclass-super-unconstructed",
    breaks: "a class component's constructor never calls `super(props)`, so `this` is a ReferenceError and `this.props` unset",
    file: "src/lower/classes/member_functions.rs",
    find: "                function.body.insert(0, first);\n",
    replace: "                drop(first);\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
  {
    name: "super-placed-nowhere",
    breaks: "`super(props)` has no place, so a comment its constructor's body starts with is printed before it, where ReactART's Text writes it after (rd331)",
    file: "src/lower/classes/member_functions.rs",
    find: "StmtKind::Expr(Expr::call(Expr::super_(), given)).at(function.name_span);",
    replace: "StmtKind::Expr(Expr::call(Expr::super_(), given)).at(js::Span::NONE);",
    tests: ["test/class-components.test.ts", "-t", "comment comes after super"],
  },
  {
    name: "flattened-parameter-kept",
    breaks: "a constructor's flattened state stays one parameter, `function ReactPromise(state)`, its instance spreading it, where its keys are its parameters (rd380)",
    file: "src/lower/classes/member_functions.rs",
    find: "            spread_state(tcx, def_id, function)?;\n",
    replace: "",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made, read and replaced in place"],
  },
  {
    name: "prototype-forwarding-unplaced",
    breaks:
      "a prototype's method that only passes its parameters on is printed a function of its own, `X.prototype.m = function (..) { f(..); }` (rd715)",
    file: "src/lower/classes/member_functions.rs",
    find: "        class.forwarding.push(name.to_owned());\n",
    replace: "",
    tests: ["test/method-forwarding.test.ts", "-t", "a prototype's method that only passes its parameters on"],
  },
  {
    name: "class-receiver-left-to-tidy",
    breaks:
      "a class's method's `self` stays a parameter through tidy, so it's no `this`, its `let point` read as another variable",
    file: "src/lower/classes/member_functions.rs",
    find: "    if taken != Receiver::This {\n",
    replace: "    if taken == Receiver::This {\n",
    tests: ["test/corpus.test.ts", "-t", "^class_local_named_as_self.rs$"],
  },
  {
    name: "class-self-not-this",
    breaks:
      "a method's `self` stays the variable of its parameter, which no one declares, not `this`",
    file: "src/lower/classes/member_functions.rs",
    find: "    if js::read_as_this(body, &receiver) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "class_methods"],
  },
  {
    name: "class-constructor-returns",
    breaks:
      "a constructor's early `return { .. }` sets `this`'s fields and goes on, so the rest overwrites them",
    file: "src/lower/classes/member_functions.rs",
    find: "            if Some(i + 1) != last {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "class_methods"],
  },
  {
    name: "class-instance-not-this",
    breaks:
      "a constructor that makes its instance, `let fiber = Fiber { .. }; ..; fiber`, is refused, React's `FiberNode` its object form",
    file: "src/lower/classes/member_functions.rs",
    find: "    instance_as_this(body)?;\n",
    replace: "",
    tests: ["test/class-constructors.test.ts", "-t", "constructor's statements set this"],
  },
  {
    name: "class-instance-set-again-allowed",
    breaks:
      "a constructor whose instance is set again, `c = C { .. }`, is `this = ..`, no JS",
    file: "src/lower/classes/member_functions.rs",
    find: "    if js::Writes::sets(body, &instance) {\n",
    replace: "    if false && js::Writes::sets(body, &instance) {\n",
    tests: refusals,
  },
  {
    name: "class-constructor-none-set",
    breaks:
      "a constructor's `None` field is `this.actualDuration = undefined;`, where React's sets none until its flag's on",
    file: "src/lower/classes/member_functions.rs",
    find: "    js::undefined_keys::left_out(&mut props, layout);\n",
    replace: "",
    tests: ["test/class-constructors.test.ts", "-t", "constructor's statements set this"],
  },
  {
    name: "class-constructor-handle-on-this",
    breaks:
      "a handle on a constructor's instance's field, `bump(&fiber.reads)`, reads the instance by preparation's `this$`, `const this$ = this; .. return this$.reads;`, not by its class's name, `fiber` (#449's `prepare::this_handles` keeps its `this` right)",
    file: "src/lower/classes/member_functions.rs",
    find: "            set_this(&mut function.body)?;\n            this_outside_accessors(class, &mut function.body);\n",
    replace: "            set_this(&mut function.body)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^class_constructor_statements.rs$"],
    snapshots: true,
  },
  {
    name: "class-instance-return-kept",
    breaks:
      "a constructor's early `return fiber;` gives the instance, not `return;`",
    file: "src/lower/classes/member_functions.rs",
    find: "                stmt.kind = StmtKind::Return(None);\n",
    replace:
      "                stmt.kind = StmtKind::Return(Some(Expr::this()));\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^class_constructor_statements.rs$"],
  },
  {
    name: "method-self-in-function-undeclared",
    breaks:
      "a `with_this` function in a class's method reads `self` through a variable no one declares, a `ReferenceError`, not `const counter = this;` (ADR 0972)",
    file: "src/lower/classes/member_functions.rs",
    find: "        body.insert(0, StmtKind::Const(receiver.clone(), Expr::this()).at(js::Span::NONE));\n",
    replace: "",
    tests: ["test/js-functions.test.ts", "-t", "in a class's method"],
  },
];
