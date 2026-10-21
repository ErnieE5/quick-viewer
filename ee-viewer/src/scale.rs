//
//  Scale modes: how an image is sized into the window. ee_feh's ScaleMode (viewer.py),
//  same four, same names, same order -- Alt+F steps forward through them, Alt+Shift+F back.
//
//      Fit     fitted to the window, aspect kept; small images are scaled up
//      fit     fitted only if it is larger than the window; otherwise 1:1
//      Fill    covers the window, aspect kept; the overflow is cropped
//      un      1:1, always
//
//  1:1 means one image pixel to one screen pixel. The canvas speaks logical pixels, so
//  `px` -- the logical size of one physical pixel, 1/scale_factor -- is what turns a
//  pixel count into a size: 1000 pixels at 150% is 666.7 logical. A 1:1 image is also
//  placed on whole physical pixels, or the sampler would blur every one of them by half.
//
//  Knows nothing of iced beyond its geometry types.
//

use iced::{ Rectangle, Size, Point, };

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScaleMode {
    Fit,
    Shrink,
    Fill,
    Unscaled,
}

const SEQ: [ScaleMode; 4] = [ ScaleMode::Fit, ScaleMode::Shrink, ScaleMode::Fill, ScaleMode::Unscaled ];

impl ScaleMode {
    pub fn next(self) -> Self { SEQ[ (self.at() + 1) % SEQ.len() ] }
    pub fn prev(self) -> Self { SEQ[ (self.at() + SEQ.len() - 1) % SEQ.len() ] }

    fn at(self) -> usize { SEQ.iter().position(|m| *m == self).expect("reality") }

    /// ee_feh's names, case and all: "Fit" and "fit" are two different modes.
    pub fn name(self) -> &'static str {
        match self {
            ScaleMode::Fit      => "Fit",
            ScaleMode::Shrink   => "fit",
            ScaleMode::Fill     => "Fill",
            ScaleMode::Unscaled => "un",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        SEQ.iter().copied().find(|m| m.name() == s)
    }

    /// Where an image of `w` x `h` pixels goes in a window of `bounds`, centred. `px` is
    /// the logical size of one physical pixel. The rectangle may be larger than the
    /// window (Fill, and un or fit on a big image); the caller clips.
    pub fn place(self, bounds: Size, w: f32, h: f32, px: f32) -> Rectangle {
        if w <= 0.0 || h <= 0.0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return Rectangle::new(Point::ORIGIN, Size::ZERO);
        }

        let native  = Size::new(w * px, h * px);
        let fits    = native.width <= bounds.width && native.height <= bounds.height;

        let rw = bounds.width  / w;
        let rh = bounds.height / h;

        let q = match self {
            ScaleMode::Fit                  => scaled(w, h, rw.min(rh)),
            ScaleMode::Fill                 => scaled(w, h, rw.max(rh)),
            ScaleMode::Shrink   if !fits    => scaled(w, h, rw.min(rh)),
            ScaleMode::Shrink   |
            ScaleMode::Unscaled             => native,
        };

        //  Every edge on a whole physical pixel, scaled or not: an edge between two pixels
        //  is drawn half into each, a soft line no one asked for.
        let grid = |v: f32| (v / px).round() * px;
        let q    = Size::new(grid(q.width), grid(q.height));

        let x = grid((bounds.width  - q.width ) / 2.0);
        let y = grid((bounds.height - q.height) / 2.0);

        Rectangle::new(Point::new(x, y), q)
    }
}

fn scaled(w: f32, h: f32, r: f32) -> Size { Size::new(w * r, h * r) }


#[cfg(test)]
mod tests {
    use super::*;

    const WIN: Size = Size { width: 1000.0, height: 500.0 };

    #[test]
    fn the_sequence_goes_round_both_ways() {
        let mut m = ScaleMode::Fit;
        for _ in 0..4 { m = m.next(); }
        assert_eq!(m, ScaleMode::Fit);
        assert_eq!(ScaleMode::Fit.prev(), ScaleMode::Unscaled);
        assert_eq!(ScaleMode::Fit.next(), ScaleMode::Shrink);
    }

    #[test]
    fn names_round_trip_and_are_case_sensitive() {
        for m in SEQ { assert_eq!(ScaleMode::parse(m.name()), Some(m)); }
        assert_eq!(ScaleMode::parse("fit"), Some(ScaleMode::Shrink));
        assert_eq!(ScaleMode::parse("FIT"), None);
    }

    #[test]
    fn fit_scales_a_small_image_up() {
        let r = ScaleMode::Fit.place(WIN, 100.0, 100.0, 1.0);
        assert_eq!(r.size(), Size::new(500.0, 500.0));
        assert_eq!(r.position(), Point::new(250.0, 0.0));
    }

    #[test]
    fn shrink_leaves_a_small_image_alone_and_fits_a_big_one() {
        let r = ScaleMode::Shrink.place(WIN, 100.0, 100.0, 1.0);
        assert_eq!(r.size(), Size::new(100.0, 100.0));

        let r = ScaleMode::Shrink.place(WIN, 4000.0, 1000.0, 1.0);
        assert_eq!(r.size(), Size::new(1000.0, 250.0));
    }

    #[test]
    fn fill_covers_the_window() {
        let r = ScaleMode::Fill.place(WIN, 100.0, 100.0, 1.0);
        assert_eq!(r.size(), Size::new(1000.0, 1000.0));
        assert_eq!(r.position(), Point::new(0.0, -250.0));
    }

    #[test]
    fn one_to_one_is_physical_pixels() {
        //  At 150% one physical pixel is 2/3 of a logical one.
        let px = 1.0 / 1.5;
        let r  = ScaleMode::Unscaled.place(WIN, 300.0, 150.0, px);
        assert!((r.width  - 200.0).abs() < 1e-3);
        assert!((r.height - 100.0).abs() < 1e-3);

        //  ...and lands on the physical grid.
        let gx = r.x / px;
        let gy = r.y / px;
        assert!((gx - gx.round()).abs() < 1e-3);
        assert!((gy - gy.round()).abs() < 1e-3);
    }

    #[test]
    fn shrink_judges_fit_in_physical_pixels() {
        //  1200 px wide fits a 1000-logical window at 150% (800 logical) and is left 1:1.
        let r = ScaleMode::Shrink.place(WIN, 1200.0, 600.0, 1.0 / 1.5);
        assert!((r.width - 800.0).abs() < 1e-3);
    }

    #[test]
    fn scaled_images_land_on_the_physical_grid_too() {
        //  2000 x 3000 fitted into 1707 x 960 logical at 150%: 640 logical wide is 960
        //  physical, but the centre and the odd sizes fall between pixels unless snapped.
        let px  = 1.0 / 1.5;
        let win = Size::new(2561.0 * px, 1439.0 * px);
        for m in SEQ {
            for (w, h) in [ (2000.0, 3000.0), (333.0, 777.0), (5001.0, 1003.0) ] {
                let r = m.place(win, w, h, px);
                for v in [ r.x, r.y, r.width, r.height ] {
                    let g = v / px;
                    assert!((g - g.round()).abs() < 1e-2, "{m:?} {w}x{h}: {v} is {g} physical");
                }
            }
        }
    }

    #[test]
    fn a_degenerate_image_is_nowhere() {
        let r = ScaleMode::Fit.place(WIN, 0.0, 0.0, 1.0);
        assert_eq!(r.size(), Size::ZERO);
    }
}
