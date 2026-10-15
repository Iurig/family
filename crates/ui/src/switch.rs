//! The switch for an on/off parameter.
//!
//! A pill-shaped track with a knob that slides to the right when on, followed by the parameter's
//! name. Clicking anywhere on it, name included, flips it inside one begin/end gesture, and it
//! follows host automation.
//!
//! The stylesheet sets its look: the track is as tall as the view, `background-color` is the
//! track, `outline-color` the knob, and `:checked` restyles both for on. Its children (the name)
//! are laid out after the track with `child-left`, and should be made non-hoverable.

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::{prelude::*, vg};
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;

/// The track's width as a multiple of its height.
const TRACK_ASPECT: f32 = 1.875;

/// The gap between the knob and the track's edge, as a fraction of the track's height.
const KNOB_INSET: f32 = 0.125;

/// A switch for one on/off parameter.
pub struct Switch {
    param_base: ParamWidgetBase,
}

impl std::fmt::Debug for Switch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Switch").finish_non_exhaustive()
    }
}

impl Switch {
    /// A switch for the parameter `params_to_param` picks out of `params`, with `content` (its
    /// name) built after the track.
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
        let on = ParamWidgetBase::make_lens(params, params_to_param, |param| {
            param.unmodulated_normalized_value() >= 0.5
        });
        Self {
            param_base: ParamWidgetBase::new(cx, params, params_to_param),
        }
        .build(cx, content)
        .checked(on)
    }
}

impl View for Switch {
    fn element(&self) -> Option<&'static str> {
        Some("switch")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            if let WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) = *window_event
            {
                let on = self.param_base.unmodulated_normalized_value() >= 0.5;
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, if on { 0.0 } else { 1.0 });
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let height = bounds.h;
        let width = height * TRACK_ASPECT;
        let mut track = vg::Path::new();
        track.rounded_rect(bounds.x, bounds.y, width, height, height / 2.0);
        canvas.fill_path(&track, &vg::Paint::color(cx.background_color().into()));

        let inset = height * KNOB_INSET;
        let radius = height / 2.0 - inset;
        let on = self.param_base.unmodulated_normalized_value() >= 0.5;
        let centre_x = if on {
            bounds.x + width - inset - radius
        } else {
            bounds.x + inset + radius
        };
        let mut knob = vg::Path::new();
        knob.circle(centre_x, bounds.y + height / 2.0, radius);
        canvas.fill_path(&knob, &vg::Paint::color(cx.outline_color().into()));
    }
}
