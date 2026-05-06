use clap::Parser;

/// Quick Image View Application
///
/// A rust/iced based quick view application.
///
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {

    /// start full screen
    #[arg(long, short='F', visible_alias="fs")]
    pub fullscreen: bool,

    /// start in slideshow mode
    #[arg(long,short='S',)]
    pub slideshow: bool,

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
