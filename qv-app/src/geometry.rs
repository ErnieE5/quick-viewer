//
//  Window geometry.
//
//  iced hands window::Settings to winit before any event loop exists, and monitor
//  enumeration only becomes available after that point -- so at the moment qv has to say
//  how big the window is, iced cannot tell it how big the screen is. That is why the
//  numbers come from the platform directly here rather than from iced.
//
//  The rules are ee_feh's, deliberately: a base rectangle comes from the target screen,
//  then --x/--y/--w/--h/--b override individual components, and naming any of them cancels
//  full screen. Asking for a size is asking for a window.
//
//  Everything in this module is LOGICAL pixels -- physical pixels divided by the monitor's
//  DPI scale. That is the unit window::Settings speaks, and the same unit Qt gives ee_feh,
//  so a number that meant one thing there means the same thing here.
//
//  The mod declaration in main.rs carries #[rustfmt::skip] for this whole file -- the
//  column alignment below is deliberate.
//

use iced::window::{Position, Settings};
use iced::{Point, Size};

use ee_conio::cprintln;

use crate::args::Args;

//  iced's own defaults, used whenever nothing asks for anything else. Kept here so plain
//  `qv <dir>` behaves exactly as it always has.
const DEFAULT_W: f32 = 1024.0;
const DEFAULT_H: f32 =  768.0;

//  iced asks wgpu for Limits::default() (iced_wgpu/src/window/compositor.rs:152), whose
//  max_texture_dimension_2d is 8192. The surface is a texture, so a window wider or taller
//  than this cannot be configured -- on ANY backend, because the ceiling is iced's request
//  rather than the hardware's capability. Exceeding it panics inside wgpu, so --span checks
//  first and says something useful instead.
const MAX_SURFACE: f32 = 8192.0;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub full:    Rect,      // the whole display
    pub work:    Rect,      // minus taskbar and friends
    pub scale:   f32,       // 1.5 at 150%
    pub primary: bool,
}

//---------------------------------------------------------------------------------------
//
//  Windows: EnumDisplayMonitors + GetMonitorInfoW, with per-monitor effective DPI.
//
#[cfg(windows)]
mod platform {
    use super::{Monitor, Rect};

    use windows_sys::Win32::Foundation::{LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows_sys::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
        SetProcessDpiAwarenessContext,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    const MONITORINFOF_PRIMARY: u32 = 1;

    //
    //  Claim per-monitor DPI awareness before anything asks Windows about a monitor.
    //
    //  Until this is set the process is DPI-unaware, and Windows lies to it kindly:
    //  GetDpiForMonitor answers 96 for every display whatever the real scaling is, so a
    //  150% monitor reads as 100% and every rectangle converts by the wrong factor.
    //
    //  winit sets the same context from become_dpi_aware(), but not until the event loop
    //  is built -- long after we need these numbers. Its call sits behind a Once and fails
    //  harmlessly when the context is already set, so claiming it first is safe.
    //
    //  It is worth having for its own sake too: an unaware process gets its window
    //  bitmap-stretched by Windows on a scaled display instead of rendering at full
    //  resolution.
    //
    pub fn become_dpi_aware() {
        use std::sync::Once;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        });
    }

    unsafe extern "system" fn collect(
        h: HMONITOR, _hdc: HDC, _rc: *mut RECT, data: LPARAM,
    ) -> windows_sys::core::BOOL {
        unsafe {
            let v = &mut *(data as *mut Vec<HMONITOR>);
            v.push(h);
        }
        1
    }

    fn to_logical(r: RECT, scale: f32) -> Rect {
        Rect {
            x:  r.left   as f32 / scale,
            y:  r.top    as f32 / scale,
            w: (r.right  - r.left) as f32 / scale,
            h: (r.bottom - r.top ) as f32 / scale,
        }
    }

    pub fn monitors() -> Vec<Monitor> {
        become_dpi_aware();

        let mut handles: Vec<HMONITOR> = Vec::new();

        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(collect),
                &mut handles as *mut _ as LPARAM,
            );
        }

        let mut out: Vec<Monitor> = Vec::new();

        for h in handles {
            let mut mi: MONITORINFO = unsafe { std::mem::zeroed() };
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

            if unsafe { GetMonitorInfoW(h, &mut mi) } == 0 {
                continue;
            }

            //  Effective DPI is per-monitor; 96 is 100%. A failure here means "assume
            //  100%", which is what an unaware process would have been shown anyway.
            let mut dx: u32 = 96;
            let mut dy: u32 = 96;
            unsafe {
                let _ = GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
            }
            let scale = if dx == 0 { 1.0 } else { dx as f32 / 96.0 };

            out.push(Monitor {
                full:    to_logical(mi.rcMonitor, scale),
                work:    to_logical(mi.rcWork,    scale),
                scale,
                primary: (mi.dwFlags & MONITORINFOF_PRIMARY) != 0,
            });
        }

        //  EnumDisplayMonitors order is whatever the driver felt like handing back. Sort
        //  it into something stable -- primary first, then left to right -- so a --screen
        //  index means the same monitor on every run. --list-screens prints the result,
        //  because this order is NOT guaranteed to match ee_feh's Qt screen order.
        out.sort_by(|a, b| {
            b.primary
                .cmp(&a.primary)
                .then(a.full.x.partial_cmp(&b.full.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        out
    }

    pub fn virtual_rect() -> Option<Rect> {
        //  The virtual desktop comes back in physical pixels. winit resolves a logical
        //  position against the primary monitor, so scale it the same way.
        let scale = monitors()
            .iter()
            .find(|m| m.primary)
            .map(|m| m.scale)
            .unwrap_or(1.0);

        unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN)  as f32;
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN)  as f32;
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN) as f32;
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN) as f32;

            if w <= 0.0 || h <= 0.0 {
                return None;
            }

            //  The metrics are physical, and physical is what the surface has to be.
            if w > super::MAX_SURFACE || h > super::MAX_SURFACE {
                ee_conio::cprintln!(
                    "~[c196]--span~[c227] wants a {w:.0}x{h:.0} surface; iced caps a texture at {:.0}",
                    super::MAX_SURFACE
                );
                ee_conio::cprintln!("~[c227]in either direction, so the whole desktop cannot be covered.");
                ee_conio::cprintln!("~[c227]Full screen on the primary display instead.~[XK]");
                return None;
            }

            Some(Rect { x: x / scale, y: y / scale, w: w / scale, h: h / scale })
        }
    }
}

//---------------------------------------------------------------------------------------
//
//  Everywhere else: no monitor query. --screen and --span have nothing to stand on, but
//  --x/--y/--w/--h still work, because an explicit number needs no base rectangle.
//
#[cfg(not(windows))]
mod platform {
    use super::{Monitor, Rect};

    pub fn become_dpi_aware() {}

    pub fn monitors() -> Vec<Monitor> { Vec::new() }

    pub fn virtual_rect() -> Option<Rect> { None }
}

pub use platform::{become_dpi_aware, monitors};

#[derive(Debug, Clone)]
pub struct Resolved {
    pub size:        Size,
    pub position:    Position,
    pub fullscreen:  bool,
    pub decorations: bool,
}

//
//  --list-screens. Printed before anything else in qv has run, then we are done.
//
pub fn print_screens() {
    let mons = monitors();

    if mons.is_empty() {
        cprintln!("~[c196]no monitor information is available on this platform~[XK]");
        return;
    }

    cprintln!(
        "~[c245]{:<8}  {:<7}  {:>24}  {:>24}  {:>5}~[XK]",
        "--screen", "", "full (logical)", "work (logical)", "scale"
    );

    for (i, m) in mons.iter().enumerate() {
        cprintln!(
            "~[c51]{:<8}~[x0]  ~[c227]{:<7}~[x0]  {:>24}  {:>24}  {:>4.0}%~[XK]",
            i,
            if m.primary { "primary" } else { "" },
            format!("{:.0}x{:.0} @ {:.0},{:.0}", m.full.w, m.full.h, m.full.x, m.full.y),
            format!("{:.0}x{:.0} @ {:.0},{:.0}", m.work.w, m.work.h, m.work.x, m.work.y),
            m.scale * 100.0,
        );
    }
}

//
//  Pick a base rectangle, then let explicit components overwrite it. The ordering mirrors
//  ee_feh's in viewer.py so the same flags land in the same place.
//
pub fn resolve(args: &Args) -> Resolved {
    let asked_for_geometry = args.x.is_some()
        || args.y.is_some()
        || args.w.is_some()
        || args.h.is_some()
        || args.b.is_some()
        || args.screen != 0
        || args.span;

    //  Nothing asked for: leave qv exactly as it was, except that --fs is now born full
    //  screen instead of toggling into it after the first frame.
    if !asked_for_geometry {
        return Resolved {
            size:        Size::new(DEFAULT_W, DEFAULT_H),
            position:    Position::Centered,
            fullscreen:  args.fullscreen,
            decorations: !args.borderless,
        };
    }

    let mons = monitors();

    //  Full screen wants the whole display; a window wants the part that is not taskbar.
    let base: Option<Rect> = if args.span {
        //  A desktop too wide for one surface falls back to the primary display, which is
        //  the nearest thing to "as much as you can have". virtual_rect() has already said
        //  why on stderr.
        platform::virtual_rect()
            .or_else(|| mons.iter().find(|m| m.primary).map(|m| m.full))
    } else {
        mons.get(args.screen)
            .map(|m| if args.fullscreen { m.full } else { m.work })
    };

    //  Covering a whole display or the whole desktop is full screen in all but name, so
    //  the decorations go -- but only while every component still comes from the screen.
    let covers = base.is_some() && (args.span || args.fullscreen);

    let mut w = base.map(|r| r.w).unwrap_or(DEFAULT_W);
    let mut h = base.map(|r| r.h).unwrap_or(DEFAULT_H);
    let mut x = base.map(|r| r.x);
    let mut y = base.map(|r| r.y);

    let mut overridden = false;

    if let Some(v) = args.x { x = Some(v as f32);          overridden = true; }
    if let Some(v) = args.y { y = Some(v as f32);          overridden = true; }
    if let Some(v) = args.w { w = v as f32;                overridden = true; }
    if let Some(v) = args.h { h = v as f32;                overridden = true; }
    if let Some(v) = args.b { h = (h - v as f32).max(1.0); overridden = true; }

    let position = match (x, y) {
        (Some(x), Some(y)) => Position::Specific(Point::new(x,   y  )),
        (Some(x), None   ) => Position::Specific(Point::new(x,   0.0)),
        (None,    Some(y)) => Position::Specific(Point::new(0.0, y  )),
        (None,    None   ) => Position::Centered,
    };

    Resolved {
        size:        Size::new(w.max(1.0), h.max(1.0)),
        position,
        fullscreen:  false,
        decorations: !args.borderless && !(covers && !overridden),
    }
}

//
//  Fold a Resolved into the window::Settings main() is about to hand iced.
//
pub fn apply(settings: Settings, r: &Resolved, overlay: bool) -> Settings {
    use iced::window::Level;

    Settings {
        size:        r.size,
        position:    r.position,
        fullscreen:  r.fullscreen,
        decorations: r.decorations,
        level:       if overlay { Level::AlwaysOnTop } else { Level::Normal },
        ..settings
    }
}
