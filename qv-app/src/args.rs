use clap::Parser;

/// Quick Image View Application
///
/// A rust/iced based quick view application.
///
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct Args {

    //
    //  Window geometry. Names and rules follow ee_feh: a value given here overrides that
    //  one component of the target screen's rectangle, and naming ANY of them cancels
    //  full screen -- asking for a size is asking for a window. Values are logical
    //  (DPI-scaled) pixels, the same units ee_feh's Qt geometry uses.
    //

    /// window width in pixels
    #[arg(long, value_name="PIXELS")]
    pub w: Option<u32>,

    /// window height in pixels
    #[arg(long, value_name="PIXELS")]
    pub h: Option<u32>,

    /// window x position
    #[arg(long, value_name="PIXELS", allow_negative_numbers=true)]
    pub x: Option<i32>,

    /// window y position
    #[arg(long, value_name="PIXELS", allow_negative_numbers=true)]
    pub y: Option<i32>,

    /// trim this many pixels off the bottom of the window
    #[arg(long, value_name="PIXELS")]
    pub b: Option<u32>,

    /// which monitor to open on (see --list-screens)
    #[arg(long, default_value_t=0, value_name="N")]
    pub screen: usize,

    /// cover every display
    #[arg(long, visible_alias="all-screens")]
    pub span: bool,

    /// keep the window above every other window
    #[arg(long)]
    pub overlay: bool,

    /// no title bar and no border
    #[arg(long, visible_alias="bl")]
    pub borderless: bool,

    /// print what each --screen index refers to, then exit
    #[arg(long)]
    pub list_screens: bool,


    /// start full screen
    #[arg(long, short='F', visible_alias="fs")]
    pub fullscreen: bool,

    /// start in slideshow mode
    #[arg(long,short='S',)]
    pub slideshow: bool,

    //  How a step gets to the next image -- instant, fade, wipe or slide -- as ee_feh's
    //  --fade/--wipe/--slide/--no-slide. A wipe is the default; Alt+S cycles them live.

    /// step by sliding the strip
    #[arg(long, overrides_with="no_slide")]
    pub slide: bool,

    /// step by landing on the image at once: no motion
    #[arg(long)]
    pub no_slide: bool,

    /// how long a slide takes, in milliseconds
    #[arg(long, value_name="MS", default_value_t=400, value_parser=clap::value_parser!(u64).range(1..=10_000))]
    pub slide_ms: u64,

    /// step by dissolving the new image in over <MS> (300 when bare)
    #[arg(long, value_name="MS", num_args=0..=1, default_missing_value="300", value_parser=clap::value_parser!(u64).range(1..=10_000))]
    pub fade: Option<u64>,

    /// step by sweeping a soft edge across over <MS> (150 when bare); the default
    #[arg(long, value_name="MS", num_args=0..=1, default_missing_value="150", value_parser=clap::value_parser!(u64).range(1..=10_000))]
    pub wipe: Option<u64>,

    //  How an image is sized into the window, as ee_feh's --scale-mode/--fit/--fill.
    //  Alt+F cycles them live. 1:1 is one image pixel to one screen pixel.

    /// Fit: fit the window, scaling small images up. fit: shrink large images only, else 1:1.
    /// Fill: cover the window, cropping. un: 1:1 always
    #[arg(long, value_name="MODE", default_value="Fit", value_parser=["Fit","fit","Fill","un"])]
    pub scale_mode: String,

    /// --scale-mode Fit
    #[arg(long, conflicts_with_all=["fill"])]
    pub fit: bool,

    /// --scale-mode Fill
    #[arg(long)]
    pub fill: bool,

    /// delay to advance a slideshow in milliseconds
    #[arg(long,value_name="DELAY", default_value_t=1000, value_parser=clap::value_parser!(u64).range(crate::MIN_DELAY..86_400_00))]
    pub delay: u64,


    // Debugging / development args

    /// Maximum depth to recurse sub directories looking for images
    #[arg(long, default_value_t=10000)]
    pub max_depth:usize,


    /// number of images to keep around
    #[arg(long, default_value_t=20)]
    pub cache_size:usize,



    /// don't show Jasper on start :(
    #[arg(long, visible_alias="ns")]
    pub no_splash: bool,

    /// window events to console
    #[arg(long)]
    pub window_events: bool,

    /// window frames to console
    #[arg(long)]
    pub window_frames: bool,

    #[arg(long)]
    pub view_cache_look_ahead:bool,

    #[arg(long)]
    pub view_exif:bool,

    /// Cache look ahead
    #[arg(long, default_value_t=5)]
    pub look_ahead: isize,

    /// Cache look behind
    #[arg(long, default_value_t=5)]
    pub look_behind: isize,

    #[arg(long, default_value_t=12)]
    pub font_size: u32,

    /// emit time of a full loop through images
    #[arg(long)]
    pub time_forward_loop: bool,

    #[arg(long)]
    pub no_canvas: bool,


    /// List of files and/or directories
    #[arg(value_name="FILES/DIRS", default_values_t = vec![String::from(".")])]
    pub dirs: Vec<String>,
}


pub fn do_args() -> Args {
    Args::parse()
}
