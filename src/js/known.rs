//! What the lowering knows of an expression that its kind doesn't say,
//! each fact a `Known`, held by `Facts` (`super::Facts`).

/// What the lowering knows of an expression that its kind doesn't say:
/// the one place a later pass reads it from, not from where it's written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Known {
    /// A call of a method of a `str` or a slice, as Rust's types tell,
    /// `s.startsWith(t)`: never a crate's or a binding's of the same
    /// name, `P.at(p)`, which may do anything.
    Read,
    /// A binding's read of a property, `get x`, `get X.y` or `get []`,
    /// `js::get`'s, or of the method a binding calls on a JS value,
    /// `obj.send` of `obj.send(a)`: a getter may answer it, which may do
    /// anything, and may give another value read again (ADRs 0672, 0750).
    /// So is a binding's JS operator of any value, `key in value`, `value
    /// instanceof f`, `a == b`, or a conversion that may throw, `+value`:
    /// JS may throw of it, or run a trap or a `valueOf`, never a value
    /// nothing reads that can go (#462).
    Getter,
    /// A binding's write of a property, `set x` or `set X.y`, the target
    /// of its assignment: a setter may answer it, which may do anything,
    /// throw too, so the write never only makes a value (ADR 0660,
    /// amended).
    Setter,
    /// A read of a field Rust can't change while the body runs: of a
    /// struct a `&T` parameter is to, or a `&'static T` a call gives, the
    /// field no cell (`LocalFacts::lasting`). Only JS can give another
    /// value of it, a getter's (`Getter`) of a struct JS made (rd285).
    Lasting,
    /// A slice's items in a range the body's ranges show in bounds,
    /// `items.slice(a, b)`: an array made here, which nothing else holds,
    /// as `$slice(..)` is (ADR 0806).
    Sliced,
    /// A set's items in its order, `Array.from(set)` of a `HashSet`'s JS
    /// `Set`, or a class of it, `$KeySet`: an array made here, whose
    /// `forEach` of a callback of the item alone is the set's own,
    /// `set.forEach((x) => ..)`, which steps the same items in the same
    /// order, the set unchanged while Rust borrows it (ADR 0877).
    SetItems,
    /// A string's `char`s, an array made here, `Array.from(text)`: a case
    /// mapping's or a text escape's (ADRs 0070, 0971), which joined is the
    /// text itself.
    Chars,
    /// Never `undefined`, though maybe `null`: a whole `Option` that's
    /// `null` or a value already, a nullable thread-local's whole read
    /// (`nulled::reads_nullable_local`, ADR 0949), a nullable function's
    /// call (`calls::given_by`), a nullable field's read where it's made
    /// (ADR 0620), or what's set through a variable holding a cell's
    /// handle that MIR shows is so, `make(context)` of
    /// `set(Some(make(context)))`. A nullable place stores it as it is
    /// (`Storage::written`, ADR 0841). It says nothing of whether an
    /// `Option` is there, which may be `None`: that's `Present`'s (#539).
    Defined,
    /// There, never `undefined` nor `null`: a `Some`'s value its `Option`
    /// doesn't box, `map`'s, `then`'s, `kind.object()`'s. An `Option` made
    /// of it where it's tested, `t ? v : undefined`, is there exactly where
    /// `t` holds (`Expr::presence`, ADR 0910). Never `undefined`, so a
    /// nullable place stores it as it is too.
    Present,
    /// The `None` a `take()` writes over a place rust-js made, a variable
    /// or a crate struct's field, a plain data property, read into a
    /// `const` just before (ADR 0996): nothing but that `const` sees the
    /// place between, so the write may go where the `const` is `Some`,
    /// upstream's `if (x.f !== null) { const c = x.f; x.f = null; .. }`,
    /// `None` written over `None` changing nothing.
    Taken,
    /// A read of a thread-local made where it's first used, `key()`
    /// (ADR 0037, amended): making it runs its initializer, which Rust runs
    /// inside the call that uses it, after what that call is given, so it's
    /// read there, not before them; read again, it's what it made.
    /// `LocalKey::set` of one not made yet makes it of the value given,
    /// `key.initWith?.(cell)` (#252).
    LocalRead,
    /// A `Rev`, an `Enumerate` or a `Map` made in place, the recognizers'
    /// JS iterator, not std's struct, as the call that takes it says
    /// (`mir_iter_call`, ADR 0871): a step past that call, which takes its
    /// form apart, reads it here, not from its form.
    InPlace,
    /// A `$iter` of std's iterator over an array's items, a slice's `Iter`
    /// or a `Vec`'s `IntoIter` (`steps_its_array`), of the array it steps from
    /// its start, as Rust's types tell: what the array's `values()` steps
    /// too, `iterators::stepped_slices`'s (rd270), and an index loop, `for
    /// (let i = 0; i < v.length; i++)`, of a `for` (ADR 1042). Or a
    /// binding's `Iterator.from(list)` of an indexed list, a `NodeList`'s,
    /// whose iteration is an array's (`#[rust_js::indexed]`).
    ArrayItems,
    /// One of the program's functions, its own or a library's, by its
    /// name, `noop` (`items.rs::fn_ref`): a binding nothing assigns, the
    /// same function wherever and whenever it's read (`Expr::is_fixed`,
    /// ADR 0693, amended).
    Item,
    /// A read of another module's variable, an imported `export let x`
    /// that its module sets (ADRs 0270, 0096, amended): what it is where it's
    /// read, as ES's live binding, never another name's that read it
    /// before.
    Live,
    /// A binding's method, `obj.send` of `obj.send(a)`: JS the program
    /// declared, which may do anything, whatever its name, `join` too (the
    /// review's P2, ADR 0890), never one of JS's own that only read.
    Binding,
    /// A dictionary the function was given, `TStore` of `TStore.join(t)`:
    /// its methods are the program's, whatever their names, never JS's own
    /// that only read (ADR 0900).
    Evidence,
    /// A call that throws only where what it's given does: std's print,
    /// `console.log(text)`, whose stream never fails (ADRs 0132, 0901); or
    /// a function each call of which does, one of the crate's that never
    /// unwinds, `loudDrop_drop` (ADR 0902). Or a read of a field of a
    /// struct JS made, which a getter may answer (`Getter`), where Rust
    /// reads a place, which can't panic: a getter's throw is none Rust
    /// sees, so a scope's `finally` runs past it (rd227, rd228).
    NeverThrows,
    /// A binding's call that runs no code but its own, none of the
    /// program's, so it sets none of its variables: a weak set's `has(key)`,
    /// `#[rust_js::runs_nothing]`'s, or one that runs only what it's given,
    /// given only numbers and text (ADRs 0867, 0898). Its callee and
    /// arguments are read before it as any call's are (ADR 0277, amended).
    RunsNothing,
    /// A `bool`, as Rust's types tell, where JS's form doesn't show one: a
    /// `bool` constant read by its name, `ENABLED` (ADR 0031, amended),
    /// `true` or `false` alone, which `&&` and `||` give as it is.
    Boolean,
    /// A call that never returns, MIR's of a `-> !` function's, a panic's:
    /// what's after it is never run (`js::ends`, ADR 0954).
    Diverges,
    /// The test of an `if` whose `else` the program wrote, its branches
    /// each as written: an `else` after a `then` that leaves stays one,
    /// `if (a) { ..; return; } else if (b) { return; }` (ADR 0237,
    /// amended), not an `if` after.
    Else,
    /// Of an `Option`, that its `None` is only ever `undefined`, never
    /// `null`, as its representation guarantees: a binding's result
    /// TypeScript types `T | undefined`, `#[rust_js::optional]`'s,
    /// `window.event`, or a variable each of whose writes is one, a
    /// `Some(..)` or the crate's `None` (`OptionWrites::nones`). Of its
    /// test, `x != null`, the test is written `x !== undefined`
    /// (`stmt_walk::strict_presence`, ADR 0298, amended).
    NoneUndefined,
    /// Of an `Option`, that its `None` is only ever `null`, never
    /// `undefined`: a call of a function whose result is
    /// `#[rust_js::nullable]`, a binding's TypeScript types `T | null`,
    /// `node.textContent`, or a variable each of whose writes is one or a
    /// `Some(..)`. Of its test, `x != null`, the test is written `x !==
    /// null` (ADR 0298, amended).
    NoneNull,
    /// The test of a `let … else`'s guard, `if (parentA == null) { break; }`,
    /// where the program wrote it: never the test of the loop it's first in,
    /// `while ((parentA = a.return) != null)` (ADRs 0860, 0879, amended).
    Guard,
    /// The test of an `if` the program wrote, `if r.is_err()` or `if let
    /// Err(e) = r`, never one MIR makes of a `match`'s arm: what's said
    /// right above it is said of that `if`, which a `try` may fold, its
    /// `catch` an `Err` test's (`prepare/catches.rs`, #489).
    Written,
    /// Of an `f64`, that it's a whole number below 10^21 in size, never
    /// `-0`: what a `#[rust_js::whole]` binding gives, a WebIDL integer,
    /// `response.status`. Its `{}` is JS's text of it, `${response.status}`,
    /// which is Rust's (ADR 0994, amended).
    Whole,
    /// A binding's call of no argument that gives what JS's own iteration
    /// of its receiver steps, `set.values()` and `set.keys()` of a `Set`,
    /// `map.entries()` of a `Map`, `#[rust_js::iterates_receiver]`'s:
    /// collected, `Array.from(set)`, as JS writes it (`Expr::collected`,
    /// the user's decision, bw8).
    ReceiverIterated,
    /// A function the program holds as a value, a variable's of an `Rc` or
    /// a `Box` of a closure, `let commit_hook: Rc<dyn Fn()> = Rc::new(..)`:
    /// a `const` of its arrow, `const commitHook = () => { .. }`, as
    /// TypeScript writes a value of a function's type, never a function
    /// declared, which a closure a `let` binds is (ADR 0882, amended).
    Valued,
    /// Text whatever it's of, a `String` written to, `s` of `s += t`: a
    /// `+` of it makes text of what's joined to it, as `"" +` would
    /// (`Expr::text_joined`, ADR 0639, amended).
    Text,
    /// A comparison of what's never a NaN, as Rust's types tell: of
    /// integers, `n < 0`, whose negation is its converse, `n >= 0`
    /// (`Expr::negated_test`, #366); a `===` of text, `bool`s, `char`s or
    /// an object equal only to itself too, whose search of an array is the
    /// array's `includes`, which finds what `===` does but a NaN (rd353,
    /// ADR 0966, amended).
    NeverNaN,
    /// A JS iterator already, as its Rust type tells (`as_js_iterator`'s):
    /// a variable holding a std adapter, `takeWhile` of `takeWhile.filter(..)`,
    /// is never made one again by `Iterator.from`, whose call of its
    /// `[Symbol.iterator]` makes a held `$Steps` close as it's done, before
    /// the statement that holds it ends (ADR 0570, amended).
    Iterator,
    /// An `undefined` argument that's no argument where it's last, alone
    /// too: a `None` of a parameter that may be left out, or a `()` (ADRs
    /// 0443, 0441), as lowering says it of the call's parameter types
    /// (`calls::given_to_js_but`), which prepare's one rule leaves out
    /// (ADR 0313, amended).
    Absent,
    /// A `resume_unwind(x)`'s `$unwinding(x)` of a variable of the body's
    /// own, in a body that lends no `&mut` to a `dyn Any`, or one a
    /// closure captured, in a family of bodies none of which does, so
    /// nothing is written through `x` (`BodyFacts::resumed_as_given`). Where `x` is a
    /// `catch`'s payload, `const x = $unwound(thrown)`, never set again,
    /// it's what was thrown, `throw thrown`, as `$unwinding($unwound(e))`
    /// is `e` (ADR 0706, amended, rd550).
    AsGiven,
    /// A call `f.call(value, ..args)` of a method `f` read of `value`, the
    /// same value, which nothing changed between, `js::function::call(
    /// js::method((value, key)), Some(value), args)`'s (`BodyFacts::method_calls`):
    /// where the read is in place, the method's call, `value.key(..args)`,
    /// which gives it that `this`, as preparation writes it
    /// (`Expr::own_method_call`, rd334).
    OwnMethodCall,
    /// A call of a function its holder's member is, which reads no `this`:
    /// a field the crate fills with its own functions, `system.link(..)`
    /// (`Pointers::call_facts`), or a namespace's method through a value
    /// (ADR 1037). It may be read apart and called alone (ADR 0610, #533).
    ThisUnread,
    /// Of those, a namespace's method, which `prepare::forwarding::members`
    /// takes its value apart for; not a struct's field (#561, #584).
    NamespaceMethod,
    /// A member a key names of a value of any shape, `js::method((value,
    /// key))`'s, which is read only as its value's method (rd334): the
    /// form of no other call, which a `let` of its value only it reads
    /// is read in place of (`Expr::own_keyed_method_receiver`, rd226b).
    KeyedMethod,
    /// A collect that closes what it steps, at its end and as a panic
    /// unwinds, `$collectDropping(iter)` of a `Vec`'s: what it consumed,
    /// one dropped by closing it, is dropped there, nothing after
    /// (`consumed_call`, #660).
    ClosesWhatItSteps,
    /// A part of a tuple, `t[1]`, of the array rust-js made of it
    /// (`Representation::project`): a read of an array's item, which calls
    /// nothing and throws nothing, so tests of a tuple's parts may read in
    /// the order its pattern names them (`decisions::paths::part`, #674).
    /// Never a binding's `js::get(o, &1.0)`, which a getter may answer
    /// (#756).
    Part,
    /// An iterator's value as `lower::iterators::value` wrote it, which only
    /// that module reads (ADR 1092, #730): a JS iterator of the value it
    /// steps, `$iterator(value, next, ..)` or `$given(value, ..)`.
    IterStepped,
    /// Of those, a range's numbers, `$iterator(range, step)`.
    IterRange,
    /// Of those, the crate's own or a dictionary's steps, not `$given`'s.
    IterOwn,
    /// Std's compiled struct given what a use site knows, `$knowing(it, ..)`.
    IterKnowing,
    /// One closed to drop what it hasn't given, `$closing(it, drop)`.
    IterClosing,
    /// One lent, `$lent(it)`, which nothing past it closes.
    IterLent,
    /// A `flat_map()`'s closure whose result is made in place, as its `flat_map()` is.
    IterResultInPlace,
    /// One stepped from its back, `$rev(it)`.
    IterRev,
    /// One stepped every `n`th item, `$stepBy(it, n, ..)`.
    IterStepBy,
    /// `repeat_n(x, n)`'s, `$repeatingN(x, n, ..)`.
    IterRepeatingN,
    /// A `rev()` its maker made knowing its back (`reversed_knowing`, ADR 0545).
    IterKnowsBack,
}
