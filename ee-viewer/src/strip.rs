//
//  The strip: left/right as a slide rather than a cut. The Rust cousin of ee_feh's
//  viewport.py.
//
//  The list is a strip of slots, one window wide, and the window is posed over it. Slot 0
//  is the current image, -1 the one before, +1 the one after. `offset` is where the window
//  is, in slots, relative to slot 0: a slot k is drawn at x = (k + offset) * width. At rest
//  the offset is 0 and slot 0 fills the window exactly as a plain draw would.
//
//  A step of +1 has already made the next image current by the time go() is called, so to
//  look unchanged on the first frame the window must still be over the old one: offset +1,
//  eased back to 0. The new image slides in from the right, the old one out to the left.
//
//  go() slides from wherever the window is right now, so a second step before the first
//  lands just bends the path -- no queue, no jump. The ease is ee_feh's OutCubic: fast out,
//  decelerating, settled exactly on 0.
//
//  The slide keeps pace with the keys. Each one gets the lesser of --slide-ms and the time
//  since the step before it, so it has always landed by the time the next step arrives. A
//  tap gets the whole slide; a held key, repeating every ~33ms, gets ~33ms slides and reads
//  as a fast continuous scroll rather than a pile-up trailing behind the keys. Nothing
//  paces the keys themselves -- qv steps as fast as they come, as it always has.
//
//  Knows nothing of iced, the cache or the list. It is timing and arithmetic; the viewer
//  asks it where to draw and when it is done.
//

use std::ops::RangeInclusive;
use std::time::{Duration, Instant};

//  The default; --slide-ms sets the real one. ee_feh's viewport.py uses 300, which felt
//  too fast here.
pub const SLIDE_MS: u64 = 400;

//  The furthest the window may trail its target, in slots. Holding an arrow down steps
//  faster than a slide settles; past this the trail is clamped rather than jumped, so the
//  motion stays continuous and never streams more than a few neighbours past.
const MAX_TRAIL: f32 = 3.0;

#[derive(Debug)]
pub struct Strip {
    from:   f32,                // offset the current slide started at
    offset: f32,                // where the window is now; what draw() reads
    start:  Option<Instant>,    // Some while sliding
    dur:    Duration,           // --slide-ms: the most any slide takes
    cur:    Duration,           // what this slide takes: dur, or less if the keys are quicker
    last:   Option<Instant>,    // the previous go(); survives jump(), it is key history
}

impl Strip {
    pub fn new(ms: u64) -> Self {
        Self {
            from:   0.0,
            offset: 0.0,
            start:  None,
            dur:    Duration::from_millis(ms.max(1)),     // 0 would divide by zero in tick()
            cur:    Duration::from_millis(ms.max(1)),
            last:   None,
        }
    }

    /// The list has moved `step` slots; slide there from wherever we are now.
    #[allow(dead_code)]     // the viewer uses go_from(); this is the plain form, used by the tests
    pub fn go(&mut self, step: i32, now: Instant) {
        self.go_from(self.offset, step, now);
    }

    /// go(), from an offset read before something else jump()ed the strip. The viewer's
    /// goto lands every move, arrows included, so an arrow reads the offset first and
    /// hands it back here -- otherwise every step would restart from a full slot.
    pub fn go_from(&mut self, offset: f32, step: i32, now: Instant) {
        let gap = match self.last {
            Some(last)  => now.saturating_duration_since(last),
            None        => self.dur,
        };

        self.cur    = gap.clamp(Duration::from_millis(1), self.dur);
        self.last   = Some(now);
        self.from   = (offset + step as f32).clamp(-MAX_TRAIL, MAX_TRAIL);
        self.offset = self.from;
        self.start  = Some(now);
    }

    /// Be there. No slide; ends any slide in progress.
    pub fn jump(&mut self) {
        self.from   = 0.0;
        self.offset = 0.0;
        self.start  = None;
    }

    /// Advance the ease to `now`. Lands, and stops itself, once the duration is up.
    pub fn tick(&mut self, now: Instant) {
        let Some(start) = self.start else { return; };

        let t = now.saturating_duration_since(start).as_secs_f32() / self.cur.as_secs_f32();

        if t >= 1.0 {
            self.jump();
        } else {
            self.offset = self.from * (1.0 - out_cubic(t));
        }
    }

    pub fn offset(&self) -> f32 { self.offset }

    pub fn active(&self) -> bool { self.start.is_some() }

    /// The slots at least partly on screen: those with |k + offset| < 1. At rest, just 0.
    pub fn slots(&self) -> RangeInclusive<i32> {
        let c = -self.offset;
        (c.floor() as i32) ..= (c.ceil() as i32)
    }
}

fn out_cubic(t: f32) -> f32 {
    let u = 1.0 - t;
    1.0 - u * u * u
}


#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration { Duration::from_millis(n) }

    #[test]
    fn at_rest_draws_only_the_current_slot() {
        let s = Strip::new(SLIDE_MS);
        assert_eq!(s.offset(), 0.0);
        assert_eq!(s.slots(), 0..=0);
        assert!(!s.active());
    }

    #[test]
    fn a_step_starts_over_the_old_image() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);

        s.go(1, t0);
        assert_eq!(s.offset(), 1.0);       // old image (slot -1) exactly in the window
        assert_eq!(s.slots(), -1..=-1);

        s.tick(t0 + ms(150));
        assert!(s.offset() > 0.0 && s.offset() < 1.0);
        assert_eq!(s.slots(), -1..=0);     // both halves on screen

        s.go(-1, t0 + ms(150));
        assert!(s.offset() < 0.0);         // reversed mid-slide: continues from where it was
    }

    #[test]
    fn it_settles_exactly_on_zero_and_stops() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);

        s.go(-1, t0);
        s.tick(t0 + ms(SLIDE_MS));
        assert_eq!(s.offset(), 0.0);
        assert_eq!(s.slots(), 0..=0);
        assert!(!s.active());
    }

    #[test]
    fn the_ease_is_monotonic_and_decelerating() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);
        s.go(1, t0);

        let mut last  = s.offset();
        let mut dlast = f32::MAX;
        for i in 1..30 {
            s.tick(t0 + ms(i * 10));
            let d = last - s.offset();
            assert!(d >= 0.0,   "went backwards at {i}");
            assert!(d <= dlast, "sped up at {i}");
            last  = s.offset();
            dlast = d;
        }
    }

    #[test]
    fn a_step_retargets_from_where_the_window_is() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);

        s.go(1, t0);
        s.tick(t0 + ms(100));
        let mid = s.offset();

        s.go(1, t0 + ms(100));
        assert_eq!(s.offset(), mid + 1.0); // bends the path; no jump
    }

    #[test]
    fn a_held_key_trail_is_clamped() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);
        for _ in 0..10 { s.go(1, t0); }
        assert_eq!(s.offset(), MAX_TRAIL);
        for _ in 0..20 { s.go(-1, t0); }
        assert_eq!(s.offset(), -MAX_TRAIL);
    }

    #[test]
    fn a_quick_step_gets_a_quick_slide() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);

        s.go(1, t0);
        s.go(1, t0 + ms(33));              // a key repeat
        s.tick(t0 + ms(33 + 33));
        assert_eq!(s.offset(), 0.0);       // landed within the repeat interval
        assert!(!s.active());
    }

    #[test]
    fn a_slow_step_gets_the_whole_slide() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);

        s.go(1, t0);
        s.go(1, t0 + ms(2000));            // a fresh tap, long after
        s.tick(t0 + ms(2000 + SLIDE_MS / 2));
        assert!(s.active());               // still sliding at half the full duration
    }

    #[test]
    fn go_from_carries_an_offset_across_a_jump() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);

        s.go(1, t0);
        s.tick(t0 + ms(100));
        let mid = s.offset();

        s.jump();                          // what goto_image_task does
        s.go_from(mid, 1, t0 + ms(100));
        assert_eq!(s.offset(), mid + 1.0);
    }

    #[test]
    fn jump_lands_and_stops() {
        let t0 = Instant::now();
        let mut s = Strip::new(SLIDE_MS);
        s.go(2, t0);
        s.jump();
        assert_eq!(s.offset(), 0.0);
        assert!(!s.active());
    }
}
