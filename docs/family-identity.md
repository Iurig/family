# The plugin family's look

Doubler was the first of a set of guitar plugins; Quiet (noise reduction) is the second and a pitch shifter is planned. They share one look, called **Night**, chosen in Doubler's #21. This page is what each plugin starts from.

## What the family shares

- **The panel:** dark graphite, the same in every plugin.
- **The typeface:** Archivo, bundled, at one width in two weights (regular for values and labels, semi-bold for names), with tabular figures so numbers don't jitter.
- **The components:** the dial, the switch and the lamp, drawn the same way everywhere.
- **One colour per plugin.** Each plugin owns a signal colour and uses it for the same things: the value arc, the lamp, the switch when on, and a bar along the top edge. Nothing else is coloured. The plugin's name stays in the ink colour.
- **Glow on what's lit.** The value arc and a lit lamp glow in the plugin's colour, like LEDs behind smoked plastic.

The layout is not shared. Doubler's one large dial suits a plugin with one main control; the next plugin's layout follows its own controls (a semitone display, a gate meter). Different layouts also help tell the plugins apart.

## The tokens

| Role | Value |
| --- | --- |
| Panel | `#171b20` |
| Ink (names, values) | `#e9edf0` |
| Muted (units, end labels, secondary values) | `#86919c` |
| Rule between areas | `#262c33` |
| Dial track, switch track when off | `#262d35` |
| Lamp when dark | `#2e353d` |
| Switch knob when off | `#86919c` |
| Dial end dot | `#ffffff` |
| Colour bar | 3 px, the plugin's colour |

Each plugin's colour comes in three strengths: the colour itself (arc, switch, bar), a lighter tint for the lit lamp, and the colour at reduced opacity for the glow.

| Plugin | Colour | Lit lamp | Glow |
| --- | --- | --- | --- |
| Doubler | `#57cfbd` | `#8af0e0` | `#57cfbd8c` on the arc, `#57cfbdcc` on the lamp |
| Pitch shifter (proposed) | `#f0a43a` | `#ffc56e` | same opacities |
| Quiet | `#a396ff` | `#c4bbff` | same opacities |

The glow is set in CSS as a `box-shadow`: `0px 0px 18px` on a lit dial's arc, `0px 0px 10px 3px` on the lamp. The views that draw their own shapes read it through `Glow`, since vizia only draws box shadows around a view's box.

## The shared code

`family-ui` in this repo holds what the plugins share:

- `night.css`: the panel, the typeface's sizes, the colour bar, the rule, and the components' colours and sizes, with the plugin's colour written in by `stylesheet(Colour)`. vizia's CSS has no variables, so the colour is replaced in the text.
- `install(cx, Colour)`: loads the bundled Archivo as the default font and adds that stylesheet. A plugin's editor calls it first, then adds its own stylesheet for its layout, which can override anything.
- `Dial`: the 270° dial, with drag, fine drag, double-click reset, wheel steps and host gestures. Size it in the plugin's stylesheet; the `glow` class lights its arc.
- `Switch`: the on/off switch.
- `Lamp`: the lamp that blinks when a counter the audio thread keeps moves.
- `Glow`: the blurred glow under a self-drawn shape.

Each takes its colours from the stylesheet and knows nothing about the plugin it's in.
