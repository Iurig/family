//! The family's editor look, Night (see `docs/family-identity.md`): the dial, the switch, the lamp
//! and the glow, the bundled typeface, and the stylesheet every plugin shares.
//!
//! A plugin's editor calls [`install`] with its colour first, then adds its own stylesheet for its
//! layout. The components take their colours from the stylesheet and know nothing about the plugin
//! they're in.

pub mod dial;
pub mod glow;
pub mod lamp;
pub mod switch;

use nih_plug::nih_error;
use nih_plug_vizia::vizia::prelude::*;

pub use dial::Dial;
pub use glow::Glow;
pub use lamp::Lamp;
pub use switch::Switch;

/// The typeface: Archivo (SIL Open Font License, `assets/fonts/OFL.txt`).
///
/// At its normal width, in two weights. vizia can't switch on OpenType features, so these copies
/// have Archivo's tabular figures made the default ones, and numbers keep their width while a dial
/// moves. Renamed from "Archivo" for that change, which also keeps an installed Archivo from
/// standing in for them.
pub const FONT_FAMILY: &str = "Archivo Tabular";

/// Values and labels.
const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/ArchivoTabular-Regular.ttf");

/// Names, at `font-weight: semi-bold`.
const FONT_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/ArchivoTabular-SemiBold.ttf");

/// A plugin's signal colour, which it uses for the value arc, the lamp, the switch when on and the
/// bar along the top edge, as CSS hex colours (`#rrggbb`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colour {
    /// The colour itself: the arc, the switch when on, the bar.
    pub signal: &'static str,
    /// A lighter tint, for the lit lamp.
    pub lit: &'static str,
}

/// Loads the typeface as the default font and adds the shared stylesheet in `colour`. Call it
/// before adding the plugin's own stylesheet, which can then override any of it.
pub fn install(cx: &mut Context, colour: Colour) {
    cx.add_font_mem(FONT_REGULAR);
    cx.add_font_mem(FONT_SEMIBOLD);
    cx.set_default_font(&[FONT_FAMILY]);
    if let Err(err) = cx.add_stylesheet(CSS::String(stylesheet(colour))) {
        nih_error!("Failed to load the family's stylesheet: {err:?}");
    }
}

/// The stylesheet every plugin shares, in `colour`: the panel, the typeface's sizes and weights,
/// the colour bar, the rule between areas, and the components' colours and sizes.
///
/// vizia's CSS has no variables, so the colour is written into the text.
#[must_use]
pub fn stylesheet(colour: Colour) -> String {
    SHARED_STYLE
        .replace("$signal", colour.signal)
        .replace("$lit", colour.lit)
}

/// The shared stylesheet, with `$signal` and `$lit` for the plugin's colour. The glows are the
/// signal colour at 55 % opacity on a dial's arc (`8c`) and 80 % on the lamp (`cc`).
const SHARED_STYLE: &str = include_str!("night.css");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stylesheet_is_in_the_plugins_colour() {
        let style = stylesheet(Colour {
            signal: "#123456",
            lit: "#abcdef",
        });

        assert!(!style.contains('$'), "{style}");
        assert!(style.contains("background-color: #123456;"), "{style}");
        assert!(
            style.contains("box-shadow: 0px 0px 18px #1234568c;"),
            "{style}"
        );
        assert!(style.contains("color: #abcdef;"), "{style}");
        assert!(
            style.contains("box-shadow: 0px 0px 10px 3px #123456cc;"),
            "{style}"
        );
    }
}
