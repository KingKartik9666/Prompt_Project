//! Diffused background light that follows the cursor.
//!
//! One large, very soft radial gradient sits behind the content (`.glow` in app.css). It trails the
//! pointer through a critically damped spring, so it drifts like light rather than sticking to the
//! cursor, and only its `translate` changes (compositor-only, no layout or paint per frame). It fades
//! out when the pointer leaves the window, and stays off for people who ask for reduced motion.

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

/// Spring response in seconds: slower than the scroll driver, so the light lags softly behind the pointer.
const RESPONSE_S: f64 = 0.7;

#[derive(Default)]
struct Glow {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    tx: f64,
    ty: f64,
    placed: bool,
    running: bool,
    last: Option<f64>,
}

thread_local! {
    static GLOW: RefCell<Glow> = RefCell::default();
}

fn element() -> Option<HtmlElement> {
    web_sys::window()?.document()?.query_selector(".glow").ok().flatten()?.dyn_into().ok()
}

fn place(x: f64, y: f64) {
    if let Some(el) = element() {
        // The element is centred on its own origin by CSS margins, so translate = the light's centre.
        let _ = el.style().set_property("translate", &format!("{x:.1}px {y:.1}px"));
    }
}

fn set_visible(on: bool) {
    if let Some(el) = element() {
        let _ = if on { el.set_attribute("data-on", "") } else { el.remove_attribute("data-on") };
    }
}

fn next_frame() {
    let cb = Closure::once_into_js(tick);
    if let Some(w) = web_sys::window() {
        let _ = w.request_animation_frame(cb.unchecked_ref());
    }
}

fn tick(now: f64) {
    let again = GLOW.with_borrow_mut(|g| {
        let dt = g.last.map_or(1.0 / 60.0, |l| ((now - l) / 1000.0).clamp(0.0, 0.1));
        g.last = Some(now);
        let w = std::f64::consts::TAU / RESPONSE_S;
        let steps = (dt / 0.004).ceil().max(1.0);
        let h = dt / steps;
        for _ in 0..steps as usize {
            g.vx += (w * w * (g.tx - g.x) - 2.0 * w * g.vx) * h;
            g.vy += (w * w * (g.ty - g.y) - 2.0 * w * g.vy) * h;
            g.x += g.vx * h;
            g.y += g.vy * h;
        }
        let settled = (g.tx - g.x).abs() < 0.3 && (g.ty - g.y).abs() < 0.3 && g.vx.abs() < 1.0 && g.vy.abs() < 1.0;
        if settled {
            g.x = g.tx;
            g.y = g.ty;
        }
        place(g.x, g.y);
        g.running = !settled;
        g.running
    });
    if again {
        next_frame();
    }
}

/// Install the pointer listeners once, at start-up.
pub fn install() {
    if crate::bridge::reduced_motion() {
        return;
    }
    let Some(win) = web_sys::window() else { return };
    let on_move = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
        let (x, y) = (e.client_x() as f64, e.client_y() as f64);
        let start = GLOW.with_borrow_mut(|g| {
            g.tx = x;
            g.ty = y;
            if !g.placed {
                // first sighting: appear where the pointer is, then fade in
                g.placed = true;
                g.x = x;
                g.y = y;
            }
            let go = !g.running;
            if go {
                g.running = true;
                g.last = None;
            }
            go
        });
        set_visible(true);
        if start {
            next_frame();
        }
    });
    let _ = win.add_event_listener_with_callback("pointermove", on_move.as_ref().unchecked_ref());
    on_move.forget();
    // Leaving the window: let the light fade out where it is.
    if let Some(doc) = win.document() {
        let on_leave = Closure::<dyn FnMut()>::new(move || set_visible(false));
        let _ = doc.add_event_listener_with_callback("mouseleave", on_leave.as_ref().unchecked_ref());
        on_leave.forget();
    }
}
