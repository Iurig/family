//! The dial: a 270° arc showing a parameter's value, with a dot at its end.
//!
//! `nih_plug_vizia` has no dial widget, so this is a custom view. A plugin sizes it in its
//! stylesheet, from a large main control to a small secondary one.
//!
//! Dragging up or down changes the value (Shift for fine adjustment), double-click resets it to
//! its default and the mouse wheel steps it. Every change goes through the host's parameter setter
//! inside a begin/end gesture, so hosts record it as automation. The arc reads the parameter on
//! every draw, so it follows host automation too. There's no typing a value in.
//!
//! The stylesheet sets its look: `background-color` is the track, `border-color` the arc up to the
//! value, `outline-color` the dot and `border-width` the arc's thickness. A `box-shadow` with a
//! blur radius gives the arc a glow in its colour (see [`Glow`]).

use std::f32::consts::PI;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::{prelude::*, vg};
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt as _;

use crate::glow::Glow;

/// How far a drag of one logical pixel moves the normalized value: a 200 px drag covers the
/// whole range.
const DRAG_PER_PIXEL: f32 = 0.005;

/// The same for a Shift-drag: ten times finer, so 2000 px cover the whole range.
const FINE_DRAG_PER_PIXEL: f32 = 0.0005;

/// How far one wheel notch moves the normalized value: 5 %.
const WHEEL_STEP: f32 = 0.05;

/// The same with Shift held: 1 %.
const FINE_WHEEL_STEP: f32 = 0.01;

/// Where the arc starts, at the minimum: bottom left, in radians clockwise from the right on a
/// screen whose y axis points down.
const START_ANGLE: f32 = 0.75 * PI;

/// How far the arc sweeps from the minimum to the maximum: 270°.
const SWEEP: f32 = 1.5 * PI;

/// The end dot's radius as a fraction of the arc's thickness.
const DOT_RADIUS: f32 = 0.32;

/// The normalized value after dragging `rise` logical pixels upward from `value`.
///
/// A negative rise is a drag downward. Clamped to the parameter's range, so dragging back after
/// overshooting an end moves the value again straight away.
#[must_use]
pub const fn dragged(value: f32, rise: f32, fine: bool) -> f32 {
    let per_pixel = if fine {
        FINE_DRAG_PER_PIXEL
    } else {
        DRAG_PER_PIXEL
    };
    rise.mul_add(per_pixel, value).clamp(0.0, 1.0)
}

/// The normalized value after `notches` wheel notches from `value`, upward positive.
#[must_use]
pub const fn wheeled(value: f32, notches: i32, fine: bool) -> f32 {
    let step = if fine { FINE_WHEEL_STEP } else { WHEEL_STEP };
    #[expect(
        clippy::cast_precision_loss,
        reason = "a wheel event scrolls a handful of notches, far below f32's exact integer range"
    )]
    let notches = notches as f32;
    notches.mul_add(step, value).clamp(0.0, 1.0)
}

/// The angle of the arc's end for a normalized `value`, in radians clockwise from the right.
#[must_use]
pub const fn angle(value: f32) -> f32 {
    value.clamp(0.0, 1.0).mul_add(SWEEP, START_ANGLE)
}

/// The last value a dial sent to the host.
///
/// The CLAP wrapper only applies a change once the host has taken it, so for a moment the
/// parameter still reads its old value. Until it moves off that value, the dial builds on what it
/// sent instead, so quick drag moves and wheel notches aren't lost.
#[derive(Debug, Clone, Copy)]
struct Sent {
    /// The normalized value sent.
    value: f32,
    /// The parameter's normalized value when it was sent.
    over: f32,
}

impl Sent {
    /// The normalized value to build on when the parameter reads `param`: what was last `sent`
    /// while the parameter hasn't moved off the value it had then, else `param`, which also
    /// follows host automation.
    const fn building_on(sent: Option<Self>, param: f32) -> f32 {
        match sent {
            Some(sent) if sent.over.to_bits() == param.to_bits() => sent.value,
            _ => param,
        }
    }
}

/// A dial for one parameter. Its children are laid out inside its bounds, over the arc; they
/// should be made non-hoverable so the dial gets the mouse.
pub struct Dial {
    param_base: ParamWidgetBase,
    /// While a drag holds a gesture open: the cursor's height at the last drag event, in physical
    /// pixels.
    drag_from: Option<f32>,
    /// The last value sent, if any.
    sent: Option<Sent>,
    /// Wheel movement not yet turned into a step, since trackpads scroll in fractions of a notch.
    scrolled: f32,
    /// The value arc's glow.
    glow: Glow,
}

impl std::fmt::Debug for Dial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dial")
            .field("drag_from", &self.drag_from)
            .field("scrolled", &self.scrolled)
            .finish_non_exhaustive()
    }
}

impl Dial {
    /// A dial for the parameter `params_to_param` picks out of `params`, with `content` built
    /// inside it.
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
        content: impl FnOnce(&mut Context),
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = Params> + Copy,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        let value = ParamWidgetBase::make_lens(params, params_to_param, |param| {
            param.unmodulated_normalized_value()
        });
        Self {
            param_base: ParamWidgetBase::new(cx, params, params_to_param),
            drag_from: None,
            sent: None,
            scrolled: 0.0,
            glow: Glow::default(),
        }
        .build(cx, content)
        // The draw reads the value itself; the binding only asks for a redraw when automation or
        // another view changes it.
        .bind(value, |mut handle, _| handle.needs_redraw())
    }

    /// Sets the parameter to `normalized`: inside the drag's gesture while a drag holds one, or
    /// as one whole gesture of its own.
    fn set(&mut self, cx: &mut EventContext, normalized: f32) {
        self.sent = Some(Sent {
            value: normalized,
            over: self.param_base.unmodulated_normalized_value(),
        });
        let own_gesture = self.drag_from.is_none();
        if own_gesture {
            self.param_base.begin_set_parameter(cx);
        }
        self.param_base.set_normalized_value(cx, normalized);
        if own_gesture {
            self.param_base.end_set_parameter(cx);
        }
        // The arc shows the sent value straight away, before the parameter catches up.
        cx.needs_redraw();
    }

    /// The normalized value to show and build on: the last one sent while the parameter hasn't
    /// caught up with it, else the parameter's own, which also follows host automation.
    fn current(&self) -> f32 {
        Sent::building_on(self.sent, self.param_base.unmodulated_normalized_value())
    }
}

impl View for Dial {
    fn element(&self) -> Option<&'static str> {
        Some("dial")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match *window_event {
            // vizia reports every third quick click as a triple click; treating it as a press lets
            // a drag straight after a double-click work.
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) => {
                // A press while a drag is still open means its release was lost (focus taken
                // mid-drag, say): carry on in that drag's gesture, which the next release ends,
                // rather than opening a second one.
                if self.drag_from.is_none() {
                    cx.capture();
                    // Otherwise the keyboard modifiers that follow aren't tracked for this view.
                    cx.focus();
                    cx.set_active(true);
                    self.param_base.begin_set_parameter(cx);
                }
                self.drag_from = Some(cx.mouse().cursory);
                meta.consume();
            }
            WindowEvent::MouseMove(_, y) => {
                if let Some(from) = self.drag_from {
                    // Screen y grows downward. Logical pixels make a drag cover the same share of
                    // the dial at any scale.
                    let rise = (from - y) / cx.scale_factor();
                    self.drag_from = Some(y);
                    let value = dragged(self.current(), rise, cx.modifiers().shift());
                    self.set(cx, value);
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_from.take().is_some() {
                    cx.release();
                    cx.set_active(false);
                    self.param_base.end_set_parameter(cx);
                    meta.consume();
                }
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                self.set(cx, self.param_base.default_normalized_value());
                meta.consume();
            }
            WindowEvent::MouseScroll(_, lines) => {
                self.scrolled += lines;
                let notches = self.scrolled.trunc();
                if notches != 0.0 {
                    self.scrolled -= notches;
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a whole number of notches from one wheel event fits an i32"
                    )]
                    let notches = notches as i32;
                    let value = wheeled(self.current(), notches, cx.modifiers().shift());
                    self.set(cx, value);
                }
                meta.consume();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let thickness = cx.border_width();
        let centre_x = bounds.x + bounds.w / 2.0;
        let centre_y = bounds.y + bounds.h / 2.0;
        let radius = (bounds.w.min(bounds.h) - thickness) / 2.0;
        let value = self.current();
        let end = angle(value);

        let arc = |to: f32| {
            let mut path = vg::Path::new();
            path.arc(
                centre_x,
                centre_y,
                radius,
                START_ANGLE,
                to,
                vg::Solidity::Hole,
            );
            path
        };
        let stroke = |canvas: &mut Canvas, path: &vg::Path, paint: &vg::Paint, spread: f32| {
            let mut paint = paint.clone();
            paint.set_line_width(2.0f32.mul_add(spread, thickness));
            paint.set_line_cap(vg::LineCap::Round);
            canvas.stroke_path(path, &paint);
        };
        stroke(
            canvas,
            &arc(angle(1.0)),
            &vg::Paint::color(cx.background_color().into()),
            0.0,
        );
        if value > 0.0 {
            let value_arc = arc(end);
            self.glow.draw(cx, canvas, 1.0, |canvas, paint, spread| {
                stroke(canvas, &value_arc, paint, spread);
            });
            stroke(
                canvas,
                &value_arc,
                &vg::Paint::color(cx.border_color().into()),
                0.0,
            );
        }

        let mut dot = vg::Path::new();
        dot.circle(
            radius.mul_add(end.cos(), centre_x),
            radius.mul_add(end.sin(), centre_y),
            thickness * DOT_RADIUS,
        );
        canvas.fill_path(&dot, &vg::Paint::color(cx.outline_color().into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn dragging_up_200_pixels_covers_the_whole_range() {
        assert_close(dragged(0.0, 200.0, false), 1.0);
    }

    #[test]
    fn dragging_down_lowers_the_value() {
        assert_close(dragged(0.5, -20.0, false), 0.4);
    }

    #[test]
    fn a_fine_drag_moves_a_tenth_as_far() {
        assert_close(
            dragged(0.5, 20.0, true) - 0.5,
            (dragged(0.5, 20.0, false) - 0.5) / 10.0,
        );
    }

    #[test]
    fn dragging_past_either_end_stops_there() {
        assert_close(dragged(0.9, 100.0, false), 1.0);
        assert_close(dragged(0.1, -100.0, false), 0.0);
    }

    #[test]
    fn a_wheel_notch_steps_5_percent_or_1_percent_with_shift() {
        assert_close(wheeled(0.5, 1, false), 0.55);
        assert_close(wheeled(0.5, -2, false), 0.4);
        assert_close(wheeled(0.5, 1, true), 0.51);
    }

    #[test]
    fn wheeling_past_either_end_stops_there() {
        assert_close(wheeled(0.98, 1, false), 1.0);
        assert_close(wheeled(0.02, -1, false), 0.0);
    }

    #[test]
    fn a_sent_value_is_built_on_until_the_parameter_moves_off_its_old_value() {
        let sent = Some(Sent {
            value: 0.6,
            over: 0.5,
        });

        // The host hasn't taken the change yet.
        assert_close(Sent::building_on(sent, 0.5), 0.6);
        // It has.
        assert_close(Sent::building_on(sent, 0.6), 0.6);
        // Automation has moved the parameter since.
        assert_close(Sent::building_on(sent, 0.2), 0.2);
        // Nothing sent yet.
        assert_close(Sent::building_on(None, 0.3), 0.3);
    }

    #[test]
    fn the_arc_runs_270_degrees_clockwise_from_the_bottom_left() {
        assert_close(angle(0.0).to_degrees(), 135.0);
        assert_close(angle(0.5).to_degrees(), 270.0);
        assert_close(angle(1.0).to_degrees(), 405.0);
    }
}
