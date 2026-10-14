// CodeMirror, through bindings (ADR 0028): the editors' states, and what
// the `Editor` component does to its view. Both editors follow the system's
// light or dark setting.

use std::marker::PhantomData;

use js::{JsObject, object};
use webapi::Element;

/// Anything CodeMirror takes as an extension, an array of them too.
pub struct Extension(PhantomData<JsObject>);
pub struct Compartment(PhantomData<JsObject>);
/// A change to an editor's configuration: a compartment's new contents.
pub struct Effect(PhantomData<JsObject>);
pub struct EditorState(PhantomData<JsObject>);
pub struct EditorView(PhantomData<JsObject>);
/// A document's text.
pub struct Text(PhantomData<JsObject>);

#[allow(clashing_extern_declarations)]
unsafe extern "Rust" {
    #[link_name = "codemirror#basicSetup"]
    safe static basic_setup: &'static Extension;
    #[link_name = "@codemirror/theme-one-dark#oneDark"]
    safe static one_dark: &'static Extension;
}

#[cfg_attr(rust_js, rust_js::link_name = "@codemirror/lang-rust#rust")]
fn rust_language() -> &'static Extension {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "@codemirror/lang-javascript#javascript")]
fn javascript_language() -> &'static Extension {
    unreachable!()
}

/// Extensions together are one.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
#[allow(unused_variables)]
fn together(this: Vec<&'static Extension>) -> &'static Extension {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new @codemirror/state#Compartment")]
fn new_compartment() -> &'static Compartment {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "of")]
#[allow(unused_variables)]
fn compartment_of(this: &Compartment, content: &Extension) -> &'static Extension {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "reconfigure")]
#[allow(unused_variables)]
fn reconfigure(this: &Compartment, content: &Extension) -> &'static Effect {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "codemirror#EditorView.contentAttributes.of")]
#[allow(unused_variables)]
fn content_attributes(attributes: &JsObject) -> &'static Extension {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "@codemirror/state#EditorState.readOnly.of")]
#[allow(unused_variables)]
fn read_only(value: bool) -> &'static Extension {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "@codemirror/state#EditorState.create")]
#[allow(unused_variables)]
fn create_state(config: &StateConfig) -> &'static EditorState {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "new codemirror#EditorView")]
#[allow(unused_variables)]
fn new_editor(config: &EditorConfig) -> &'static EditorView {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "destroy")]
#[allow(unused_variables)]
pub fn destroy(this: &EditorView) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get state")]
#[allow(unused_variables)]
pub fn editor_state(this: &EditorView) -> &'static EditorState {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "setState")]
#[allow(unused_variables)]
fn set_editor_state(this: &EditorView, state: &EditorState) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "dispatch")]
#[allow(unused_variables)]
fn dispatch(this: &EditorView, transaction: &Transaction) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get doc")]
#[allow(unused_variables)]
fn doc(this: &EditorState) -> &'static Text {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "toString")]
#[allow(unused_variables)]
fn text_string(this: &Text) -> String {
    unreachable!()
}

// CodeMirror's configurations: JS objects that only CodeMirror reads.
#[allow(dead_code)]
struct StateConfig {
    doc: String,
    extensions: &'static Extension,
}

#[allow(dead_code)]
struct EditorConfig {
    state: &'static EditorState,
    parent: &'static Element,
}

#[allow(dead_code)]
struct Transaction {
    effects: &'static Effect,
}

// Made once, as the module loads (ADR 0037). One theme compartment serves
// both editors: it only names the slot a view reconfigures.
thread_local! {
    static THEME: &'static Compartment = new_compartment();
    static SOURCE: &'static Extension = together(vec![
        basic_setup,
        rust_language(),
        compartment_of(THEME.with(|theme| *theme), theme_for(false)),
        content_attributes(object::from_entries(vec![("aria-label".to_string(), "Rust source".to_string())])),
    ]);
    // Read-only, but still selectable and copyable. Highlighted as JS for a
    // generated file, plain text when it shows rustc's diagnostics.
    static JS_OUTPUT: &'static Extension = output(Some(javascript_language()));
    static PLAIN_OUTPUT: &'static Extension = output(None);
}

fn output(language: Option<&'static Extension>) -> &'static Extension {
    let mut extensions = vec![basic_setup];
    if let Some(language) = language {
        extensions.push(language);
    }
    extensions.push(compartment_of(THEME.with(|theme| *theme), theme_for(false)));
    extensions.push(read_only(true));
    extensions.push(content_attributes(object::from_entries(vec![(
        "aria-label".to_string(),
        "Generated JavaScript".to_string(),
    )])));
    together(extensions)
}

fn theme_for(dark: bool) -> &'static Extension {
    if dark { one_dark } else { together(Vec::new()) }
}

/// A Rust file's editor state. Each file keeps its own, so its undo history
/// survives switching.
pub fn source_state(text: &str) -> &'static EditorState {
    create_state(&StateConfig {
        doc: text.to_string(),
        extensions: SOURCE.with(|e| *e),
    })
}

/// A generated file's, highlighted as JS; or rustc's diagnostics, as text.
pub fn output_state(text: &str, js: bool) -> &'static EditorState {
    let extensions = if js {
        JS_OUTPUT.with(|e| *e)
    } else {
        PLAIN_OUTPUT.with(|e| *e)
    };
    create_state(&StateConfig {
        doc: text.to_string(),
        extensions,
    })
}

pub fn text_of(state: &EditorState) -> String {
    text_string(doc(state))
}

/// A view of `state` in `parent`.
pub fn open_view(parent: &'static Element, state: &'static EditorState) -> &'static EditorView {
    new_editor(&EditorConfig { state, parent })
}

/// Show `state`, unless it's the one showing: setting it again would lose
/// the view's scroll position.
pub fn show(view: &EditorView, state: &EditorState) {
    if !object::is(editor_state(view), state) {
        set_editor_state(view, state);
    }
}

/// Follow the system's light or dark setting. A state keeps the theme it was
/// made with, so this runs after `show` too.
pub fn set_theme(view: &EditorView, dark: bool) {
    let theme = THEME.with(|theme| *theme);
    dispatch(
        view,
        &Transaction {
            effects: reconfigure(theme, theme_for(dark)),
        },
    );
}
