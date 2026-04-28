use clap::Parser;

/// Quick Image View Application
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, default_value_t=10000)]
    pub max_depth:usize,

    #[arg(long, default_value_t=30)]
    pub cache_size:usize,

    #[arg(long, default_value_t=1000)]
    pub delay: u64,

    #[arg(long, visible_alias="fs")]
    pub fullscreen: bool,

    /// Foo AND bar
    #[arg(long, visible_alias="ns")]
    pub no_splash: bool,


    #[arg(long,short)]
    pub slideshow: bool,

    #[arg(long)]
    pub window_events: bool,

    #[arg(long)]
    pub window_frames: bool,

    #[arg(long, default_value_t=10)]
    pub look_ahead: isize,
    #[arg(long, default_value_t=10)]
    pub look_behind: isize,


    #[arg(long)]
    pub time_forward_loop: bool,


    /// Directory
     #[arg(value_name="FILES/DIRS", default_values_t = vec![String::from(".")])]
    pub dirs: Vec<String>,

}


pub fn do_args() -> Args {
    Args::parse()
}
