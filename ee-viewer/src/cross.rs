//
//  Motions: how a step from one image to the next gets there. ee_feh's Alt+S set, same
//  four, same order -- instant, fade, wipe, slide -- and Alt+Shift+S goes the other way.
//
//      instant     the new image is simply there
//      fade        the new image dissolves in over the old one        (--fade, 300 ms)
//      wipe        a soft edge sweeps across, the way a slide would go (--wipe, 150 ms)
//      slide       the strip slides (strip.rs)                         (--slide-ms)
//
//  Fade and wipe are a cross, the Rust cousin of viewport.py's cross(): the step lands
//  at once and, for the duration, the image it left is still drawn underneath while the
//  arriving one is mixed over it, `mix` of the way in. A cross during a cross keeps the
//  departed image and runs the mix on from where it is, so a run of steps is one long
//  blend rather than a stutter. The mix is linear, as ee_feh's is.
//
//  Like strip.rs this is timing and arithmetic only: the viewer keeps the departed
//  image and asks this how far in the arriving one is.
//

use std::time::{Duration, Instant};

//  ee_feh's defaults, the values a bare --fade or --wipe takes.
pub const FADE_MS: u64 = 300;
pub const WIPE_MS: u64 = 150;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motion {
    Instant,
    Fade,
    Wipe,
    Slide,
}

const SEQ: [Motion; 4] = [ Motion::Instant, Motion::Fade, Motion::Wipe, Motion::Slide ];

impl Motion {
    pub fn next(self) -> Self { SEQ[ (self.at() + 1) % SEQ.len() ] }
    pub fn prev(self) -> Self { SEQ[ (self.at() + SEQ.len() - 1) % SEQ.len() ] }

    fn at(self) -> usize { SEQ.iter().position(|m| *m == self).expect("reality") }

    pub fn name(self) -> &'static str {
        match self {
            Motion::Instant => "instant",
            Motion::Fade    => "fade",
            Motion::Wipe    => "wipe",
            Motion::Slide   => "slide",
        }
    }
}

#[derive(Debug)]
pub struct Cross {
    from:       f32,                // the mix this cross started at: 0, or where the last one was
    mix:        f32,                // how far in the arriving image is, 0..=1
    start:      Option<Instant>,    // Some while crossing
    dur:        Duration,
    forward:    bool,               // which way a wipe sweeps
}

impl Cross {
    pub fn new() -> Self {
        Self {
            from:       0.0,
            mix:        1.0,
            start:      None,
            dur:        Duration::from_millis(1),
            forward:    true,
        }
    }

    /// Start a cross of `ms`. Mid-cross the mix runs on from where it is; returns whether
    /// that happened, since the viewer then keeps the image it already departed from.
    pub fn go(&mut self, ms: u64, forward: bool, now: Instant) -> bool {
        let mid = self.active();

        self.from       = if mid { self.mix } else { 0.0 };
        self.mix        = self.from;
        self.start      = Some(now);
        self.dur        = Duration::from_millis(ms.max(1));     // 0 would divide by zero in tick()
        self.forward    = forward;

        mid
    }

    /// Be done. The arriving image is all there is.
    pub fn land(&mut self) {
        self.from   = 0.0;
        self.mix    = 1.0;
        self.start  = None;
    }

    pub fn tick(&mut self, now: Instant) {
        let Some(start) = self.start else { return; };

        let t = now.saturating_duration_since(start).as_secs_f32() / self.dur.as_secs_f32();

        if t >= 1.0 {
            self.land();
        } else {
            self.mix = self.from + (1.0 - self.from) * t;
        }
    }

    pub fn active(&self)  -> bool { self.start.is_some() }
    pub fn mix(&self)     -> f32  { self.mix }
    pub fn forward(&self) -> bool { self.forward }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration { Duration::from_millis(n) }

    #[test]
    fn motions_go_round_both_ways() {
        let mut m = Motion::Instant;
        for _ in 0..4 { m = m.next(); }
        assert_eq!(m, Motion::Instant);
        assert_eq!(Motion::Instant.prev(), Motion::Slide);
    }

    #[test]
    fn at_rest_the_arriving_image_is_all_there_is() {
        let c = Cross::new();
        assert!(!c.active());
        assert_eq!(c.mix(), 1.0);
    }

    #[test]
    fn a_cross_runs_linearly_and_lands() {
        let t0 = Instant::now();
        let mut c = Cross::new();

        assert!(!c.go(300, true, t0));
        assert_eq!(c.mix(), 0.0);

        c.tick(t0 + ms(150));
        assert!((c.mix() - 0.5).abs() < 1e-3);

        c.tick(t0 + ms(300));
        assert!(!c.active());
        assert_eq!(c.mix(), 1.0);
    }

    #[test]
    fn a_cross_mid_cross_runs_on_from_where_it_is() {
        let t0 = Instant::now();
        let mut c = Cross::new();

        c.go(300, true, t0);
        c.tick(t0 + ms(150));

        assert!(c.go(300, false, t0 + ms(150)));    // says it was mid-cross
        assert!((c.mix() - 0.5).abs() < 1e-3);      // no restart from 0
        assert!(!c.forward());

        c.tick(t0 + ms(300));
        assert!((c.mix() - 0.75).abs() < 1e-3);     // half of the remaining half
    }

    #[test]
    fn land_ends_it() {
        let t0 = Instant::now();
        let mut c = Cross::new();
        c.go(300, true, t0);
        c.land();
        assert!(!c.active());
        assert_eq!(c.mix(), 1.0);
    }
}
