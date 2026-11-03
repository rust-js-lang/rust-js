// A call whose arguments rustfmt writes a line each keeps its comments
// where they're written, as React's preinitModuleForSSR has them: a block
// comment ending the line of a link of its callee's chain, `.d() /*
// ReactDOMCurrentDispatcher */`, is before the next link, and one first on
// an argument's line, `/* preinitModuleScript */ href,`, is before the
// argument on its line. Each is before the argument it's written before,
// `href`, though the next argument is a call made after it, the map's
// `Option::map` (rd710).
pub struct Dispatcher {
    pub preinit_module_script: fn(&str, Option<u32>),
    pub request_form_reset: fn(&str),
}

pub struct Internals {
    pub dispatcher: Dispatcher,
}

impl Internals {
    pub fn d(&self) -> &Dispatcher {
        &self.dispatcher
    }
}

pub fn preinit_module_for_ssr(internals: &Internals, href: &str, nonce: Option<u32>) {
    (internals
        .d() /* ReactDOMCurrentDispatcher */
        .preinit_module_script)(
        /* preinitModuleScript */ href,
        nonce.map(|n| n + 1000000000),
    );
}

pub fn request_form_reset(internals: &Internals, form: &str) {
    (internals
        .d() /* ReactDOMCurrentDispatcher */
        .request_form_reset)(/* requestFormReset */ form);
}

pub fn dispatch_hint(dispatcher: &Dispatcher, href: &str, options: Option<u32>) {
    if href.len() > 3 {
        if href.len() > 4 {
            if href.len() > 5 {
                if href.len() > 6 {
                    (dispatcher.preinit_module_script)(
                        /* preinitModuleScript */ href,
                        options,
                    );
                }
            }
        }
    }
}

fn preinit(href: &str, nonce: Option<u32>) {
    println!("preinit {href} {nonce:?}");
}

fn reset(form: &str) {
    println!("reset {form}");
}

fn main() {
    let internals = Internals {
        dispatcher: Dispatcher {
            preinit_module_script: preinit,
            request_form_reset: reset,
        },
    };
    preinit_module_for_ssr(&internals, "/a.js", Some(7));
    preinit_module_for_ssr(&internals, "/b.js", None);
    request_form_reset(&internals, "form");
    dispatch_hint(&internals.dispatcher, "/module.js", Some(1));
    dispatch_hint(&internals.dispatcher, "/m", None);
}
