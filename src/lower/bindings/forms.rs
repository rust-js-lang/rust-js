//! How a call of a binding is written, the JS form its `#[link_name]`
//! says (ADR 0024): a call, a getter, `new`, an operator or a question
//! asked of a global, `typeof X === "function"`.

use super::js_name;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// How a call to a JS function is written, from its `#[link_name]` (ADR 0024).
pub(in crate::lower) enum JsForm {
    /// `name(..)`, or `this.name(..)` for a method.
    Call(String),
    /// `this.name`.
    Get(String),
    /// `this.name = value`.
    Set(String),
    /// `new Name(..)`.
    New(String),
    /// `this` itself: an unchecked cast.
    This,
    /// `this(..)`: `this` is a JS function, like React's `setCount`.
    CallThis,
    /// `new this(..)`: `this` is a JS function, constructed, `js::new`'s
    /// (ADR 0695).
    NewThis,
    /// `delete []`: `delete this[key]`, an own property gone, `js::delete`'s
    /// (ADR 0697).
    Delete,
    /// A JSX element (ADR 0040): `<div>`, `<>`, an imported component like
    /// `<react#StrictMode>`, or `<*>` for the component given first.
    Jsx(String),
    /// `prop className`: a JSX attribute of `this`, the element being built,
    /// or a field of `this`, an object being built. Just `prop`: the name
    /// comes first, as a string literal.
    Prop(Option<String>),
    /// `{}`, or `{__html}`: an object, with the arguments as its fields, in
    /// order. With `prop`, it builds `style={{ color: "red" }}` and options.
    Object(Vec<String>),
    /// `this instanceof Class`: a checked one, as a `bool`.
    InstanceOf(String),
    /// `instanceof`: `this instanceof constructor`, of a constructor
    /// given as a value, `js::instance_of`'s (ADR 0878).
    InstanceOfGiven,
    /// `get []`: `this[key]`, a property by a name given (ADR 0225).
    GetIndex,
    /// `in []`: `key in this`, whether it has a property by a name given.
    In,
    /// `!!`: `!!this`, whether it's truthy.
    Truthy,
    /// `===`: `this === other`, JS's strict equality (ADR 0721).
    StrictEquals,
    /// `==`: `this == other`, JS's loose equality, which coerces, `"0.0" ==
    /// 0` (ADR 1012).
    LooseEquals,
    /// `typeof`: `typeof this`, what it is, as text (ADR 0310).
    TypeOf,
    /// `typeof !== "undefined"`, `typeof === "function"` or `typeof ===
    /// "number"`: what's asked of the global of the binding given (ADRs
    /// 0597, 0942, 0269).
    Asks(Asked),
    /// `"" +`: `"" + this`, any value as JS's `+` makes it text (ADRs 0310,
    /// 0639).
    Text,
    /// `set []`: `this[key] = value`.
    SetIndex,
    /// `import()`: the item given, of its module loaded when it's asked
    /// for; `import(*)`, the module whose default export it is (ADR 0304).
    Import { module: bool },
    /// `for in`: the keys `for (const key in this)` visits, `js::keys_in`'s,
    /// a JS iterator, `$keysIn(this)`, which a `for` loop of it is that
    /// loop (ADR 0719).
    KeysIn,
    /// `throw`: `throw this`, a JS value thrown as it is, `js::throw`'s
    /// (ADR 0760).
    Throw,
    /// `class`: the constructor function of the class its type argument
    /// is, `Component`, `js::class::<Component>()`'s (ADR 0761).
    Class,
    /// `null`: JS's `null`, an `Option` of any type's `None` where it's
    /// written, `js::null()`'s (ADR 0935, amended).
    Null,
    /// `typeof object`: `this`, an untagged enum's value, where it's an
    /// object or an array, `typeof this === "object" && this !== null`,
    /// else `undefined`, `js::Kind::object`'s (ADR 0910).
    TypeofObject,
    /// `function`: a JS function of the closure given, `function (a, b) {
    /// .. }`, its first parameter JS's `this`, its second the tuple of what
    /// it's called with, `js::function::with_this`'s (ADR 0972).
    WithThis,
}

/// What `js::is_defined`, `js::is_function` or `js::is_number` asks of a
/// binding's global: whether the host has it, `typeof MessageChannel !==
/// "undefined"` (ADR 0597), whether it's a function, `typeof setTimeout ===
/// "function"` (ADR 0942), or a number, `typeof devicePixelRatio ===
/// "number"` (ADR 0269).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Asked {
    Defined,
    Function,
    Number,
}

impl Asked {
    /// The function that asks it, as a program writes it.
    pub(in crate::lower) fn function(self) -> &'static str {
        match self {
            Asked::Defined => "js::is_defined",
            Asked::Function => "js::is_function",
            Asked::Number => "js::is_number",
        }
    }

    /// The `typeof` it's of, `"function"`; `None` of whether it's there,
    /// `!== "undefined"`.
    pub(in crate::lower) fn type_of(self) -> Option<&'static str> {
        match self {
            Asked::Defined => None,
            Asked::Function => Some("function"),
            Asked::Number => Some("number"),
        }
    }
}

pub(in crate::lower) fn js_form(tcx: TyCtxt<'_>, def_id: DefId) -> JsForm {
    let name = js_name(tcx, def_id);
    match name.as_str() {
        "this" => return JsForm::This,
        "this()" => return JsForm::CallThis,
        "new this()" => return JsForm::NewThis,
        "delete []" => return JsForm::Delete,
        "prop" => return JsForm::Prop(None),
        "get []" => return JsForm::GetIndex,
        "in []" => return JsForm::In,
        "!!" => return JsForm::Truthy,
        "===" => return JsForm::StrictEquals,
        "==" => return JsForm::LooseEquals,
        "instanceof" => return JsForm::InstanceOfGiven,
        "typeof" => return JsForm::TypeOf,
        "typeof !== \"undefined\"" => return JsForm::Asks(Asked::Defined),
        "typeof === \"function\"" => return JsForm::Asks(Asked::Function),
        "typeof === \"number\"" => return JsForm::Asks(Asked::Number),
        "\"\" +" => return JsForm::Text,
        "set []" => return JsForm::SetIndex,
        "import()" => return JsForm::Import { module: false },
        "import(*)" => return JsForm::Import { module: true },
        "for in" => return JsForm::KeysIn,
        "throw" => return JsForm::Throw,
        "class" => return JsForm::Class,
        "null" => return JsForm::Null,
        "typeof object" => return JsForm::TypeofObject,
        "function" => return JsForm::WithThis,
        _ => {}
    }
    if let Some(tag) = name.strip_prefix('<').and_then(|t| t.strip_suffix('>')) {
        return JsForm::Jsx(tag.to_string());
    }
    if let Some(keys) = name.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        let keys = keys.split(',').map(str::trim).filter(|k| !k.is_empty());
        return JsForm::Object(keys.map(str::to_string).collect());
    }
    if let Some(prop) = name.strip_prefix("prop ") {
        return JsForm::Prop(Some(prop.to_string()));
    }
    // Each prefix, and the form of what follows it.
    type Form = fn(String) -> JsForm;
    let forms: [(&str, Form); 4] = [
        ("get ", JsForm::Get),
        ("set ", JsForm::Set),
        ("new ", JsForm::New),
        ("instanceof ", JsForm::InstanceOf),
    ];
    for (prefix, form) in forms {
        if let Some(rest) = name.strip_prefix(prefix) {
            return form(rest.to_string());
        }
    }
    JsForm::Call(name)
}
