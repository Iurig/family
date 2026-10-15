//! A lamp that blinks when a counter the audio thread keeps moves, and fades out after: for an
//! event the player should see, such as a detected attack. All timing and drawing run on the
//! editor thread.
//!
//! The stylesheet sets its look: `background-color` is the dark lamp, `color` the lit one, and a
//! `box-shadow` with a blur radius its glow while lit (see [`Glow`]).

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use nih_plug_vizia::vizia::{prelude::*, vg};

use crate::glow::Glow;

/// How long the lamp takes to fade completely after the counter moves.
const FADE: Duration = Duration::from_millis(150);

/// A lamp that polls the audio thread's counter at the editor's frame rate.
#[derive(Debug)]
pub struct Lamp {
    /// From 0 (dark) to 1 (just lit), as of the last frame.
    brightness: f32,
    glow: Glow,
}

/// The lamp's own model, holding the counter it polls, in its own module so the lint exception
/// for the code vizia's `Lens` derive generates stays scoped to it.
#[expect(
    clippy::expl_impl_clone_on_copy,
    reason = "vizia's `Lens` derive implements `Clone` by hand on the `Copy` lens types it generates"
)]
mod source {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;

    use nih_plug_vizia::vizia::prelude::*;

    #[derive(Lens)]
    pub(super) struct Counter {
        pub(super) count: Arc<AtomicU64>,
    }

    impl Model for Counter {}
}

/// The lamp's brightness as of this frame.
#[derive(Debug)]
struct Brightness(f32);

impl Lamp {
    /// A lamp that lights each time `counter` changes. The audio thread only stores to it, with
    /// relaxed ordering: the count is the whole message.
    pub fn new(cx: &mut Context, counter: Arc<AtomicU64>) -> Handle<'_, Self> {
        Self {
            brightness: 0.0,
            glow: Glow::default(),
        }
        .build(cx, |cx| {
            // Opening an editor must not flash for changes that happened while it was closed.
            let seen = Cell::new(counter.load(Ordering::Relaxed));
            let lit_at = Cell::new(None::<Instant>);
            // vizia's timers never fire under the baseview backend plugins run on, but it re-reads
            // every lens bound to a model on every frame. So the polling lives in a lens, like
            // nih_plug_vizia's peak meter, and the binding only fires while the brightness
            // changes: on a change and through its fade.
            let brightness = source::Counter::count.map(move |count| {
                let count = count.load(Ordering::Relaxed);
                if count != seen.get() {
                    seen.set(count);
                    lit_at.set(Some(Instant::now()));
                }
                lit_at.get().map_or(0.0, |at| {
                    (1.0 - at.elapsed().as_secs_f32() / FADE.as_secs_f32()).max(0.0)
                })
            });
            source::Counter { count: counter }.build(cx);
            Binding::new(cx, brightness, |cx, brightness| {
                let brightness = brightness.get(cx);
                cx.emit(Brightness(brightness));
            });
        })
    }
}

impl View for Lamp {
    fn element(&self) -> Option<&'static str> {
        Some("lamp")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|&Brightness(brightness), meta| {
            self.brightness = brightness;
            cx.needs_redraw();
            meta.consume();
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let brightness = self.brightness;
        let bounds = cx.bounds();
        let circle = |grow: f32| {
            let mut path = vg::Path::new();
            path.circle(
                bounds.x + bounds.w / 2.0,
                bounds.y + bounds.h / 2.0,
                bounds.w.min(bounds.h) / 2.0 + grow,
            );
            path
        };
        self.glow
            .draw(cx, canvas, brightness, |canvas, paint, spread| {
                canvas.fill_path(&circle(spread), paint);
            });
        let path = circle(0.0);
        let dark: vg::Color = cx.background_color().into();
        let lit: vg::Color = cx.font_color().into();
        let mix = |from: f32, to: f32| (to - from).mul_add(brightness, from);
        let color = vg::Color::rgbaf(
            mix(dark.r, lit.r),
            mix(dark.g, lit.g),
            mix(dark.b, lit.b),
            mix(dark.a, lit.a),
        );
        canvas.fill_path(&path, &vg::Paint::color(color));
    }
}
