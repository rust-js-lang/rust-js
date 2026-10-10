//! [React DOM](https://react.dev/reference/react-dom): portals, resource
//! hints and forms here; putting React on a page in [`client`]; rendering to
//! HTML in [`server`] and [`prerender`] (`react-dom/static`).

use super::*;

/// [`createPortal`](https://react.dev/reference/react-dom/createPortal):
/// `children`, rendered into `container`, somewhere else in the DOM. Events
/// still bubble through the React tree. A [`ReactPortal`], as @types/react
/// types it: a child as it is, a component's result by `.element()`.
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#createPortal")]
pub fn create_portal(children: impl ReactNode, container: &webapi::Element) -> &'static ReactPortal {
    unreachable!()
}

/// `createPortal(children, container, key)`.
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#createPortal")]
pub fn create_portal_with_key(children: impl ReactNode, container: &webapi::Element, key: impl Key) -> &'static ReactPortal {
    unreachable!()
}

/// [`flushSync`](https://react.dev/reference/react-dom/flushSync): apply the
/// updates in `f` to the DOM before returning.
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#flushSync")]
pub fn flush_sync(f: impl FnOnce() + 'static) {
    unreachable!()
}

unsafe extern "Rust" {
    /// React DOM's version, like `"19.3.0"`.
    #[link_name = "react-dom#version"]
    pub safe static VERSION: &'static str;
}

// ── Resource hints ──────────────────────────────────────────────────────

/// [`prefetchDNS`](https://react.dev/reference/react-dom/prefetchDNS): look up
/// a server's IP address early.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#prefetchDNS")]
pub fn prefetch_dns(href: &str) {
    unreachable!()
}

/// [`preconnect`](https://react.dev/reference/react-dom/preconnect): connect
/// to a server early.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#preconnect")]
pub fn preconnect(href: &str) {
    unreachable!()
}

/// `preconnect(href, options)`.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#preconnect")]
pub fn preconnect_with(href: &str, options: PreconnectOptions) {
    unreachable!()
}

/// What a resource is, for [`preload`].
#[cfg(react = "19.0")]
pub enum PreloadAs {
    #[cfg_attr(rust_js, rust_js::name = "audio")]
    Audio,
    #[cfg_attr(rust_js, rust_js::name = "document")]
    Document,
    #[cfg_attr(rust_js, rust_js::name = "embed")]
    Embed,
    #[cfg_attr(rust_js, rust_js::name = "fetch")]
    Fetch,
    #[cfg_attr(rust_js, rust_js::name = "font")]
    Font,
    #[cfg_attr(rust_js, rust_js::name = "image")]
    Image,
    #[cfg_attr(rust_js, rust_js::name = "object")]
    Object,
    #[cfg_attr(rust_js, rust_js::name = "script")]
    Script,
    #[cfg_attr(rust_js, rust_js::name = "style")]
    Style,
    #[cfg_attr(rust_js, rust_js::name = "track")]
    Track,
    #[cfg_attr(rust_js, rust_js::name = "video")]
    Video,
    #[cfg_attr(rust_js, rust_js::name = "worker")]
    Worker,
}

/// Declares an options object: `new(..)` makes it with its required fields,
/// and a method per optional one.
macro_rules! options {
    ($(#[doc = $doc:literal])* $(#[cfg($cfg:meta)])? $name:ident($new:literal $(, $arg:ident: $argty:ty)*) { $($(#[doc = $fdoc:literal])* $(#[cfg($fcfg:meta)])? $field:ident: $ty:ty = $js:literal;)* }) => {
        $(#[doc = $doc])*
        $(#[cfg($cfg)])?
        pub struct $name(PhantomData<JsObject>);

        $(#[cfg($cfg)])?
        impl $name {
            #[cfg_attr(rust_js, rust_js::link_name = $new)]
            pub fn new($($arg: $argty),*) -> $name {
                unreachable!()
            }

            $(
                $(#[doc = $fdoc])*
                $(#[cfg($fcfg)])?
                #[cfg_attr(rust_js, rust_js::link_name = concat!("prop ", $js))]
                pub fn $field(self, value: $ty) -> $name {
                    unreachable!()
                }
            )*
        }
    };
}

options! {
    /// [`preconnect_with`]'s options.
    #[cfg(react = "19.0")]
    PreconnectOptions("{}") {
        /// `"anonymous"`, `"use-credentials"` or `""`.
        cross_origin: impl Value = "crossOrigin";
    }
}

options! {
    /// [`preload`]'s options: `PreloadOptions::new(PreloadAs::Font)`.
    #[cfg(react = "19.0")]
    PreloadOptions("{as}", r#as: PreloadAs) {
        /// `"anonymous"`, `"use-credentials"` or `""`; needed for `PreloadAs::Fetch`.
        cross_origin: impl Value = "crossOrigin";
        referrer_policy: impl Value = "referrerPolicy";
        integrity: impl Value = "integrity";
        /// Its MIME type.
        r#type: impl Value = "type";
        nonce: impl Value = "nonce";
        /// `"auto"`, `"high"` or `"low"`.
        fetch_priority: impl Value = "fetchPriority";
        /// For `PreloadAs::Image`.
        image_src_set: impl Value = "imageSrcSet";
        image_sizes: impl Value = "imageSizes";
        /// A media query the resource is for, `"(min-width: 800px)"`.
        media: impl Value = "media";
    }
}

/// [`preload`](https://react.dev/reference/react-dom/preload): fetch a
/// resource you'll need soon.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#preload")]
pub fn preload(href: &str, options: PreloadOptions) {
    unreachable!()
}

/// What a module is, for [`preload_module_with`]: a fetch's
/// `RequestDestination`, `"script"` where none is given.
#[cfg(react = "19.0")]
pub enum PreloadModuleAs {
    #[cfg_attr(rust_js, rust_js::name = "")]
    Empty,
    #[cfg_attr(rust_js, rust_js::name = "audio")]
    Audio,
    #[cfg_attr(rust_js, rust_js::name = "audioworklet")]
    AudioWorklet,
    #[cfg_attr(rust_js, rust_js::name = "document")]
    Document,
    #[cfg_attr(rust_js, rust_js::name = "embed")]
    Embed,
    #[cfg_attr(rust_js, rust_js::name = "font")]
    Font,
    #[cfg_attr(rust_js, rust_js::name = "frame")]
    Frame,
    #[cfg_attr(rust_js, rust_js::name = "iframe")]
    IFrame,
    #[cfg_attr(rust_js, rust_js::name = "image")]
    Image,
    #[cfg_attr(rust_js, rust_js::name = "manifest")]
    Manifest,
    #[cfg_attr(rust_js, rust_js::name = "object")]
    Object,
    #[cfg_attr(rust_js, rust_js::name = "paintworklet")]
    PaintWorklet,
    #[cfg_attr(rust_js, rust_js::name = "report")]
    Report,
    #[cfg_attr(rust_js, rust_js::name = "script")]
    Script,
    #[cfg_attr(rust_js, rust_js::name = "sharedworker")]
    SharedWorker,
    #[cfg_attr(rust_js, rust_js::name = "style")]
    Style,
    #[cfg_attr(rust_js, rust_js::name = "track")]
    Track,
    #[cfg_attr(rust_js, rust_js::name = "video")]
    Video,
    #[cfg_attr(rust_js, rust_js::name = "worker")]
    Worker,
    #[cfg_attr(rust_js, rust_js::name = "xslt")]
    Xslt,
}

options! {
    /// [`preload_module_with`]'s options: `PreloadModuleOptions::new(PreloadModuleAs::Script)`.
    #[cfg(react = "19.0")]
    PreloadModuleOptions("{as}", r#as: PreloadModuleAs) {
        cross_origin: impl Value = "crossOrigin";
        integrity: impl Value = "integrity";
        nonce: impl Value = "nonce";
    }
}

/// [`preloadModule`](https://react.dev/reference/react-dom/preloadModule):
/// fetch an ES module you'll need soon.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#preloadModule")]
pub fn preload_module(href: &str) {
    unreachable!()
}

/// `preloadModule(href, options)`.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#preloadModule")]
pub fn preload_module_with(href: &str, options: PreloadModuleOptions) {
    unreachable!()
}

/// What [`preinit`] loads: a script or a stylesheet.
#[cfg(react = "19.0")]
pub enum PreinitAs {
    #[cfg_attr(rust_js, rust_js::name = "script")]
    Script,
    #[cfg_attr(rust_js, rust_js::name = "style")]
    Style,
}

options! {
    /// [`preinit`]'s options: `PreinitOptions::new(PreinitAs::Style).precedence("high")`.
    #[cfg(react = "19.0")]
    PreinitOptions("{as}", r#as: PreinitAs) {
        /// A stylesheet's place among others: `"reset"`, `"low"`, `"medium"`
        /// or `"high"`. A stylesheet needs one.
        precedence: impl Value = "precedence";
        cross_origin: impl Value = "crossOrigin";
        integrity: impl Value = "integrity";
        nonce: impl Value = "nonce";
        fetch_priority: impl Value = "fetchPriority";
    }
}

/// [`preinit`](https://react.dev/reference/react-dom/preinit): fetch and run
/// a script, or insert a stylesheet, early.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#preinit")]
pub fn preinit(href: &str, options: PreinitOptions) {
    unreachable!()
}

/// What [`preinit_module_with`] loads: a script, as for now only one is.
#[cfg(react = "19.0")]
pub enum PreinitModuleAs {
    #[cfg_attr(rust_js, rust_js::name = "script")]
    Script,
}

options! {
    /// [`preinit_module_with`]'s options.
    #[cfg(react = "19.0")]
    PreinitModuleOptions("{}") {
        /// `PreinitModuleAs::Script` where none is given.
        r#as: PreinitModuleAs = "as";
        cross_origin: impl Value = "crossOrigin";
        integrity: impl Value = "integrity";
        nonce: impl Value = "nonce";
    }
}

/// [`preinitModule`](https://react.dev/reference/react-dom/preinitModule):
/// fetch and run an ES module early.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#preinitModule")]
pub fn preinit_module(href: &str) {
    unreachable!()
}

/// `preinitModule(href, options)`.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#preinitModule")]
pub fn preinit_module_with(href: &str, options: PreinitModuleOptions) {
    unreachable!()
}

// ── Forms ───────────────────────────────────────────────────────────────

/// [`useFormStatus`](https://react.dev/reference/react-dom/hooks/useFormStatus):
/// the last submission of the `<form>` this component is in.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#useFormStatus")]
pub fn use_form_status() -> FormStatus {
    unreachable!()
}

/// What [`use_form_status`] gives: the submission, while the form is
/// being submitted, `{ pending: true, .. }`, or `{ pending: false, .. }`.
/// @types/react-dom's `FormStatusPending | FormStatusNotPending`.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::tag = "pending")]
pub enum FormStatus {
    #[cfg_attr(rust_js, rust_js::name = true)]
    Pending {
        /// What it's submitting.
        data: &'static webapi::FormData,
        /// `"get"` or `"post"`.
        method: String,
        action: FormStatusAction,
    },
    #[cfg_attr(rust_js, rust_js::name = false)]
    NotPending,
}

/// What a pending form is submitted to: the `<form>`'s `action`, a URL or
/// a function.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum FormStatusAction {
    Url(String),
    Function(&'static dyn Fn(&webapi::FormData) -> Option<Promise<()>>),
}

/// [`requestFormReset`](https://react.dev/reference/react-dom/requestFormReset):
/// reset `form` once the current Transition is done.
#[cfg(react = "19.0")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#requestFormReset")]
pub fn request_form_reset(form: &webapi::HTMLFormElement) {
    unreachable!()
}

/// What [`browser`] marks: content that only renders in the browser.
#[cfg(react = "19.3")]
pub struct BrowserUsable(PhantomData<JsObject>);

#[cfg(react = "19.3")]
impl Usable for BrowserUsable {
    type Output = ();
}

/// [`browser`](https://react.dev/reference/react-dom/browser): `use_(browser(reason))`
/// renders a component only in the browser. On the server, the nearest
/// [`suspense`]'s fallback shows instead, and `reason` says why.
#[cfg(react = "19.3")]
#[cfg_attr(rust_js, rust_js::link_name = "react-dom#browser")]
pub fn browser(reason: &str) -> BrowserUsable {
    unreachable!()
}

// ── react-dom/client ────────────────────────────────────────────────────

/// [`react-dom/client`](https://react.dev/reference/react-dom/client): putting
/// React on a page.
pub mod client {
    use super::*;

    /// Where React renders, made by [`create_root`] or [`hydrate_root`].
    pub struct Root(PhantomData<JsObject>);

    impl Root {
        /// Show `children` in the root's element, replacing what was there.
        #[cfg_attr(rust_js, rust_js::link_name = "render")]
        pub fn render(&self, children: impl ReactNode) {
            unreachable!()
        }

        /// Take React off the element.
        #[cfg_attr(rust_js, rust_js::link_name = "unmount")]
        pub fn unmount(&self) {
            unreachable!()
        }
    }

    /// What [`create_root`] renders into: an element, a document fragment
    /// or a document. `M` tells the three apart, as no one type is all.
    pub trait Container<M> {}

    pub struct ElementContainer;
    pub struct DocumentFragmentContainer;
    pub struct DocumentContainer;

    impl<T: webapi::IsA<webapi::Element>> Container<ElementContainer> for &T {}
    impl Container<DocumentFragmentContainer> for &webapi::DocumentFragment {}
    impl Container<DocumentContainer> for &webapi::Document {}

    /// What [`hydrate_root`] takes over: an element, or the whole document.
    pub trait HydrationContainer<M> {}

    impl<T: webapi::IsA<webapi::Element>> HydrationContainer<ElementContainer> for &T {}
    impl HydrationContainer<DocumentContainer> for &webapi::Document {}

    options! {
        /// A root's options: what to call on errors, and a prefix for [`use_id`]'s ids.
        RootOptions("{}") {
            /// An error an error boundary caught.
            on_caught_error: impl Fn(&Error, &ErrorInfo) + 'static = "onCaughtError";
            /// An error nothing caught.
            on_uncaught_error: impl Fn(&Error, &ErrorInfo) + 'static = "onUncaughtError";
            /// An error React recovered from, like a hydration mismatch.
            on_recoverable_error: impl Fn(&Error, &ErrorInfo) + 'static = "onRecoverableError";
            identifier_prefix: impl Value = "identifierPrefix";
        }
    }

    options! {
        /// [`hydrate_root_with`]'s options: a root's, and the state of the
        /// form the server's action rendered.
        HydrationOptions("{}") {
            #[cfg(react = "19.0")]
            form_state: Option<&'static ReactFormState> = "formState";
            identifier_prefix: impl Value = "identifierPrefix";
            on_caught_error: impl Fn(&Error, &ErrorInfo) + 'static = "onCaughtError";
            on_uncaught_error: impl Fn(&Error, &ErrorInfo) + 'static = "onUncaughtError";
            on_recoverable_error: impl Fn(&Error, &ErrorInfo) + 'static = "onRecoverableError";
        }
    }

    /// The state of a form a server action handled, which the server's
    /// framework gives, to render and hydrate the page as the action left it.
    #[cfg(react = "19.0")]
    pub struct ReactFormState(PhantomData<JsObject>);

    /// [`createRoot`](https://react.dev/reference/react-dom/client/createRoot).
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/client#createRoot")]
    pub fn create_root<M>(container: impl Container<M>) -> &'static Root {
        unreachable!()
    }

    /// `createRoot(container, options)`.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/client#createRoot")]
    pub fn create_root_with<M>(container: impl Container<M>, options: RootOptions) -> &'static Root {
        unreachable!()
    }

    /// [`hydrateRoot`](https://react.dev/reference/react-dom/client/hydrateRoot):
    /// attach React to HTML the server rendered from `children`.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/client#hydrateRoot")]
    pub fn hydrate_root<M>(container: impl HydrationContainer<M>, children: impl ReactNode) -> &'static Root {
        unreachable!()
    }

    /// `hydrateRoot(container, children, options)`.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/client#hydrateRoot")]
    pub fn hydrate_root_with<M>(
        container: impl HydrationContainer<M>,
        children: impl ReactNode,
        options: HydrationOptions,
    ) -> &'static Root {
        unreachable!()
    }
}

// ── react-dom/server ────────────────────────────────────────────────────

/// [`react-dom/server`](https://react.dev/reference/react-dom/server):
/// rendering to HTML. Web streams (browsers, Deno, Bun, edge runtimes), or
/// Node's streams, or a string.
pub mod server {
    #[cfg(react = "19.0")]
    use super::client::ReactFormState;
    use super::*;
    #[cfg(react = "19.0")]
    pub use super::prerender::ResumeOptions;
    #[cfg(react = "19.0")]
    use js::Dict;

    options! {
        /// The options of [`render_to_string_with`] and [`render_to_static_markup_with`].
        ServerOptions("{}") {
            identifier_prefix: impl Value = "identifierPrefix";
        }
    }

    options! {
        /// A script that hydrates the page, its `src`, with how to check it.
        BootstrapScriptDescriptor("{src}", src: &str) {
            integrity: impl Value = "integrity";
            cross_origin: impl Value = "crossOrigin";
        }
    }

    /// A script of `bootstrap_scripts` or `bootstrap_modules`: its `src`,
    /// or its [`BootstrapScriptDescriptor`].
    #[cfg_attr(rust_js, rust_js::untagged)]
    pub enum BootstrapScript {
        Src(&'static str),
        Descriptor(BootstrapScriptDescriptor),
    }

    /// The nonce of the page's inline scripts and styles, for a Content
    /// Security Policy: one for both, or each its own.
    #[cfg_attr(rust_js, rust_js::untagged)]
    pub enum NonceOption {
        Both(&'static str),
        Each(Nonces),
    }

    options! {
        /// A [`NonceOption`] of scripts' and styles' own.
        Nonces("{}") {
            script: impl Value = "script";
            style: impl Value = "style";
        }
    }

    options! {
        /// An import map, for the page's modules: each specifier's URL,
        /// each URL's integrity, and the specifiers of a scope.
        #[cfg(react = "19.0")]
        ReactImportMap("{}") {
            imports: &'static Dict<String> = "imports";
            integrity: &'static Dict<String> = "integrity";
            scopes: &'static Dict<&'static Dict<String>> = "scopes";
        }
    }

    /// [`renderToString`](https://react.dev/reference/react-dom/server/renderToString):
    /// HTML that [`client::hydrate_root`] can take over. A suspending
    /// component gets its fallback.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#renderToString")]
    pub fn render_to_string(children: impl ReactNode) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#renderToString")]
    pub fn render_to_string_with(children: impl ReactNode, options: ServerOptions) -> String {
        unreachable!()
    }

    /// [`renderToStaticMarkup`](https://react.dev/reference/react-dom/server/renderToStaticMarkup):
    /// HTML that won't be hydrated.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#renderToStaticMarkup")]
    pub fn render_to_static_markup(children: impl ReactNode) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#renderToStaticMarkup")]
    pub fn render_to_static_markup_with(children: impl ReactNode, options: ServerOptions) -> String {
        unreachable!()
    }

    options! {
        /// How to stream a page as a Web stream: the scripts that hydrate
        /// it, and what to call on errors.
        RenderToReadableStreamOptions("{}") {
            identifier_prefix: impl Value = "identifierPrefix";
            #[cfg(react = "19.0")]
            import_map: ReactImportMap = "importMap";
            namespace_uri: impl Value = "namespaceURI";
            nonce: NonceOption = "nonce";
            bootstrap_script_content: impl Value = "bootstrapScriptContent";
            bootstrap_scripts: Vec<BootstrapScript> = "bootstrapScripts";
            bootstrap_modules: Vec<BootstrapScript> = "bootstrapModules";
            #[cfg(react = "19.0")]
            max_headers_length: u32 = "maxHeadersLength";
            progressive_chunk_size: u32 = "progressiveChunkSize";
            /// Stop rendering, and leave the rest to the client.
            signal: &'static webapi::AbortSignal = "signal";
            /// Called when [`browser`] stops a component rendering here.
            #[cfg(react = "19.3")]
            on_browser_bailout: impl Fn(&Error, &ErrorInfo) + 'static = "onBrowserBailout";
            /// Called with each error on the server, recovered from or not:
            /// what it gives is the error's digest, sent to the client.
            on_error: impl Fn(&Error, &ErrorInfo) -> Option<String> + 'static = "onError";
            /// Called with the `Link` headers for the page's preloads.
            #[cfg(react = "19.0")]
            on_headers: impl Fn(&webapi::Headers) + 'static = "onHeaders";
            /// The state of the form a server action handled.
            #[cfg(react = "19.0")]
            form_state: Option<&'static ReactFormState> = "formState";
        }
    }

    /// What [`render_to_readable_stream`] gives: a stream of the page's HTML.
    pub struct ReactDOMServerReadableStream(PhantomData<JsObject>);

    impl Deref for ReactDOMServerReadableStream {
        type Target = webapi::ReadableStream;

        fn deref(&self) -> &webapi::ReadableStream {
            // Never runs: rust-js compiles this `Deref` to the object itself.
            unsafe { &*(self as *const Self as *const webapi::ReadableStream) }
        }
    }

    impl ReactDOMServerReadableStream {
        /// Resolves once everything, suspended parts too, is rendered.
        #[cfg_attr(rust_js, rust_js::link_name = "get allReady")]
        pub fn all_ready(&self) -> Promise<()> {
            unreachable!()
        }
    }

    /// [`renderToReadableStream`](https://react.dev/reference/react-dom/server/renderToReadableStream):
    /// a Web stream of the page's HTML, sent as it's ready. It rejects if the
    /// page's shell fails.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#renderToReadableStream")]
    pub fn render_to_readable_stream(children: impl ReactNode) -> Promise<&'static ReactDOMServerReadableStream> {
        unreachable!()
    }

    /// `renderToReadableStream(children, options)`.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#renderToReadableStream")]
    pub fn render_to_readable_stream_with(
        children: impl ReactNode,
        options: RenderToReadableStreamOptions,
    ) -> Promise<&'static ReactDOMServerReadableStream> {
        unreachable!()
    }

    /// [`resume`](https://react.dev/reference/react-dom/server/resume): finish,
    /// as a Web stream, a page [`prerender`] postponed.
    #[cfg(react = "19.2")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#resume")]
    pub fn resume(children: impl ReactNode, postponed: &PostponedState) -> Promise<&'static ReactDOMServerReadableStream> {
        unreachable!()
    }

    /// `resume(children, postponed, options)`.
    #[cfg(react = "19.2")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#resume")]
    pub fn resume_with(
        children: impl ReactNode,
        postponed: &PostponedState,
        options: ResumeOptions,
    ) -> Promise<&'static ReactDOMServerReadableStream> {
        unreachable!()
    }

    /// What [`RenderToPipeableStreamOptions::on_headers`] is given: the
    /// `Link` header of the page's preloads.
    #[cfg(react = "19.0")]
    pub struct HeadersDescriptor(PhantomData<JsObject>);

    #[cfg(react = "19.0")]
    impl HeadersDescriptor {
        #[cfg_attr(rust_js, rust_js::link_name = "get Link")]
        pub fn link(&self) -> Option<String> {
            unreachable!()
        }
    }

    options! {
        /// How to stream a page to Node's streams: as
        /// [`RenderToReadableStreamOptions`], with when the page's shell and
        /// all of it are ready.
        RenderToPipeableStreamOptions("{}") {
            identifier_prefix: impl Value = "identifierPrefix";
            namespace_uri: impl Value = "namespaceURI";
            nonce: NonceOption = "nonce";
            bootstrap_script_content: impl Value = "bootstrapScriptContent";
            bootstrap_scripts: Vec<BootstrapScript> = "bootstrapScripts";
            bootstrap_modules: Vec<BootstrapScript> = "bootstrapModules";
            #[cfg(react = "19.0")]
            max_headers_length: u32 = "maxHeadersLength";
            #[cfg(react = "19.0")]
            import_map: ReactImportMap = "importMap";
            progressive_chunk_size: u32 = "progressiveChunkSize";
            #[cfg(react = "19.0")]
            on_headers: impl Fn(&HeadersDescriptor) + 'static = "onHeaders";
            /// When the shell is ready: time to [`pipe`](PipeableStream::pipe).
            on_shell_ready: impl Fn() + 'static = "onShellReady";
            on_shell_error: impl Fn(&Error) + 'static = "onShellError";
            /// When everything is ready, for crawlers and static pages.
            on_all_ready: impl Fn() + 'static = "onAllReady";
            #[cfg(react = "19.3")]
            on_browser_bailout: impl Fn(&Error, &ErrorInfo) + 'static = "onBrowserBailout";
            on_error: impl Fn(&Error, &ErrorInfo) -> Option<String> + 'static = "onError";
            #[cfg(react = "19.0")]
            form_state: Option<&'static ReactFormState> = "formState";
        }
    }

    /// What [`render_to_pipeable_stream`] gives.
    pub struct PipeableStream(PhantomData<JsObject>);

    impl PipeableStream {
        /// Send the HTML to a Node `Writable`, like an HTTP response, and
        /// give it back.
        #[cfg_attr(rust_js, rust_js::link_name = "pipe")]
        pub fn pipe<W>(&self, destination: &W) -> &'static W {
            unreachable!()
        }

        /// Stop rendering, and leave the rest to the client.
        #[cfg_attr(rust_js, rust_js::link_name = "abort")]
        pub fn abort(&self) {
            unreachable!()
        }

        /// `abort(reason)`: stop, with why, which the error callbacks get.
        #[cfg_attr(rust_js, rust_js::link_name = "abort")]
        pub fn abort_with<R>(&self, reason: R) {
            unreachable!()
        }
    }

    /// [`renderToPipeableStream`](https://react.dev/reference/react-dom/server/renderToPipeableStream):
    /// the page's HTML, for Node's streams.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#renderToPipeableStream")]
    pub fn render_to_pipeable_stream(children: impl ReactNode) -> &'static PipeableStream {
        unreachable!()
    }

    /// `renderToPipeableStream(children, options)`.
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#renderToPipeableStream")]
    pub fn render_to_pipeable_stream_with(children: impl ReactNode, options: RenderToPipeableStreamOptions) -> &'static PipeableStream {
        unreachable!()
    }

    options! {
        /// [`resume_to_pipeable_stream_with`]'s options.
        #[cfg(react = "19.2")]
        ResumeToPipeableStreamOptions("{}") {
            nonce: NonceOption = "nonce";
            on_shell_ready: impl Fn() + 'static = "onShellReady";
            on_shell_error: impl Fn(&Error) + 'static = "onShellError";
            on_all_ready: impl Fn() + 'static = "onAllReady";
            #[cfg(react = "19.3")]
            on_browser_bailout: impl Fn(&Error, &ErrorInfo) + 'static = "onBrowserBailout";
            on_error: impl Fn(&Error, &ErrorInfo) -> Option<String> + 'static = "onError";
        }
    }

    /// [`resumeToPipeableStream`](https://react.dev/reference/react-dom/server/resumeToPipeableStream):
    /// finish, for Node's streams, a page [`prerender`] postponed.
    #[cfg(react = "19.2")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#resumeToPipeableStream")]
    pub fn resume_to_pipeable_stream(children: impl ReactNode, postponed: &PostponedState) -> Promise<&'static PipeableStream> {
        unreachable!()
    }

    /// `resumeToPipeableStream(children, postponed, options)`.
    #[cfg(react = "19.2")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/server#resumeToPipeableStream")]
    pub fn resume_to_pipeable_stream_with(
        children: impl ReactNode,
        postponed: &PostponedState,
        options: ResumeToPipeableStreamOptions,
    ) -> Promise<&'static PipeableStream> {
        unreachable!()
    }

    /// What a prerender left for later: JSON, to keep until the request.
    pub struct PostponedState(PhantomData<JsObject>);
}

// ── react-dom/static ────────────────────────────────────────────────────

/// [`react-dom/static`](https://react.dev/reference/react-dom/static):
/// rendering a whole page ahead of time, waiting for all of it.
pub mod prerender {
    #[cfg(react = "19.0")]
    use super::server::PostponedState;
    pub use super::server::{BootstrapScript, BootstrapScriptDescriptor, NonceOption, Nonces};
    #[cfg(react = "19.0")]
    pub use super::server::ReactImportMap;
    #[cfg(react = "19.0")]
    use super::*;

    options! {
        /// [`prerender_with`]'s and [`prerender_to_node_stream_with`]'s options.
        #[cfg(react = "19.0")]
        PrerenderOptions("{}") {
            bootstrap_script_content: impl Value = "bootstrapScriptContent";
            bootstrap_scripts: Vec<BootstrapScript> = "bootstrapScripts";
            bootstrap_modules: Vec<BootstrapScript> = "bootstrapModules";
            max_headers_length: u32 = "maxHeadersLength";
            identifier_prefix: impl Value = "identifierPrefix";
            import_map: ReactImportMap = "importMap";
            namespace_uri: impl Value = "namespaceURI";
            #[cfg(react = "19.3")]
            on_browser_bailout: impl Fn(&Error, &ErrorInfo) + 'static = "onBrowserBailout";
            on_error: impl Fn(&Error, &ErrorInfo) -> Option<String> + 'static = "onError";
            on_headers: impl Fn(&webapi::Headers) + 'static = "onHeaders";
            progressive_chunk_size: u32 = "progressiveChunkSize";
            /// Stop prerendering: what's left is postponed.
            signal: &'static webapi::AbortSignal = "signal";
        }
    }

    options! {
        /// [`server::resume_with`]'s options, and, but its nonce,
        /// [`resume_and_prerender_with`]'s.
        #[cfg(react = "19.0")]
        ResumeOptions("{}") {
            nonce: NonceOption = "nonce";
            signal: &'static webapi::AbortSignal = "signal";
            #[cfg(react = "19.3")]
            on_browser_bailout: impl Fn(&Error, &ErrorInfo) + 'static = "onBrowserBailout";
            on_error: impl Fn(&Error) -> Option<String> + 'static = "onError";
        }
    }

    /// What [`prerender`] gives: the HTML, and what it left for later.
    #[cfg(react = "19.0")]
    pub struct PrerenderResult(PhantomData<JsObject>);

    #[cfg(react = "19.0")]
    impl PrerenderResult {
        /// The HTML, a Web stream.
        #[cfg_attr(rust_js, rust_js::link_name = "get prelude")]
        pub fn prelude(&self) -> &'static webapi::ReadableStream {
            unreachable!()
        }

        /// What [`server::resume`] finishes, if anything was postponed.
        #[cfg_attr(rust_js, rust_js::link_name = "get postponed")]
        pub fn postponed(&self) -> Option<&'static PostponedState> {
            unreachable!()
        }
    }

    /// What [`prerender_to_node_stream`] gives: the HTML, and what it left
    /// for later.
    #[cfg(react = "19.0")]
    pub struct PrerenderToNodeStreamResult(PhantomData<JsObject>);

    #[cfg(react = "19.0")]
    impl PrerenderToNodeStreamResult {
        /// The HTML, a Node `Readable`, as the type that binds it, which
        /// this crate doesn't have.
        #[cfg_attr(rust_js, rust_js::link_name = "get prelude")]
        pub fn prelude<R>(&self) -> &'static R {
            unreachable!()
        }

        /// What [`server::resume`] finishes, if anything was postponed.
        #[cfg_attr(rust_js, rust_js::link_name = "get postponed")]
        pub fn postponed(&self) -> Option<&'static PostponedState> {
            unreachable!()
        }
    }

    /// [`prerender`](https://react.dev/reference/react-dom/static/prerender),
    /// with Web streams.
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/static#prerender")]
    pub fn prerender(children: impl ReactNode) -> Promise<&'static PrerenderResult> {
        unreachable!()
    }

    /// `prerender(children, options)`.
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/static#prerender")]
    pub fn prerender_with(children: impl ReactNode, options: PrerenderOptions) -> Promise<&'static PrerenderResult> {
        unreachable!()
    }

    /// [`prerenderToNodeStream`](https://react.dev/reference/react-dom/static/prerenderToNodeStream),
    /// with Node's streams.
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/static#prerenderToNodeStream")]
    pub fn prerender_to_node_stream(children: impl ReactNode) -> Promise<&'static PrerenderToNodeStreamResult> {
        unreachable!()
    }

    /// `prerenderToNodeStream(children, options)`.
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/static#prerenderToNodeStream")]
    pub fn prerender_to_node_stream_with(
        children: impl ReactNode,
        options: PrerenderOptions,
    ) -> Promise<&'static PrerenderToNodeStreamResult> {
        unreachable!()
    }

    /// [`resumeAndPrerender`](https://react.dev/reference/react-dom/static/resumeAndPrerender):
    /// go on with a prerender that was postponed.
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/static#resumeAndPrerender")]
    pub fn resume_and_prerender(children: impl ReactNode, postponed: Option<&PostponedState>) -> Promise<&'static PrerenderResult> {
        unreachable!()
    }

    /// `resumeAndPrerender(children, postponed, options)`, options without a nonce.
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/static#resumeAndPrerender")]
    pub fn resume_and_prerender_with(
        children: impl ReactNode,
        postponed: Option<&PostponedState>,
        options: ResumeOptions,
    ) -> Promise<&'static PrerenderResult> {
        unreachable!()
    }

    /// [`resumeAndPrerenderToNodeStream`](https://react.dev/reference/react-dom/static/resumeAndPrerenderToNodeStream).
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/static#resumeAndPrerenderToNodeStream")]
    pub fn resume_and_prerender_to_node_stream(
        children: impl ReactNode,
        postponed: Option<&PostponedState>,
    ) -> Promise<&'static PrerenderToNodeStreamResult> {
        unreachable!()
    }

    /// `resumeAndPrerenderToNodeStream(children, postponed, options)`.
    #[cfg(react = "19.0")]
    #[cfg_attr(rust_js, rust_js::link_name = "react-dom/static#resumeAndPrerenderToNodeStream")]
    pub fn resume_and_prerender_to_node_stream_with(
        children: impl ReactNode,
        postponed: Option<&PostponedState>,
        options: ResumeOptions,
    ) -> Promise<&'static PrerenderToNodeStreamResult> {
        unreachable!()
    }
}
