// #![allow(unused_imports)]
use ee_conio::{cprintln};

mod args;
mod img_traits;
mod img_list;
mod list_files;

use crate::list_files::SomeFiles;

use crate::img_traits::{ImageError};
use crate::list_files::{FileSystemHelper};

use crate::img_list::{ImageList};

use iced::keyboard;
use iced::mouse;
use iced::time::Instant;
// use iced::widget::{ Column, Container, Slider,Image};
// use iced::Function;
use iced::widget::image::Handle;
use iced::widget::text::Wrapping;
use iced::widget::{
    // Column,
    // center,
    button,
    // center_x, center_y, checkbox,
    column,
    container,
    // text_input, toggler,
    mouse_area,
    //stack,
    // image, radio, rich_text,
    row,
    // scrollable, slider, space, span,
    text,
};
use iced::{
    // Center, Color,
    Element,
    Fill,
    Font,
    Point,
    // Pixels,
    Rectangle,
    Renderer,
    // Rotation, Radians, ContentFit,window,
    Size,
    Subscription,
    Task,
    Theme,
    color,
};

use iced::widget::canvas;
use iced::widget::canvas::{
    // Canvas,
    Frame,
    // Text,
    // Event,
    Program,
};


// use iced_core::widget::Widget;
use iced_core::image::Renderer as CoreRenderer;

use std::collections::HashSet;

use lru::LruCache;
use std::num::NonZeroUsize;




#[derive(Debug, Clone)]
enum Message {
    Up,
    Left,
    Down,
    Right,
    Space,
    Quit,
    Sort,
    RandomImage,
    PageDown,
    PageUp,
    Home,
    End,
    Update,
    FullScreenToggle,
    Noop,

    RequestAnImage(usize),
    ImageLoaded(Result<(usize, Handle), ImageError>),

    FindFilesOnPath,
    FoundSomeFiles(SomeFiles),
    FileFindComplete ,
    CancelFileFind,


}


#[derive(Debug)]
pub struct QuickViewer {
    args:                   args::Args,
    start:                  Instant,
    now:                    Instant,
    img_list:               ImageList,


    show_when_loaded:       Option<usize>,
    current_image_handle:   Option<Handle>,
    cache_image_handle:     LruCache<usize, Handle>,
    pending_image_handles:  HashSet<usize>,

    scan_dir_task:          Option<iced::task::Handle>,
    current_scan_dir:       String,

    fullscreen:bool,

}


use num_format::{Locale, ToFormattedString};
fn num<T:ToFormattedString>(n:T) -> String
{
    n.to_formatted_string(&Locale::en)
}


impl QuickViewer {
    fn default() -> Self {
        Self {
            // state:State::Idle,
            args: args::do_args(),
            start: Instant::now(),
            now: Instant::now(),
            img_list: ImageList::new(),


            current_image_handle: None,
            // s: vec![ Scanner::new(1) ],
            cache_image_handle: LruCache::new(NonZeroUsize::new(30).unwrap()),
            show_when_loaded: None,
            pending_image_handles: HashSet::new(),
            scan_dir_task: None,
            fullscreen:false,
            current_scan_dir:String::from(""),
        }
    }

    fn new() -> (Self, Task<Message>) {
        (
            QuickViewer::default(),
            Task::done(Message::FindFilesOnPath)
        )
    }

    fn showit(&mut self) -> bool {
        let key = match self.img_list.key() {
            Ok(k) => k,
            Err(_) => { return false; } //todo!()
        };

        if self.cache_image_handle.get( &key ).is_some() {
            self.current_image_handle = Some(self.cache_image_handle.get(&key).unwrap().clone());
            return true;
        }

        false
    }

    fn goto_image(&mut self, index:usize) -> Task<Message> {

        if self.img_list.is_empty() { return Task::none(); }

        let key = match self.img_list.key_at(index) {
            Ok(k)   => k,
            Err(e)  => { cprintln!("~[c196]{e:?} ~[c255]{index}"); todo!()}
        };

        match self.img_list.goto(index) {
            Ok(i)   => { assert_eq!(index,i); },
            Err(e)  => { cprintln!("~[c196]{e:?} ~[c255]{index}"); todo!()}
        };

        if self.cache_image_handle.get( &key ).is_some() {
            self.showit();
        } else {
            self.show_when_loaded = Some(key);
        }

        self.preload()
    }

    fn preload(&mut self) -> Task<Message> {

        if self.img_list.is_empty() { return Task::none(); }

        let mut m: Vec<Task<Message>> = vec![
            Task::done(Message::Update)
        ];

        let mut add_to_batch = |key| {

            if self.cache_image_handle.get(&key).is_none() {
                if self.pending_image_handles.insert(key) {
                    m.push(Task::done(Message::RequestAnImage(key)));
                }
            }
        };

        let il = &self.img_list;

        match il.key() {
            Ok(k) => add_to_batch(k),
            Err(_) => todo!(),
        };

        for i in il.peek_foreward(10) {
            match il.key_at(i) {
                Ok(k) => add_to_batch(k),
                Err(_) => todo!(),
            };
        }
        for i in il.peek_backward(10) {
            match il.key_at(i) {
                Ok(k) => add_to_batch(k),
                Err(_) => todo!(),
            };
        }

        Task::batch(m)
    }

    fn update(&mut self, event: Message, now: Instant) -> Task<Message> {
        self.now = now;

        match event {
            Message::Noop =>    { Task::none() },
            Message::Up =>      { Task::none() },
            Message::Down =>    { Task::none() },

            Message::ImageLoaded(Err(e)) => {
                cprintln!("{:?}",e);
                Task::none()
            },

            Message::ImageLoaded(Ok((k, han))) => {
                self.pending_image_handles.remove(&k);
                self.cache_image_handle.push(k, han);

                if Some(k) == self.show_when_loaded {

                    let _ = match self.img_list.find_index(k)
                    {
                        Ok(i) => match self.img_list.goto(i) { Ok(i) => i, Err(_) => 0 },
                        Err(_) => { 0 }
                    };

                    if self.showit() {
                        self.show_when_loaded = None;
                    }

                    return Task::done(Message::Update);
                }

                Task::none()
            },

            Message::FoundSomeFiles(p) => {
                self.current_scan_dir   = p.current_dir;

                if self.img_list.total_items() < 20 {
                    self.img_list.append(p.files);
                    return self.preload();
                }

                self.img_list.append(p.files);
                Task::none()
            },
            Message::FileFindComplete => {
                self.scan_dir_task = None;
                return self.preload();
            },

            Message::CancelFileFind => {
                match &self.scan_dir_task {
                    None => { },
                    Some(h) => {
                        h.abort();
                        self.scan_dir_task = None;
                    }
                }
                return self.preload();
            },


            Message::FindFilesOnPath => {
                let (m,h) = Task::sip(
                    FileSystemHelper::find_files_sipper(self.args.dir.to_string(),self.args.max_depth),
                    Message::FoundSomeFiles,
                    | _e | { Message::FileFindComplete }
                ).abortable();

                self.scan_dir_task = Some(h);

                m
            },

            Message::FullScreenToggle => {
                use iced::window;

                let _id = window::latest();

                let (fullscreen, mode) = if self.fullscreen  {
                    ( false, window::Mode::Windowed )
                } else {
                    ( true, window::Mode::Fullscreen )
                };
                self.fullscreen = fullscreen;

                window::latest().and_then(move |id| window::set_mode(id, mode))
            }


            Message::RequestAnImage(key) => {
                // cprintln!("RAI ~[c61]{:x}",key);

                match self.img_list.item_from_key(key) {
                    Ok(ic) => {
                        Task::perform(FileSystemHelper::load_image(ic.fqp(), key), Message::ImageLoaded)
                    },

                    Err(x) => {
                        cprintln!("~[c197]{x:?}");
                        Task::none()
                    }
                }
            }

            Message::Quit => {
                iced::exit()
            },

            Message::Sort => {
                self.img_list.sort();
                self.preload()
            }

            Message::RandomImage => {
                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                } else {
                    let idx = self.img_list.random();
                    self.goto_image( idx )
                }
            }

            Message::PageUp => {
                Task::none()
            }

            Message::PageDown => {
                Task::none()
            }

            Message::Home => {
                let next_image = self.img_list.first();
                self.goto_image(next_image)
            },
            Message::End => {
                let next_idx = self.img_list.last();
                self.goto_image( next_idx )
            },

            Message::Left => {
                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                } else {
                    let Some(next_idx) = self.img_list.peek_backward(1).next() else {
                        return Task::done(Message::Update);
                    };

                    self.goto_image(next_idx)
                }
            }

            Message::Right => {

                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                } else {
                    let Some(next_idx) = self.img_list.peek_foreward(1).next() else {
                        return Task::done(Message::Update);
                    };

                    if next_idx == 1
                    {
                        cprintln!("{:?}",now-self.start);
                    }

                    self.goto_image(next_idx)
                }
            }

            Message::Space => {
                self.args.slideshow = !self.args.slideshow;
                Task::none()
            },

            Message::Update => {
                self.showit();
                if self.current_image_handle.is_none() {
                    return Task::done(Message::Update);
                }
                Task::none()
            }
        }
    }



    fn view(&self) -> Element<'_, Message> {

        let c = self.img_list.the_index();
        let t = self.img_list.total_items();

        let fnam = match self.img_list.item() {
            Ok(n) => n.display(),
            Err(_) => String::from(" ")
        };

        let clr = match self.show_when_loaded {
            None    => { color!(0xF5F5F5)  }
            Some(_) => { color!(0xFF00FF)  }
        };

        let progress_area = if self.scan_dir_task.is_some() { row![
                    text(self.current_scan_dir.clone())
                    .size(8)
                    .color(color!(0xFFFFFF))
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .align_x(text::Alignment::Right)
                    .align_y(iced::alignment::Vertical::Center)
                    .wrapping(Wrapping::None),
                    button( text("stop").size(10) ).padding([0,2]).height(iced::Length::Shrink).on_press(Message::CancelFileFind)
                ].spacing(10).padding([0,10])
            } else {
                row![ text( format!("{}/{}",self.pending_image_handles.len(),self.cache_image_handle.len()))
                        .size(10)
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                        .align_x(text::Alignment::Right)
                        .align_y(iced::alignment::Vertical::Center)
                ]
            };

        let counter = if t == 0 {
            row![ text!("No images").size(12).color(color!(0xC5B358)) ]
        } else {
            row![
                text!("{:>12}", num(c))
                    .size(12)
                    .color(clr)
                    .font(Font::MONOSPACE),
                text!("/")
                    .size(12)
                    .color(color!(0xafafaf))
                    .font(Font::MONOSPACE),
                text!("{:<12}", num(t))
                    .size(12)
                    .color(color!(0x536878))
                    .font(Font::MONOSPACE),
                text(fnam)
                    .size(12)
                    .color(color!(0xC5B358))
                    .wrapping(Wrapping::None),
            ]
        };


        column![
            mouse_area(canvas(self).width(Fill).height(Fill))
                .on_press(Message::Left)
                .on_right_press(Message::Right),
            container(row![
                row![ counter],
                row![ progress_area ]
            ]).height(iced::Length::Fixed(15.0))
            .clip(true),
        ]
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        use keyboard::Event as EV;
        use keyboard::Key as KK;
        use keyboard::key::Named as KN;


        let mut s = vec![
            keyboard::listen().filter_map(|event| match event {
                EV::KeyPressed { key: KK::Named(key), ..} => match key {

                    KN::ArrowUp      => Some(Message::Up),
                    KN::ArrowLeft    => Some(Message::Left),
                    KN::ArrowDown    => Some(Message::Down),
                    KN::ArrowRight   => Some(Message::Right),
                    KN::PageDown     => Some(Message::PageDown),
                    KN::PageUp       => Some(Message::PageUp),
                    KN::Space        => Some(Message::Space),
                    KN::Home         => Some(Message::Home),
                    KN::End          => Some(Message::End),
                    KN::Escape       => Some(Message::Quit),

                    _ => None,
                },
                EV::KeyPressed { key: KK::Character(key), ..} => match key.as_ref() {
                    "f" => Some(Message::FullScreenToggle),
                    "q" => Some(Message::Quit),
                    "s" => Some(Message::Sort),
                    "r" => Some(Message::RandomImage),
                    // a   => { cprintln!("{a}"); None }
                    _   => None,
                },
                _ => None,
            }),

        ];


        use iced::time;
        if self.args.slideshow {
            s.push( time::every(time::Duration::from_millis(self.args.delay)).map(|_| Message::Right) );
        }

        if self.args.window_events {
            s.push( iced::window::events().map(|x| {
                cprintln!("{x:?}");
                Message::Noop } ) );
        }

        if self.args.window_frames {
            s.push( iced::window::frames().map(|x| {
                cprintln!("{:?}",x);
                Message::Noop } ) );
        }

        Subscription::batch(s)
    }

    pub fn theme(&self) -> Theme {
        Theme::Oxocarbon
    }

}

fn fit(bounds: Rectangle, w: f32, h: f32) -> Rectangle {
    let rw = bounds.width / w;
    let rh = bounds.height / h;

    let q = if (w * rw).floor() <= bounds.width && (h * rw).floor() <= bounds.height {
        Size::new(w * rw, h * rw)
    } else if (w * rh).floor() <= bounds.width && (h * rh).floor() <= bounds.height {
        Size::new(w * rh, h * rh)
    } else {
        cprintln!("{w:?} {h:?} {bounds:?} {rw:?} {rh:?}");
        Size::new(0.0, 0.0);
        todo!();
    };

    let a = Point::new(
        (bounds.width - q.width) / 2.0,
        (bounds.height - q.height) / 2.0,
    );

    Rectangle::new(a, q)
}


impl<Message> Program<Message> for QuickViewer {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if let Some(han) = self.current_image_handle.clone() {
            match renderer.load_image(&han) {
                Ok(_) => {}
                Err(_) => {
                    todo!();
                }
            }

            let (w, h) = match renderer.measure_image(&han) {
                Some(g) => (g.width as f32, g.height as f32),
                None    => (0.0, 0.0),
            };

            frame.draw_image( fit(bounds, w , h), &han.clone());
        } else {
            // let ll = Point::new(0.0, bounds.height - 15.0 );

            // frame.fill_text(Text {
            //     content: String::from("Loading..."),
            //     position: ll,
            //     color: color!(0xF87431),
            //     size: 15.0.into(),
            //     ..Text::default()
            // });
        }

        vec![frame.into_geometry()]
    }
}

pub fn main() -> iced::Result {

    let yy = iced::window::Settings {
        icon: Some(iced::window::icon::from_file_data(include_bytes!("icon_png"),Some(image::ImageFormat::Png)).expect("1")),
        ..iced::window::Settings::default()
    };

    let app = iced::application::timed(QuickViewer::new, QuickViewer::update, QuickViewer::subscription, QuickViewer::view)
        .theme(QuickViewer::theme)
        .title("Quick Viewer")
        .window(yy)
        .position( iced::window::Position::Centered )
        .centered()
        .run();

    app
}

