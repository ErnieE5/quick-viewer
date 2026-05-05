// #![allow(unused_imports)]
use ee_conio::{cprintln};

mod args;
mod viewer;
mod img_traits;
mod img_list;
mod list_files;
mod toast;


const MIN_DELAY: u64 = 5;

use crate::viewer::{QuickViewer,Message};

use iced::mouse::{ ScrollDelta };
use iced::time::Instant;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::text::Wrapping;

use iced::task::Handle as TaskHandle;

use std::hash::{Hash, Hasher};

use iced::widget::{
    Container,
    // Row,
    table,
    Theme,
    float,
    stack,
    center,
    button,
    hover,
    // center_x, center_y, checkbox,
    column,
    container,
    scrollable,
    container::Style     as CStyle,
    progress_bar,
    progress_bar::Style  as PBStyle,
    mouse_area,
    image as iced_image,
    row,
    text,
};

use iced::{
    Element,
    Length,
    Fill,
    Font,
    Subscription,
    Task,
    Background,
    Padding,
    Renderer,
    border,
    color,
    keyboard,
    clipboard,
    alignment::Vertical,
    alignment::Horizontal,
};

use std::collections::{ HashMap };
use std::path::PathBuf;
use std::cmp::max;

use lru::LruCache;
use std::num::NonZeroUsize as ImageKey;

use iced::advanced::image::Allocation as ImageAllocation;
use iced::advanced::image::Error      as AllocError;


enum Msg {
    Hi,
    Qv(Message),
}

struct App {
    hi:String,
    qv:QuickViewer,
}

impl App {
    fn default() -> Self {
        Self {
            hi: "Hi!".into(),
            qv: QuickViewer::default(),
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


pub fn main() -> iced::Result {
    #[cfg(feature = "heif")]
    libheif_rs::integration::image::register_all_decoding_hooks();

    let settings = iced::window::Settings {
        transparent:true,
        // decorations:false,
        icon: Some(iced::window::icon::from_file_data(include_bytes!("../assets/icon.png"),Some(image::ImageFormat::Png)).expect("1")),
        ..iced::window::Settings::default()
    };

    iced::application::timed(
        App::new,
        App::update,
        App::subscription,
        App::view
    )
    .title("Quick Viewer")
    .centered()
    .run()

    // iced::application::timed(
    //         QuickViewer::new,
    //         QuickViewer::update,
    //         QuickViewer::subscription,
    //         QuickViewer::view
    //     )
    //     .theme(QuickViewer::theme)
    //     .title("Quick Viewer")
    //     .window(settings)
    //     .centered()
    //     .transparent(true)
    //     // .style(|_state, _theme| {
    //     //     iced::theme::Style {
    //     //         background_color:
    //     //         iced::Color { r:0.0,g:0.0,b:0.0,a:0.0 },
    //     //         text_color: color!(0xefefef),
    //     //     }
    //     // })
    //     .run()
}

