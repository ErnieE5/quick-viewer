// #![allow(unused_imports)]
use ee_conio::{cprintln};

mod args;
mod viewer;
mod img_traits;
mod img_list;
mod list_files;
mod toast;


const MIN_DELAY: u64 = 5;

use crate::viewer::{QuickViewer,QVMsg};

use crate::args::{Args};

use iced::time::Instant;

use iced::widget::{
    container,
    // mouse_area,
    row,
    text,
};

use iced::{
    application,
    Element,
    Subscription,
    Task,
    // Background,
    // Padding,
    // Renderer,
    // border,
    // color,
    // keyboard,
    // clipboard,
    // alignment::Vertical,
    // alignment::Horizontal,
    window,
    window::{Settings,icon},
    Result as IcedResult
};

use image::{self,ImageFormat};


enum Msg {
    Hi,
    Qv(QVMsg),
}

struct App {
    args:           Args,
    hi:             String,
    qv:             QuickViewer,
}

impl App {
    fn default() -> Self {
        let args = args::do_args();

        Self {
            hi:     "Hi!".into(),
            qv:     QuickViewer::default(),

            args
        }
    }

    fn new() -> (Self, Task<Msg>) {
        ( App::default(), Task::done(Msg::Hi) )
    }

    fn update(&mut self, event: Msg, now: Instant) -> Task<Msg> {
        match event {
            Msg::Qv(m)  => { self.qv.update(m,now).map(Msg::Qv) },
            Msg::Hi     => { self.hi = "Hello".into(); Task::none() }
        }
    }
    fn view(&self) -> Element<'_, Msg> {
        container( 
            row![
                text!("{}",self.hi ),
                self.qv.view().map(Msg::Qv)
            ]
        ).into()
    }
    fn subscription(&self) -> Subscription<Msg> {
        let m = vec![
            self.qv.subscription().map(Msg::Qv)
        ];
        Subscription::batch( m )
    }
}


pub fn main() -> IcedResult {
    #[cfg(feature = "heif")]
    libheif_rs::integration::image::register_all_decoding_hooks();

    let settings = Settings {
        transparent:true,
        icon: Some(icon::from_file_data(include_bytes!("../assets/icon.png"),Some(ImageFormat::Png)).expect("1")),
        ..Settings::default()
    };

    application::timed(
        App::new,
        App::update,
        App::subscription,
        App::view
    )
    .window(settings)
    .title("Quick Viewer")
    .centered()
    .run()
}

