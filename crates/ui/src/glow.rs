//! A soft glow under a shape a view draws itself, such as the dial's arc, as if it were lit.
//!
//! vizia draws CSS `box-shadow`s only around a view's box, so a view that draws its own shapes
//! reads its first `box-shadow` here instead: the colour is the glow's, the blur radius its size,
//! and the spread radius how far the shape grows before it's blurred.
//! It's drawn the way vizia draws shadows: the shape into an offscreen image, blurred, then laid
//! under the crisp shape.

use std::cell::Cell;

use nih_plug_vizia::vizia::{prelude::*, vg};

/// The offscreen images a glow is drawn and blurred in, kept between frames and remade when the
/// view's size changes.
#[derive(Debug, Default)]
pub struct Glow {
    images: Cell<Option<(vg::ImageId, vg::ImageId)>>,
}

impl Glow {
    /// Draws the glow of the shape `draw_shape` fills or strokes with the paint it's given, grown
    /// by the spread it's given in physical pixels, under whatever is drawn next, at `strength` (0
    /// to 1) of the `box-shadow`'s opacity. Draws nothing without a `box-shadow` with a blur
    /// radius.
    pub fn draw(
        &self,
        cx: &mut DrawContext,
        canvas: &mut Canvas,
        strength: f32,
        draw_shape: impl Fn(&mut Canvas, &vg::Paint, f32),
    ) {
        let Some(shadow) = cx.box_shadows().and_then(|shadows| shadows.first()) else {
            return;
        };
        let mut color: vg::Color = shadow.color.unwrap_or_default().into();
        color.a *= strength.clamp(0.0, 1.0);
        let blur = shadow
            .blur_radius
            .as_ref()
            .and_then(Length::to_px)
            .unwrap_or(0.0)
            * cx.scale_factor();
        let spread = shadow
            .spread_radius
            .as_ref()
            .and_then(Length::to_px)
            .unwrap_or(0.0)
            * cx.scale_factor();
        if blur <= 0.0 || color.a <= 0.0 {
            return;
        }
        // vizia's shadows blur with a sigma of half the radius; three sigmas hold all but a
        // trace of it.
        let sigma = blur / 2.0;
        let margin = sigma.mul_add(3.0, spread.max(0.0)).ceil();
        let bounds = cx.bounds();
        let (x, y) = (bounds.x - margin, bounds.y - margin);
        let (w, h) = (
            2.0f32.mul_add(margin, bounds.w),
            2.0f32.mul_add(margin, bounds.h),
        );
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a view's size in pixels is positive and far below usize's range"
        )]
        let size = (w.ceil() as usize, h.ceil() as usize);

        let Some((source, target)) = self.images(canvas, size) else {
            return;
        };

        canvas.save();
        canvas.set_render_target(vg::RenderTarget::Image(source));
        canvas.reset_scissor();
        canvas.reset_transform();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a view's size in pixels is far below u32's range"
        )]
        canvas.clear_rect(
            0,
            0,
            size.0 as u32,
            size.1 as u32,
            vg::Color::rgba(0, 0, 0, 0),
        );
        canvas.translate(-x, -y);
        draw_shape(canvas, &vg::Paint::color(color), spread);
        canvas.restore();
        canvas.filter_image(target, vg::ImageFilter::GaussianBlur { sigma }, source);
        canvas.set_render_target(vg::RenderTarget::Screen);

        let mut rect = vg::Path::new();
        rect.rect(x, y, w, h);
        canvas.fill_path(&rect, &vg::Paint::image(target, x, y, w, h, 0.0, 1.0));
    }

    /// The source and target images at `size`, made or remade as needed. `None` if the canvas
    /// can't make them, in which case there's no glow.
    fn images(
        &self,
        canvas: &mut Canvas,
        size: (usize, usize),
    ) -> Option<(vg::ImageId, vg::ImageId)> {
        if let Some((source, target)) = self.images.get() {
            if canvas.image_size(source).ok() == Some(size) {
                return Some((source, target));
            }
            canvas.delete_image(source);
            canvas.delete_image(target);
            self.images.set(None);
        }
        let flags = vg::ImageFlags::FLIP_Y | vg::ImageFlags::PREMULTIPLIED;
        let source = canvas
            .create_image_empty(size.0, size.1, vg::PixelFormat::Rgba8, flags)
            .ok()?;
        let Ok(target) = canvas.create_image_empty(size.0, size.1, vg::PixelFormat::Rgba8, flags)
        else {
            canvas.delete_image(source);
            return None;
        };
        self.images.set(Some((source, target)));
        Some((source, target))
    }
}
