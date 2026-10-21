//
//  Zoom and pan on an image, in physical pixels. Middle-click's mode (RenderMode::Viewer).
//
//  This is iced's image Viewer (iced_widget 0.14.2, src/image/viewer.rs; MIT, (c) Héctor
//  Ramón and the iced contributors), taken in because it cannot be told the scale factor.
//  iced's widget measures an image in logical pixels and starts it fitted to the window,
//  so at 150% its "actual size" is 1.5 screen pixels to every image pixel and resampled,
//  and no zoom step ever lands on 1:1. What changed:
//
//      - the zoom is a multiplier on the scale mode's size (Alt+F), as ee_feh's zoom is:
//        1.0 is the size the canvas showed, so going in and out of zoom moves nothing.
//        It carries from image to image, each image scaled from its own fitted size.
//      - `px`, the logical size of one physical pixel (1/scale_factor), places true 1:1,
//        and a zoom step that would cross it stops on it. It is a detent: the rest of that
//        spin is absorbed, and only a click after the wheel has rested (DETENT) goes on.
//      - the image is centred, and at 1:1 it lands on whole physical pixels.
//      - panning stops at the image's edges, rather than at its edge reaching mid-window.
//      - ContentFit and padding are gone: this widget has one fit, 1:1 times scale.
//
use crate::scale::ScaleMode;

use std::time::{Duration, Instant};

//  How long the wheel must rest before a click in the same direction leaves 1:1.
const DETENT: Duration = Duration::from_millis(350);

use iced_core::border;
use iced_core::image::{self, FilterMethod};
use iced_core::layout;
use iced_core::mouse;
use iced_core::renderer;
use iced_core::widget::tree::{self, Tree};
use iced_core::{
    Clipboard, Element, Event, Image, Layout, Length, Point, Radians, Rectangle, Shell, Size,
    Vector, Widget,
};

pub struct Zoom<Handle> {
    px:             f32,
    mode:           ScaleMode,
    width:          Length,
    height:         Length,
    min_scale:      f32,
    max_scale:      f32,
    scale_step:     f32,
    handle:         Handle,
    filter_method:  FilterMethod,
}

pub fn zoom<Handle>(handle: impl Into<Handle>, px: f32, mode: ScaleMode) -> Zoom<Handle> {
    Zoom::new(handle, px, mode)
}

impl<Handle> Zoom<Handle> {
    pub fn new<T: Into<Handle>>(handle: T, px: f32, mode: ScaleMode) -> Self {
        Zoom {
            handle:         handle.into(),
            px:             if px > 0.0 { px } else { 1.0 },
            mode,
            width:          Length::Shrink,
            height:         Length::Shrink,
            min_scale:      0.05,
            max_scale:      20.0,
            scale_step:     0.10,
            filter_method:  FilterMethod::default(),
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    //  On a whole physical pixel.
    fn grid(&self, v: f32) -> f32 {
        (v / self.px).round() * self.px
    }

    //  The image's size at zoom 1.0 -- what the canvas shows in this scale mode -- and the
    //  zoom at which it is 1:1, in logical pixels.
    fn fitted<Renderer>(&self, renderer: &Renderer, bounds: Size) -> (Size, f32)
    where
        Renderer: image::Renderer<Handle = Handle>,
    {
        let Size { width, height } = renderer.measure_image(&self.handle).unwrap_or_default();
        let (w, h) = (width as f32, height as f32);

        let fit = self.mode.place(bounds, w, h, self.px).size();
        let one = if fit.width > 0.0 { w * self.px / fit.width } else { 1.0 };

        (fit, one)
    }

    //  The image's size on screen at the current zoom, in logical pixels.
    fn scaled_size<Renderer>(&self, renderer: &Renderer, state: &State, bounds: Size) -> Size
    where
        Renderer: image::Renderer<Handle = Handle>,
    {
        let (fit, _) = self.fitted(renderer, bounds);

        Size::new(
            self.grid(fit.width  * state.scale),
            self.grid(fit.height * state.scale),
        )
    }
}

impl<Message, Theme, Renderer, Handle> Widget<Message, Theme, Renderer> for Zoom<Handle>
where
    Renderer: image::Renderer<Handle = Handle>,
    Handle: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width:  self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let Size { width, height } = renderer.measure_image(&self.handle).unwrap_or_default();
        let native = Size::new(width as f32 * self.px, height as f32 * self.px);

        layout::Node::new(limits.resolve(self.width, self.height, native))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let Some(cursor_position) = cursor.position_over(bounds) else {
                    return;
                };

                let (mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. }) = *delta;

                let state = tree.state.downcast_mut::<State>();
                let previous_scale = state.scale;
                let now = Instant::now();
                let dir = y.signum();

                //  Held at 1:1: clicks the same way that keep coming are the same spin.
                if let Some((d, last)) = state.detent {
                    if d == dir && now - last < DETENT {
                        state.detent = Some((d, now));
                        shell.capture_event();
                        return;
                    }
                    state.detent = None;
                }

                let (_, one) = self.fitted(renderer, bounds.size());
                let min_scale = self.min_scale.min(one);
                let max_scale = self.max_scale.max(one);

                if y < 0.0 && previous_scale > min_scale
                    || y > 0.0 && previous_scale < max_scale
                {
                    let mut scale = if y > 0.0 {
                        previous_scale * (1.0 + self.scale_step)
                    } else {
                        previous_scale / (1.0 + self.scale_step)
                    }
                    .clamp(min_scale, max_scale);

                    //  A step across or onto 1:1 stops on it, and holds.
                    //  (A step from 1:1 is leaving it, not crossing it.)
                    if (previous_scale - one).abs() > 1e-6 && (previous_scale - one) * (scale - one) <= 0.0 {
                        scale = one;
                        state.detent = Some((dir, now));
                    }

                    state.scale = scale;

                    //  Keep the image point under the cursor under the cursor.
                    let factor           = state.scale / previous_scale - 1.0;
                    let cursor_to_center = cursor_position - bounds.center();
                    let adjustment       = (cursor_to_center + state.current_offset) * factor;

                    let size = self.scaled_size(renderer, state, bounds.size());
                    state.current_offset = clamp(state.current_offset + adjustment, bounds, size);
                }

                shell.request_redraw();
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(cursor_position) = cursor.position_over(bounds) else {
                    return;
                };

                let state = tree.state.downcast_mut::<State>();

                state.cursor_grabbed_at = Some(cursor_position);
                state.starting_offset   = state.current_offset;

                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let state = tree.state.downcast_mut::<State>();

                state.cursor_grabbed_at = None;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let state = tree.state.downcast_mut::<State>();

                if let Some(origin) = state.cursor_grabbed_at {
                    let size  = self.scaled_size(renderer, state, bounds.size());
                    let delta = *position - origin;

                    state.current_offset = clamp(state.starting_offset - delta, bounds, size);

                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        if state.cursor_grabbed_at.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state  = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let size   = self.scaled_size(renderer, state, bounds.size());

        //  Centred, less the pan, on whole physical pixels.
        let offset      = clamp(state.current_offset, bounds, size);
        let translation = Vector::new(
            self.grid((bounds.width  - size.width ) / 2.0 - offset.x),
            self.grid((bounds.height - size.height) / 2.0 - offset.y),
        );

        let drawing_bounds = Rectangle::new(bounds.position(), size);

        let render = |renderer: &mut Renderer| {
            renderer.with_translation(translation, |renderer| {
                renderer.draw_image(
                    Image {
                        handle:         self.handle.clone(),
                        border_radius:  border::Radius::default(),
                        filter_method:  self.filter_method,
                        rotation:       Radians(0.0),
                        opacity:        1.0,
                        snap:           true,
                    },
                    drawing_bounds,
                    *viewport - translation,
                );
            });
        };

        renderer.with_layer(bounds, render);
    }
}

//  The pan, held so the image's edges never come inside the window on an axis where it is
//  larger than the window, and nil on an axis where it is not.
fn clamp(offset: Vector, bounds: Rectangle, size: Size) -> Vector {
    let hidden_width  = ((size.width  - bounds.width ) / 2.0).max(0.0);
    let hidden_height = ((size.height - bounds.height) / 2.0).max(0.0);

    Vector::new(
        offset.x.clamp(-hidden_width,  hidden_width ),
        offset.y.clamp(-hidden_height, hidden_height),
    )
}

#[derive(Debug, Clone, Copy)]
struct State {
    scale:              f32,
    starting_offset:    Vector,
    current_offset:     Vector,
    cursor_grabbed_at:  Option<Point>,
    detent:             Option<(f32, Instant)>,     // held at 1:1: the spin's direction, its last click
}

impl Default for State {
    fn default() -> Self {
        Self {
            scale:              1.0,
            starting_offset:    Vector::default(),
            current_offset:     Vector::default(),
            cursor_grabbed_at:  None,
            detent:             None,
        }
    }
}

impl<'a, Message, Theme, Renderer, Handle> From<Zoom<Handle>> for Element<'a, Message, Theme, Renderer>
where
    Renderer: 'a + image::Renderer<Handle = Handle>,
    Message: 'a,
    Handle: Clone + 'a,
{
    fn from(zoom: Zoom<Handle>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(zoom)
    }
}
