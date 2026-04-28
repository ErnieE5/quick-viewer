// #![allow(unused_imports)]
use ee_conio::{cprintln};

mod args;
mod img_traits;
mod img_list;
mod list_files;
mod toast;

use toast::{Toast};

use crate::list_files::SomeFiles;

use crate::img_traits::{ImageError};
use crate::list_files::{FileSystemHelper};

use crate::img_list::{ImageList};

use iced::keyboard;
use iced::mouse::{ ScrollDelta };
use iced::time::Instant;
// use iced::widget::{ Column, Container, Slider,Image};
// use iced::Function;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::text::Wrapping;

use iced_core::image::Handle;

use iced::widget::{
    // Column,
    center,
    button,
    // center_x, center_y, checkbox,
    column,
    container,
    // text_input, toggler,
    mouse_area,
    image as iced_image,
    //stack,
    // , radio, rich_text,
    row,
    // scrollable, slider, space, span,
    text,
    image::viewer,
};
use iced::{
    // Center, Color,
    Element,
    Fill,
    Font,
    // Point,
    // Pixels,
    // Rectangle,
    // Renderer,
    // Rotation, Radians, ContentFit,window,
    // Size,
    Subscription,
    Task,
    Theme,
    color,
};

use std::collections::HashSet;
use std::path::PathBuf;

use lru::LruCache;
use std::num::NonZeroUsize;

use iced::advanced::image::Allocation;


#[derive(Debug, Clone)]
enum Message {
    Up,
    Left,
    Down,
    Right,
    Space,
    Quit,
    Sort,
    Shuffle,
    Swap,
    Scrolled(ScrollDelta),
    FileDropped(PathBuf),
    RandomImage,
    PageDown,
    PageUp,
    Home,
    End,
    Update,
    FullScreenToggle,
    Noop,
    Goodbye,
    ByeToaster(usize),

    RequestAnImage(NonZeroUsize),
    ImageLoaded( Result<(NonZeroUsize, ImageHandle ), ImageError>),
    ImageCached( NonZeroUsize, Result<Allocation,iced::advanced::image::Error> ),

    FindFilesOnPath,
    FoundSomeFiles(SomeFiles),
    FileFindComplete ,
    CancelFileFind,


}


#[derive(Debug)]
pub struct QuickViewer {
    args:                   args::Args,
    #[allow(unused)]
    start:                  Instant,
    loop_start:             Instant,
    now:                    Instant,
    img_list:               ImageList,


    toasts: Vec<Toast>,


    show_when_loaded:       Option<NonZeroUsize>,
    current_image_handle:   Option<ImageHandle>,
    cache_image_alloc:      LruCache<NonZeroUsize, Allocation>,
    pending_image_requests:  HashSet<NonZeroUsize>,

    scan_dir_task:          Option<iced::task::Handle>,
    current_scan_dir:       String,

    // scale_factor:           viewer::State,

    zoom:bool,
    fullscreen:bool,

    empty_image: ImageHandle,

}


use num_format::{Locale, ToFormattedString};
fn num<T:ToFormattedString>(n:T) -> String
{
    n.to_formatted_string(&Locale::en)
}


impl QuickViewer {
    fn default() -> Self {
        let args = args::do_args();


        Self {
            cache_image_alloc:  LruCache::new(NonZeroUsize::new(args.cache_size).unwrap()),

            args,


            toasts: vec![
                // Toast { message: "0".into() },
                // Toast { message: "1".into(),},
                // Toast { message: "2".into(),},
            ],


            start:          Instant::now(),
            loop_start:     Instant::now(),
            now:            Instant::now(),
            img_list:       ImageList::new(),


            current_image_handle: None,

            show_when_loaded: None,
            pending_image_requests: HashSet::new(),
            scan_dir_task: None,
            // scale_factor: viewer::State::new(),
            zoom:false,
            fullscreen:false,
            current_scan_dir:String::from(""),

            empty_image: {
                use image::ImageReader;
                use std::io::Cursor;

                let Ok(reader) = ImageReader::new(Cursor::new(include_bytes!("../assets/icon.png"))).with_guessed_format() else {
                    panic!();
                };

                let image = match reader.decode() {
                    Ok(i) => i,
                    Err(_e) => { panic!(); }
                };

                let w = image.width();
                let h = image.height();
                let d = image.to_rgba8().into_raw();

                ImageHandle::from_rgba(w, h, d)
            }
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
            Err(_) => { return false; }
        };

        match self.cache_image_alloc.get( &key ) {
            None => false,
            Some(h) => {
                if Some(h.handle()) != self.current_image_handle.as_ref() {
                    self.current_image_handle = Some(h.handle().clone());
                    self.show_when_loaded = None;
                    true
                }
                else { false }
            }
        }
    }

    fn goto_image(&mut self, index:NonZeroUsize) -> Task<Message> {
        if self.img_list.is_empty() { return Task::none(); }

        let key = match self.img_list.key_at(index) {
            Ok(k)   => k,
            Err(e)  => { cprintln!("~[c196]{e:?} ~[c255]{index}"); todo!()}
        };

        match self.img_list.goto(index) {
            Ok(i)   => { assert_eq!(index,i); },
            Err(e)  => { cprintln!("~[c196]{e:?} ~[c255]{index}"); todo!()}
        };

        if self.cache_image_alloc.get( &key ).is_some() {
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

        // If it isn't already in the cache, request that it be loaded in
        // anticipation that we will want it soon.
        let mut add_to_batch = |key| {
            if self.cache_image_alloc.get(&key).is_none() {
                if self.pending_image_requests.insert(key) {
                    m.push(Task::done(Message::RequestAnImage(key)));
                }
            }
        };

        let il = &self.img_list;

        // Request current first so it might get scheduled quicker
        match il.key() {
            Ok(k) => add_to_batch(k),
            Err(_) => { },
        };

        // MOST of the time the only item that NEEDS preload will be either
        // -10 back or 10 forward depending on the direction moved when cycling
        // images 1 by one.  All other just get ignored in the batching routine.
        for i in il.peek_range(-1*self.args.look_behind..=self.args.look_ahead) {
            match il.key_at(i) {
                Ok(k) => add_to_batch(k),
                Err(_) => todo!(),
            };
        }

        Task::batch(m)
    }

    fn handle_error(&mut self,key:NonZeroUsize,t:&str,i:&[u8]) -> Task<Message> {

        let path = match self.img_list.item_from_key(key) {
            Ok(ic) => { ic.fqp().display().to_string() },
            Err(_) => { String::from("") }
        };
            cprintln!("{t} {key:x} {path}");

        self.pending_image_requests.remove(&key);

        self.current_image_handle = Some( Handle::from_bytes( i.to_vec() ) );

        if Some(key) == self.show_when_loaded {
            let idx = self.img_list.find_index(key).expect("key not found");
            let _   = self.img_list.goto(idx).expect("key not valid");

            if self.showit() {
                self.show_when_loaded = None;
            }

            // if self.args.slideshow {
            //     self.args.slideshow = false;
            // }

            return Task::done(Message::Update);

        }

        Task::none()
    }


    fn update(&mut self, event: Message, now: Instant) -> Task<Message> {
        self.now = now;
        // cprintln!("~[c7]{:?}    {:40.40}",now-self.start,format!("{:?}",event) );
        match event {
            Message::Noop     =>    { Task::none() },
            Message::Goodbye  =>    { iced::exit() },

            Message::ByeToaster(idx) => {
                self.toasts.remove(idx);
                Task::none()
            },

            Message::Up =>      {
                self.args.delay += 5;
                Task::none()
            },
            Message::Down =>    {
                self.args.delay -= 5;
                Task::none()
            },


            Message::RequestAnImage(key) => {
                // cprintln!("RAI ~[c61]{:x}",key);

                match self.img_list.item_from_key(key) {
                    Ok(ic) => {
                        let (m,_h) = Task::perform(FileSystemHelper::load_image(ic.fqp(), key), Message::ImageLoaded).abortable();
                        // TODO ???
                        m
                    },

                    Err(x) => {
                        cprintln!("~[c197]{x:?}");
                        Task::none()
                    }
                }
            }

            Message::ImageLoaded(Err(ImageError::ErrorOpeningImageFile(key))) => {
                self.handle_error(key,"ErrorOpeningImageFile",include_bytes!("../assets/open_error.png"))
            },

            Message::ImageLoaded(Err(ImageError::ErrorReadingImageFile(key))) => {
                self.handle_error(key,"ErrorReadingImageFile",include_bytes!("../assets/read_error.png"))
            },

            Message::ImageLoaded(Err(ImageError::ErrorGuessingFormat(key))) => {
                self.handle_error(key,"ErrorGuessingFormat",include_bytes!("../assets/format_error.png"))
            },

            Message::ImageLoaded(Err(ImageError::ErrorDecodingImage(key))) => {
                self.handle_error(key,"ErrorDecodingImage",include_bytes!("../assets/decode_error.png"))
            },


            Message::ImageLoaded(Err(e)) => {
                cprintln!("Message::ImageLoaded {:?}",e);
                Task::none()
            },

            Message::ImageLoaded(Ok((key,handle))) => {
                iced::widget::image::allocate(handle).map(move |alloc| { Message::ImageCached(*&key,alloc) } )
            },

            Message::ImageCached( k,Ok(a) ) => {
                // cprintln!("~[c255]{:?},~[c51]{k:x}  {:?}",self.now-self.start,a.handle());
                self.pending_image_requests.remove(&k);
                self.cache_image_alloc.push(k, a);

                if Some(k) == self.show_when_loaded {

                    let _ = match self.img_list.find_index(k)
                    {
                        Ok(i) => match self.img_list.goto(i) { Ok(i) => i, Err(_) => { panic!(); } },
                        Err(_) => { panic!(); }
                    };

                    if self.showit() {
                        self.show_when_loaded = None;
                    }
                }

                Task::none()
            }

            Message::ImageCached( _k,Err(_e) ) => {
                // cprintln!("~[c255]{:?},Message::ImageCached {k:x} -- {e:?}",self.now-self.start);
                Task::none()
            }


            Message::FoundSomeFiles(p) => {
                self.current_scan_dir   = p.current_dir;

                let items = match self.img_list.total_items() {
                    Ok(i) => i.get(),
                    Err(_) => { 0 }
                };
                if items < 20 {
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

                // let mut m: Vec<Task<Message>> = vec![];

                let mode = if self.fullscreen  {
                    self.fullscreen = false; window::Mode::Windowed

                } else {
                    self.fullscreen = true;  window::Mode::Fullscreen
                };

                window::latest().and_then(move |id| window::set_mode(id, mode))
            }


            Message::Quit => {
                use iced::window;

                let m: Vec<Task<Message>> = vec![
                    window::latest().and_then(move |id| iced::window::minimize(id, true)),
                    Task::done(Message::Goodbye),
                ];

                Task::batch(m)
            },

            Message::Sort => {
                let _ = self.img_list.sort();
                self.preload()
            }

            Message::Shuffle => {
                let _ = self.img_list.shuffle();
                self.preload()
            }


            Message::RandomImage => {
                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                } else {
                    let idx = match self.img_list.random() {
                        Ok(i) => i,
                        Err(_) => { panic!(); }
                    };
                    self.goto_image( idx )
                }
            }

            Message::PageUp => {
                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                }
                else
                {
                    let idx = self.img_list.peek_range(-100..=-100).next().expect("1");
                    self.goto_image(idx)
                }
            }

            Message::PageDown => {
                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                }
                else
                {
                    let idx = self.img_list.peek_range(100..=100).next().expect("1");
                    self.goto_image(idx)
                }
            }

            Message::Home => {
                let next_image = match self.img_list.first() {
                    Ok(i) => i,
                    Err(_) => { panic!(); }
                };
                self.goto_image(next_image)
            },
            Message::End => {
                let next_image = match self.img_list.last() {
                    Ok(i) => i,
                    Err(_) => { panic!(); }
                };

                self.goto_image( next_image )
            },

            Message::Swap => {
                self.zoom=!self.zoom;
                Task::none()
            }

            Message::Scrolled(ScrollDelta::Pixels{x: _, y: _}) => { Task::none() }
            Message::Scrolled(ScrollDelta::Lines{x:_,y}) => {
                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                }
                else
                {
                    let delta = -y as isize;
                    let idx   = self.img_list.peek_range(delta..=delta).next().expect("1");
                    self.goto_image(idx)
                }
            }

            Message::FileDropped(f) => {
                cprintln!("~[c58]{}",f.display());
                Task::none()
            }

            Message::Left => {
                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                } else {
                    let Some(next_idx) = self.img_list.peek_range(-1..=-1).next() else {
                        return Task::done(Message::Update);
                    };

                    self.goto_image(next_idx)
                }
            }

            Message::Right => {

                if self.show_when_loaded.is_some() {
                    Task::done(Message::Update)
                } else {
                    let Some(next_idx) = self.img_list.peek_range(1..=1).next() else {
                        return Task::done(Message::Update);
                    };

                    if self.args.time_forward_loop {
                        if next_idx == NonZeroUsize::new(1).expect("reality")
                        {
                            if !self.toasts.is_empty() {
                                self.toasts[0].message = format!("{:?}",now-self.loop_start);
                            }
                            cprintln!("{:?}",now-self.loop_start);
                            self.loop_start = Instant::now();
                        }
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
        // cprintln!("~[c51]{:?}      view ",self.now-self.start);

        let c = match self.img_list.the_index() {
            Ok(i) => i.get(),
            Err(_) => 0
        };
        let t = match self.img_list.total_items() {
            Ok(i) => i.get(),
            Err(_) => 0
        };

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
                // Shows the pending cache load
                let c = self.pending_image_requests.len();
                row![
                    if c > 0 {
                        text( format!("{} ",c) )
                               .size(8)
                               .width(iced::Length::Fill)
                               .height(iced::Length::Fill)
                               .align_x(text::Alignment::Right)
                               .align_y(iced::alignment::Vertical::Center)
                    } else { text("") }
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

        let h = match self.current_image_handle.clone() {
            Some(i) => i.clone(),
            None => { self.empty_image.clone() }
        };


        let img = if self.zoom {
            container(viewer(h)
                // .scale_step(0.10)
                // .content_fit(iced::ContentFit::Contain)
                .width(Fill)
                .height(Fill))
        }
        else {
            container(iced_image(h).width(Fill).height(Fill))
        };
        // let img = container(canvas(self).width(Fill).height(Fill));

        let content = column![
            mouse_area(img)
                .on_press(Message::Left)
                .on_right_press(Message::Right)
                .on_middle_press(Message::Swap)
                .on_scroll(|delta| { Message::Scrolled(delta) } ),
            container(row![
                row![ counter],
                row![ progress_area ]
            ]).height(iced::Length::Fixed(15.0))
            .clip(true),
        ];

        let stuff = if true {
            container( toast::Manager::new(content, &self.toasts, Message::ByeToaster)
                .timeout(400) )
        }
        else {
            container(content)
        };


        let cc = iced::Color { r:0.0,g:0.0,b:0.0,a:0.0 };

        center(stuff).width(Fill).height(Fill)
            .style( move |_| container::Style {
            background: Some(iced::Background::Color(cc)),
            ..container::Style::default()
        })
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
                    "h" => Some(Message::Shuffle),
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
                match x {
                    (_,iced::window::Event::FileDropped(x)) => {
                        Message::FileDropped(x)
                    },
                    (_,_) => { Message::Noop }
                }
            } ) );
        }

        if self.args.window_frames {
            s.push( iced::window::frames().map(|x| {
                cprintln!("{:?}",x);
                Message::Noop } ) );
        }

        Subscription::batch(s)
    }

    pub fn theme(&self) -> Theme {
        Theme::Moonfly
        // Theme::Oxocarbon
        // Theme::Ferra
    }

}


// use iced::mouse;
// use iced::Renderer;
// use iced::widget::canvas;
// use iced::widget::canvas::{ Program, Frame };

// use iced::{ Rectangle, Size, Point, };
// use iced_core::image::Renderer as CoreRenderer;

// impl QuickViewer {


//     fn fit(&self,bounds: Rectangle, w: f32, h: f32) -> Rectangle {
//         let rw = bounds.width / w;
//         let rh = bounds.height / h;

//         let q = if (w * rw).floor() <= bounds.width && (h * rw).floor() <= bounds.height {
//             Size::new(w * rw * self.scale_factor, h * rw * self.scale_factor)
//         } else if (w * rh).floor() <= bounds.width && (h * rh).floor() <= bounds.height {
//             Size::new(w * rh * self.scale_factor, h * rh * self.scale_factor)
//         } else {
//             cprintln!("{w:?} {h:?} {bounds:?} {rw:?} {rh:?}");
//             Size::new(0.0, 0.0);
//             todo!();
//         };

//         let a = Point::new(
//             (bounds.width - q.width) / 2.0,
//             (bounds.height - q.height) / 2.0,
//         );

//         Rectangle::new(a, q)
//     }
// }




// impl<Message> Program<Message> for QuickViewer {
//     type State = ();

//     fn draw(
//         &self,
//         _state: &Self::State,
//         renderer: &Renderer,
//         _theme: &Theme,
//         bounds: Rectangle,
//         _cursor: mouse::Cursor,
//     ) -> Vec<canvas::Geometry> {
//         let mut frame = Frame::new(renderer, bounds.size());

//         if let Some(han) = self.current_image_handle.clone() {
//             // match renderer.load_image(&han) {
//             //     Ok(_) => {}
//             //     Err(_) => {
//             //         todo!();
//             //     }
//             // }

//             let (w, h) = match renderer.measure_image(&han) {
//                 Some(g) => (g.width as f32, g.height as f32),
//                 None    => (0.0, 0.0),
//             };

//             frame.draw_image( self.fit(bounds, w , h), &han.clone());
//         } else {

//             let (w, h) = match renderer.measure_image(&self.empty_image) {
//                 Some(g) => (g.width as f32, g.height as f32),
//                 None    => (0.0, 0.0),
//             };

//             frame.draw_image( self.fit(bounds, w , h), &self.empty_image.clone());
//         }

//         vec![frame.into_geometry()]
//     }
// }






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
            QuickViewer::new,
            QuickViewer::update,
            QuickViewer::subscription,
            QuickViewer::view
        )
        .theme(QuickViewer::theme)
        .title("Quick Viewer")
        .window(settings)
        .centered()
        .transparent(true)
        // .style(|_state, _theme| {
        //     iced::theme::Style {
        //         background_color:
        //         iced::Color { r:0.0,g:0.0,b:0.0,a:0.0 },
        //         text_color: color!(0xefefef),
        //     }
        // })
        .run()
}

