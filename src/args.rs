use clap::Parser;

/// Quick Image View Application
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Directory
     #[arg(default_value_t = String::from("."))]
    pub dir: String,

    #[arg(long, default_value_t=10000)]
    pub max_depth:usize,

    #[arg(long, default_value_t=1000)]
    pub delay: u64,

    #[arg(long,short)]
    pub slideshow: bool,

    #[arg(long)]
    pub window_events: bool,

    #[arg(long)]
    pub window_frames: bool,

    #[arg(long)]
    pub time_forward_loop: bool,


}


pub fn do_args() -> Args {
    Args::parse()
}
