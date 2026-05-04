// #![allow(unused_imports)]
use ee_conio::{cprintln};

mod args;
mod img_traits;
mod img_list;
mod list_files;
mod toast;

const MIN_DELAY: u64 = 5;

use toast::{Toast};

use crate::list_files::ScanProgress;

use crate::img_traits::{ImageError,LoadData};
use crate::list_files::{FileSystemHelper};

use crate::img_list::{ImageList};


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
    image::viewer,
};

use iced::{
    Element,
    Length,
    Fill,
    Font,
    Subscription,
    Task,
    Background,
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


#[derive(Debug, Clone)]
enum Message {
    Left,
    Right,

    DecDelay,
    IncDelay,

    FontUp,
    FontDown,

    LookAheadDisplayToggle,
    ExifDisplayToggle,

    Space,
    Quit,
    Sort,
    SortSize,
    SortDate,
    Shuffle,
    Swap,
    Scrolled(ScrollDelta),
    FileDropped(PathBuf),
    RandomImage,
    PageDown,
    PageUp,
    Home,
    End,
    FullScreenToggle,
    Noop,
    Clip(String),
    #[allow(unused)]
    ClipResult(Result<(), std::fmt::Error>),
    Goodbye,
    ByeToaster(usize),

    RequestAnImage(ImageKey),
    ImageLoaded( Result<LoadData, ImageError>),
    ImageCached( ImageKey, Result<ImageAllocation,AllocError> ),

    FindFilesOnPath,
    FindFilesProgress(ScanProgress),
    FileFindComplete ,
    CancelFileFind,


}




#[derive(Debug,Clone)]
#[allow(unused)]
pub struct CacheData {
    pub key:        ImageKey,
    pub request:    Option<TaskHandle>,
    pub alloc:      Option<ImageAllocation>,
}

impl CacheData {
    pub fn new(key:ImageKey) -> CacheData {
        CacheData {
            key,
            request:    None,
            alloc:      None,

        }
    }
}

impl PartialEq for CacheData {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for CacheData {}

impl Hash for CacheData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}





#[derive(Debug)]
pub struct QuickViewer {
    args:                   args::Args,
    #[allow(unused)]
    start:                  Instant,
    loop_start:             Instant,
    now:                    Instant,
    img_list:               ImageList,

    loaded_image_info:          HashMap<ImageKey,LoadData>,


    toasts: Vec<Toast>,


    show_when_loaded:           Option<ImageKey>,
    current_image_handle:       Option<ImageHandle>,
    cache_image_alloc:          LruCache<ImageKey, ImageAllocation>,
    pending_image_requests:     HashMap<ImageKey,CacheData>,

    scan_dir_task:              Option<TaskHandle>,
    current_scan_dir:           String,

    zoom:                       bool,
    fullscreen:                 bool,

    empty_image: ImageHandle,

}


use num_format::{Locale, ToFormattedString};
fn num<T:ToFormattedString>(n:T) -> String
{
    n.to_formatted_string(&Locale::en)
}


macro_rules! whd_from_asset {
    ($fn:expr) => {
        {
            let Ok(reader) = image::ImageReader::new(
                                std::io::Cursor::new(
                                    include_bytes!($fn)
                                )
                            ).with_guessed_format()
            else {
                panic!();
            };

            let image = match reader.decode() {
                Ok(i) => i,
                Err(_e) => { panic!(); }
            };

            let w = image.width();
            let h = image.height();
            let d = image.to_rgba8().into_raw();

            (w,h,d)
        }
    }
}



impl QuickViewer {
    fn default() -> Self {
        let args = args::do_args();

        Self {
            cache_image_alloc:  LruCache::new(ImageKey::new(args.cache_size).unwrap()),
            fullscreen:         args.fullscreen,

            empty_image: {
                let (w,h,d) = if !args.no_splash {
                    whd_from_asset!("../assets/jasper.png")
                }
                else {
                    // 1 pixel zero opacity RGBA
                    (1,1,vec![0,0,0,0])
                };

                ImageHandle::from_rgba(w, h, d)
            },

            loaded_image_info: HashMap::new(),

            toasts: vec![
                // Toast { message: "1".into(),},
                // Toast { message: "2".into(),},
            ],


            start:          Instant::now(),
            loop_start:     Instant::now(),
            now:            Instant::now(),
            img_list:       ImageList::new(),


            current_image_handle:   None,

            show_when_loaded:       None,
            pending_image_requests: HashMap::new(),
            scan_dir_task:          None,
            zoom:                   false,
            current_scan_dir:       "".into(),

            args,
        }
    }

    fn new() -> (Self, Task<Message>) {
        let mut m: Vec<Task<Message>> = vec![
            Task::done(Message::FindFilesOnPath)
        ];

        let mut me = QuickViewer::default();

        if me.fullscreen {
            me.fullscreen = false;
            m.push( Task::done(Message::FullScreenToggle));
        }


        (
            me,
            Task::batch(m)
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
                }
                true
            }
        }
    }

    fn goto_image_task(&mut self, index:ImageKey) -> Task<Message> {
        if self.img_list.is_empty() { return Task::none(); }

        let key = match self.img_list.key_at(index) {
            Ok(k)   => k,
            Err(e)  => { cprintln!("~[c196]{e:?} ~[c255]{index}"); todo!()}
        };

        match self.img_list.goto(index) {
            Ok(i)   => { assert_eq!(index,i); },
            Err(e)  => { cprintln!("~[c196]{e:?} ~[c255]{index}"); todo!()}
        };

        if !self.showit() {
            // cprintln!("Zzzzzz ~[c255]{key} ~[c51]{:?}",self.now-self.start);
            self.show_when_loaded = Some(key);
        }

        self.preload_task(false)
    }

    fn preload_task(&mut self,only_one:bool) -> Task<Message> {

        if self.img_list.is_empty() { return Task::none(); }

        let mut m: Vec<Task<Message>> = vec![ ];

        // If it isn't already in the cache, request that it be loaded in
        // anticipation that we will want it soon.
        let mut add_to_batch = |key| {
            if self.cache_image_alloc.get(&key).is_none() {
                match self.pending_image_requests.get(&key) {
                    None => {
                        match self.pending_image_requests.insert(key,CacheData::new(key)) {
                            None => { m.push(Task::done(Message::RequestAnImage(key))); }
                            Some(_) => { panic!(); }
                        }
                    },
                    Some(_) => { }

                }
            }
        };

        let il = &self.img_list;

        // Request current first so it might get scheduled quicker
        match il.key() {
            Ok(k) => add_to_batch(k),
            Err(_) => { },
        };

        if !only_one {
            // MOST of the time the only item that NEEDS preload will be either
            // -10 back or 10 forward depending on the direction moved when cycling
            // images 1 by one.  All other just get ignored in the batching routine.
            for i in il.peek_range(-1*self.args.look_behind..=self.args.look_ahead) {
                match il.key_at(i) {
                    Ok(k) => add_to_batch(k),
                    Err(_) => todo!(),
                };
            }
        }

        Task::batch(m)
    }

    fn handle_error_task(&mut self,key:ImageKey,t:&str,i:&[u8]) -> Task<Message> {

        let path = match self.img_list.item_from_key(key) {
            Ok(ic) => { ic.fqp().display().to_string() },
            Err(_) => { String::from("") }
        };

        cprintln!("{t} {key:#?} {path}");

        let Some(_) = self.pending_image_requests.remove(&key) else {
            cprintln!("um");
            return Task::none();
        };



        if Some(key) == self.show_when_loaded {
            let idx = self.img_list.find_index(key).expect("key not found");
            let _   = self.img_list.goto(idx).expect("key not valid");

            self.current_image_handle = Some( ImageHandle::from_bytes( i.to_vec() ) );

            self.show_when_loaded = None;

            if self.args.slideshow {
                self.args.slideshow = false;
            }

            return Task::none();

        }

        Task::none()
    }


    fn update(&mut self, event: Message, now: Instant) -> Task<Message> {
        self.now = now;
        // cprintln!("~[c7]{:?}    {:40.40}",now-self.start,format!("{:?}",event) );
        match event {
            Message::Noop     =>    { Task::none() },
            Message::Goodbye  =>    { iced::exit() },

            Message::Clip(s)            => { clipboard::write(s) },
            Message::ClipResult(_)  => { Task::none() },
            // Message::ClipResult(Ok(_))  => { Task::none() },
            // Message::ClipResult(Err(e)) => { cprintln!("{e:?}"); Task::none() },


            Message::ByeToaster(idx) => {
                self.toasts.remove(idx);
                Task::none()
            },


            Message::FontDown => {
                if self.args.font_size > 6
                {
                    self.args.font_size -= 1;
                }
                else
                {
                    self.args.font_size = 0;
                }
                Task::none()
            },
            Message::FontUp => {
                if self.args.font_size == 0 {
                    self.args.font_size = 6;
                }
                else if self.args.font_size < 50
                {
                    self.args.font_size += 1;
                }
                Task::none()
            },

            Message::LookAheadDisplayToggle => {
                self.args.view_cache_look_ahead = !self.args.view_cache_look_ahead;
                Task::none()
            },

            Message::ExifDisplayToggle => {
                self.args.view_exif = !self.args.view_exif;
                Task::none()
            }

            Message::IncDelay =>      {
                self.args.delay += 5;

                self.args.delay = if self.args.delay%5 == 0 { self.args.delay } else {
                    (self.args.delay/5)*5
                };

                if self.toasts.len() > 0 {
                    self.toasts[0].message = self.args.delay.to_string();
                }
                else
                {
                    self.toasts.push( Toast { message: self.args.delay.to_string() } );
                }
                Task::none()
            },

            Message::DecDelay =>    {
                if self.args.delay > MIN_DELAY+5 {
                    self.args.delay -= 5;
                }
                else {
                    self.args.delay = MIN_DELAY;
                }
                self.args.delay = if self.args.delay%5 == 0 { self.args.delay } else {
                    ((self.args.delay+1)/5)*5
                };

                if self.toasts.len() > 0 {
                    self.toasts[0].message = self.args.delay.to_string();
                }
                else
                {
                    self.toasts.push( Toast { message: self.args.delay.to_string() } );
                }

                Task::none()
            },


            Message::RequestAnImage(key) => {
                // cprintln!("RAI ~[c61]{:x}",key);

                let Some(r) = self.pending_image_requests.get_mut(&key) else {
                    return Task::none();
                };

                match self.img_list.item_from_key(key) {
                    Ok(ic) => {
                        let (m,h) = Task::perform(FileSystemHelper::load_image(ic.fqp(), key), Message::ImageLoaded).abortable();

                        r.request = Some(h.abort_on_drop());

                        m
                    },

                    Err(x) => {
                        cprintln!("~[c197]{x:?}");
                        Task::none()
                    }
                }
            }

            Message::ImageLoaded(Err(ImageError::ErrorOpeningImageFile(key))) => {
                self.handle_error_task(key,"ErrorOpeningImageFile",include_bytes!("../assets/open_error.png"))
            },

            Message::ImageLoaded(Err(ImageError::ErrorReadingImageFile(key))) => {
                self.handle_error_task(key,"ErrorReadingImageFile",include_bytes!("../assets/read_error.png"))
            },

            Message::ImageLoaded(Err(ImageError::ErrorGuessingFormat(key))) => {
                self.handle_error_task(key,"ErrorGuessingFormat",include_bytes!("../assets/format_error.png"))
            },

            Message::ImageLoaded(Err(ImageError::ErrorDecodingImage(key))) => {
                self.handle_error_task(key,"ErrorDecodingImage",include_bytes!("../assets/decode_error.png"))
            },


            Message::ImageLoaded(Err(e)) => {
                cprintln!("Message::ImageLoaded {:?}",e);
                Task::none()
            },

            Message::ImageLoaded( Ok(mut ls) ) => {
                let key     = ls.id;

                let handle  = match ls.handle { Some(ref h) => h.clone(), None => { return Task::none(); } } ;

                // handle is held by the LruCache
                ls.handle = None;

                self.loaded_image_info.insert(ls.id,ls);

                iced_image::allocate(handle).map(move |alloc| { Message::ImageCached(key,alloc) } )
            },

            Message::ImageCached(k,Ok(a) ) => {
                // cprintln!("~[c255]{:?},~[c51]{k:x}  {:?}",self.now-self.start,a.handle());
                let Some(_) = self.pending_image_requests.remove(&k) else {
                    // cprintln!("{k} missed   ");
                    return Task::none();
                };


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


            Message::FindFilesProgress(p) => {

                let list = match p {
                    ScanProgress::CurrentDir(d) => { self.current_scan_dir=d; return Task::none(); }
                    ScanProgress::SomeFiles(l)  => l,
                    // ScanProgress::More(d) => {
                    //     // cprintln!("~[c51]{d21}");
                    //     self.current_scan_dir=d.clone();
                    //     let (m,h) = Task::sip(
                    //         FileSystemHelper::find_files_sipper(vec![d],1),
                    //         Message::FindFilesProgress,
                    //         | _e | { Message::FileFindComplete }
                    //     ).abortable();
                    //     return m;
                    // }

                };

                if self.img_list.is_empty() && !list.is_empty() {

                    self.img_list.append(list);

                    let next_image = match self.img_list.first() {
                        Ok(i) => i,
                        Err(_) => { panic!(); }
                    };

                    self.goto_image_task( next_image )
                }
                else
                {
                    self.img_list.append(list);
                    Task::none()
                }
            },
            Message::FileFindComplete => {
                self.scan_dir_task = None;
                return self.preload_task(false);
            },

            Message::CancelFileFind => {
                match &self.scan_dir_task {
                    None => { },
                    Some(h) => {
                        h.abort();
                        self.scan_dir_task = None;
                    }
                }
                return self.preload_task(false);
            },


            Message::FindFilesOnPath => {
                let (m,h) = Task::sip(
                    FileSystemHelper::find_files_sipper(self.args.dirs.clone(),self.args.max_depth),
                    Message::FindFilesProgress,
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
                self.preload_task(false)
            }
            Message::SortSize => {
                let _ = self.img_list.sort_size();
                self.preload_task(false)
            }

            Message::SortDate => {
                let _ = self.img_list.sort_date();

                /* Sort by exif date
                let _ = self.img_list.sort_by( |a,b| {
                    let Some(aa) = self.loaded_image_info.get(&a) else { panic!(); };
                    let Some(bb) = self.loaded_image_info.get(&b) else { panic!(); };

                    let exifa = match &aa.exif { Some(e) => e, None => { panic!(); } };
                    let exifb = match &bb.exif { Some(e) => e, None => { panic!(); } };

                    QuickViewer::best_date_from_exif(&exifa).cmp(&QuickViewer::best_date_from_exif(&exifb))
                });
                */

                self.preload_task(false)
            }


            Message::Shuffle => {
                let _ = self.img_list.shuffle();

                self.pending_image_requests.drain();
                self.preload_task(false)
            }


            Message::RandomImage => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                } else {
                    let idx = match self.img_list.random() {
                        Ok(i) => i,
                        Err(_) => { panic!(); }
                    };
                    self.pending_image_requests.drain();
                    self.goto_image_task( idx )
                }
            }

            Message::PageUp => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                }
                else
                {
                    let idx = self.img_list.peek_range(-100..=-100).next().expect("1");

                    self.pending_image_requests.drain();
                    self.goto_image_task(idx)
                }
            }

            Message::PageDown => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                }
                else
                {
                    let idx = self.img_list.peek_range(100..=100).next().expect("1");

                    self.pending_image_requests.drain();
                    self.goto_image_task(idx)
                }
            }

            Message::Home => {
                let next_image = match self.img_list.first() {
                    Ok(i) => i,
                    Err(_) => { panic!(); }
                };

                self.pending_image_requests.drain();
                self.goto_image_task(next_image)
            },

            Message::End => {
                let next_image = match self.img_list.last() {
                    Ok(i) => i,
                    Err(_) => { panic!(); }
                };

                self.pending_image_requests.drain();
                self.goto_image_task( next_image )
            },

            Message::Swap => {
                self.zoom=!self.zoom;
                Task::none()
            }

            Message::Scrolled(ScrollDelta::Pixels{x: _, y: _}) => { Task::none() }
            Message::Scrolled(ScrollDelta::Lines{x:_,y}) => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                }
                else
                {
                    let delta:isize = -y as isize;
                    let idx   = self.img_list.peek_range(delta..=delta).next().expect("1");
                    self.pending_image_requests.drain();
                    self.goto_image_task(idx)
                }
            }

            Message::FileDropped(f) => {
                cprintln!("~[c58]{}",f.display());
                Task::none()
            }

            Message::Left => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                } else {
                    let Some(next_idx) = self.img_list.peek_range(-1..=-1).next() else {
                        return Task::none();
                    };

                    self.goto_image_task(next_idx)
                }
            }

            Message::Right => {

                if self.show_when_loaded.is_some() {
                    Task::none()
                } else {
                    let Some(next_idx) = self.img_list.peek_range(1..=1).next() else {
                        return Task::none();
                    };

                    if self.args.time_forward_loop {
                        if next_idx == ImageKey::new(1).expect("reality")
                        {
                            if !self.toasts.is_empty() {
                                self.toasts[0].message = format!("{:?}",now-self.loop_start);
                            }
                            cprintln!("{:?}",now-self.loop_start);
                            self.loop_start = Instant::now();
                        }
                    }

                    self.goto_image_task(next_idx)
                }
            }

            Message::Space => {
                self.args.slideshow = !self.args.slideshow;
                Task::none()
            },
        }
    }

    fn has_exif(&self) -> Option<&exif::Exif> {
        let key = match self.img_list.key() { Ok(k) => k,Err(_) => { return None; } };
        let ii  = match self.loaded_image_info.get(&key) { Some(ii) => ii, None => { return None;} };
        let exif = match &ii.exif { Some(e) => e, None => {return None;} };
        return Some(exif);
    }

    fn best_date_from_exif(exif:&exif::Exif) -> Option<String> {
        let mut v = Vec::new();

        match exif.get_field(exif::Tag::DateTime,exif::In::PRIMARY) {
            Some(d) => { v.push(d.display_value().to_string()) }
            None => {  }
        };
        match exif.get_field(exif::Tag::DateTimeOriginal,exif::In::PRIMARY) {
            Some(d) => { v.push(d.display_value().to_string()) }
            None => { }
        };
        match exif.get_field(exif::Tag::DateTimeDigitized,exif::In::PRIMARY) {
            Some(d) => { v.push(d.display_value().to_string()) }
            None => {  }
        };
        v.sort();

        if !v.is_empty() {

            return Some(v[0].clone());
        }
        None
    }

    fn best_date(&self) -> Option<String> {

        let exif = match self.has_exif() {
            Some(e) => e,
            None => {return None;}
        };

        QuickViewer::best_date_from_exif(exif)
    }


    fn view(&self) -> Element<'_, Message> {
        let c = match self.img_list.the_index() {
            Ok(i) => i.get(),
            Err(_) => 0
        };

        let t = match self.img_list.total_items() {
            Ok(i) => i.get(),
            Err(_) => 0
        };

        let fname = match self.img_list.item() {
            Ok(n)   => n.display(),
            Err(_)  => "".into()
        };
        let fqp = match self.img_list.item() {
            Ok(n)   => n.fqp().display().to_string(),
            Err(_)  => "".into()
        };

        let file_name =
                text(fname.clone())
                    .size(self.args.font_size)
                    .color(color!(0xFDFD96))
                    .wrapping(Wrapping::None);

        let file_name_over =
                container(
                    row![
                        button( text("fqp").size(self.args.font_size-2).color(color!(0x000000)) )
                            .padding([0,5]).height(iced::Length::Fill).on_press(Message::Clip(fqp)),
                        button( text("fn").size(self.args.font_size-2).color(color!(0x000000)) )
                            .padding([0,5]).height(iced::Length::Fill).on_press(Message::Clip(fname))
                    ].spacing(5)
                );

        let file_name = hover( file_name, file_name_over );


        let image_size = match self.img_list.item() {
            Ok(n) =>    format!("{:<10}",humansize::format_size( n.size(), humansize::DECIMAL )),
            Err(_) =>   format!("{:<10}","")
        };

        let image_size = text(image_size).size(self.args.font_size).color(color!(0xa368a8)).font(Font::MONOSPACE);

        let image_dim = match self.img_list.key() {
            Ok(key) => {
                match self.loaded_image_info.get(&key) {
                    Some(i) =>  format!("{: >6} x {: <6}",num(i.dimensions.width),num(i.dimensions.height)),
                    None    =>  format!("{0:>6}   {0:>6}","")
                }
            },
            Err(_)  =>          format!("{0:>6}   {0:>6}","")
        };

        let image_dim = text(image_dim).size(self.args.font_size).color(color!(0xFD5E53)).font(Font::MONOSPACE);

        let image_dt = match self.best_date() {
            Some(d) => (d,color!(0xE1A95F)),
            None => {
                let format = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
                let y = match self.img_list.item() {
                    Ok(i)   => format!("{}",i.ftime().format(&format).unwrap()) ,
                    Err(_)  => "0000-00-00 00:00:00".into()
                };

                (y,color!(0xafafaf))
            }
        };

        let image_dt = text(image_dt.0).size(self.args.font_size).color(image_dt.1);//.font(Font::MONOSPACE);

        let clr = match self.show_when_loaded {
            None    => { color!(0xF5F5F5)  }
            Some(_) => { color!(0xFF00FF)  }
        };


        let scan_dir_progress = if self.scan_dir_task.is_some() {
            container(
                container(
                    row![
                        button( text("stop").size(max(self.args.font_size,12)-2 )).padding([0,2]).height(iced::Length::Shrink).on_press(Message::CancelFileFind),
                        container(
                            text(self.current_scan_dir.clone())
                                .size(max(self.args.font_size,12)-2)
                                .color(color!(0xFFFFFF))
                                .width(iced::Length::Fill)
                                .height(iced::Length::Fill)
                                .align_x(text::Alignment::Left)
                                .align_y(Vertical::Center)
                                .wrapping(Wrapping::None)
                        )
                        .width(Length::Shrink)

                        ,
                    ].spacing(10).padding([0,10]).height(iced::Length::Shrink).width(iced::Length::Fill)
                )
                .style( |_| {
                    CStyle {
                        background: Some(iced::Background::Color(iced::Color::from_rgba8(0, 0, 0,0.65))),
                        ..CStyle::default()
                    }
                })
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(text::Alignment::Left)
            .align_y(Vertical::Bottom)

        } else { container( row![] ) };

        // Shows the pending cache load
        let cache_status = if self.pending_image_requests.len() > 0 {
            let c = self.pending_image_requests.len();
            let b = (self.args.look_ahead + self.args.look_behind) as f32;
            container(
                progress_bar(0.0..=b,b-c as f32)
                    .length(50)
                    .style( |theme:&Theme| {
                        let t = theme.palette();

                        PBStyle{
                            background: Background::Color(t.primary     ),
                            bar:        Background::Color(t.background  ),
                            border:     border::color(iced::Color::BLACK.scale_alpha(0.0)).width(5.5),
                        }
                    } )
                )
                .width(Length::Fill)
                .height(15)
                .align_x(text::Alignment::Right)
                .align_y(Vertical::Bottom)
        }
        else { container( row![] ) };



        let counter =
        if self.args.font_size > 0 {
        if t == 0 {
            row![ text!("No images").size(self.args.font_size).color(color!(0xC5B358)) ].height(20)
        } else {
            row![
                row!(
                    text!("{:>12}", num(c))
                        .size(self.args.font_size)
                        .color(clr)
                        .font(Font::MONOSPACE),
                    text!("/")
                        .size(self.args.font_size)
                        .color(color!(0xafafaf))
                        .font(Font::MONOSPACE),
                    text!("{:<12}", num(t))
                        .size(self.args.font_size)
                        .color(color!(0x536878))
                        .font(Font::MONOSPACE)
                ),
                container( image_size ).padding([0,5]),
                container( image_dim ) .padding([0,5]),
                container( image_dt )  .padding([0,5]),
                container( file_name)  .padding([0,5]),
            ].height(iced::Length::Shrink)
        }
        }
        else{
            row![]
        };

        // EXIF info table
        type Tbl<'a> = Container<'a, Message, Theme, Renderer>;
        let exif_info:Tbl =
        if self.args.view_exif {
            match self.has_exif() {
                None      => { container( row![]) },
                Some(exf) => {
                    container(
                    scrollable(
                    table(
                        [
                            // table::column(text!("").height(1), |f:&exif::Field| text!("{}",f.ifd_num).size(self.args.font_size).color(color!(0x7f7f7f)) ),
                            table::column(text!("").height(1), |f:&exif::Field| text!("{}",f.tag    ).size(max(self.args.font_size,10)).color(color!(0xafafaf)) ),
                            table::column(text!("").height(1), |f:&exif::Field| {
                                use exif::Tag;
                                let d = match f.tag {
                                    Tag::MakerNote                  |
                                    Tag::UserComment                |
                                    Tag(exif::Context::Tiff,700)    |
                                    Tag(exif::Context::Tiff,50341)  |
                                    Tag(exif::Context::Tiff,50898)  |
                                    Tag(exif::Context::Tiff,50899)  |
                                    Tag(exif::Context::Tiff,59932)  |
                                    Tag(exif::Context::Exif,59932)  =>
                                        (color!(0xff7f7f),format!("{}...",f.display_value().with_unit(f).to_string()[..40].to_string())),

                                    Tag::ImageDescription           =>
                                        (color!(0xFF8040),f.display_value().with_unit(f).to_string()),

                                    Tag::DateTime                   |
                                    Tag::DateTimeOriginal           |
                                    Tag::DateTimeDigitized          =>
                                        (color!(0xFFFFD0),f.display_value().with_unit(f).to_string()),
                                    _                               =>
                                        (color!(0xbfbfbf),f.display_value().with_unit(f).to_string()),
                                };


                                let d = if d.1.len() > 80 {
                                    (color!(0xff1f1f), format!("{:80.80}...",d.1) )
                                }
                                else
                                {
                                    d
                                };

                                mouse_area(text!("{}",d.1).color(d.0).size(max(self.args.font_size,10))).on_press(Message::Clip(f.display_value().with_unit(f).to_string()))
                            })
                        ],
                        &mut exf.fields()
                    ) // table
                    .padding_x(10)
                    .padding_y(2)
                    .separator_x(0)
                    .separator_y(0)
                    )// scrollable
                    )// container
                    .style( |_| {
                        CStyle {
                            background: Some(iced::Background::Color(iced::Color::from_rgba8(0, 0, 0,0.65))),
                            ..CStyle::default()
                        }
                    })
                }
            }
        }
        else { container(row![]) };


        let cache_load_display = if self.args.view_cache_look_ahead {
            if self.pending_image_requests.len() > 0 {
                container(
                    container(
                        column(
                            self.pending_image_requests.iter().map(|item| {
                                let ii = match self.img_list.item_from_key(item.1.key) {
                                    Ok(i) => format!("{i}"),
                                    Err(e) => format!("{:?}",e)
                                };

                                text!("  {:x} {}  ",item.1.key,ii)
                                    .font(Font::MONOSPACE)
                                    .into()
                            })
                        )
                    )
                    .style( |_| {
                        CStyle {
                            background: Some(iced::Background::Color(iced::Color::from_rgba8(0, 0, 0,0.25))),
                            ..CStyle::default()
                        }
                    })
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Right)
                .align_y(Vertical::Bottom)
            }
            else
            {
                container( text("  idle  ") )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Vertical::Bottom)
                .align_x(Horizontal::Right)

            } }
        else
        {
            container(row![])
        };

        let h = match self.current_image_handle.clone() {
            Some(i) => i.clone(),
            None => self.empty_image.clone()
        };

        let img = if self.zoom {
            container( viewer(h).width(Fill).height(Fill) )
        }
        else {
            container( iced_image(h).width(Fill).height(Fill) )
        };
        // let img = container(canvas(self).width(Fill).height(Fill));

        let content =
            stack![
                column![
                    stack![
                        img,
                        float( cache_load_display ),
                        float( exif_info ),
                        float( scan_dir_progress ),
                    ],
                    container(counter).width(Fill),
                ],
                float(
                    container(cache_status)
                        .width(Fill)
                        .height(Fill)
                        .align_y(Vertical::Bottom)
                        .align_x(Horizontal::Right)
                )
            ]
        ;

        let stuff = if true {
            container( toast::Manager::new(content, &self.toasts, Message::ByeToaster)
                .timeout(5) )
        }
        else {
            container(content)
        };


        mouse_area(
            center(stuff).width(Fill).height(Fill)
        )
        .on_press(Message::Left)
        .on_right_press(Message::Right)
        .on_middle_press(Message::Swap)
        .on_scroll(|delta| { Message::Scrolled(delta) } )
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        use keyboard::Event as EV;
        use keyboard::Key as KK;
        use keyboard::key::Named as KN;


        let mut s = vec![
            keyboard::listen().filter_map(|event|

            match event {
                EV::KeyPressed { key: KK::Named(key), ..} => match key {

                    // KN::ArrowUp      => Some(Message::Up),
                    KN::ArrowLeft    => Some(Message::Left),
                    // KN::ArrowDown    => Some(Message::Down),
                    KN::ArrowRight   => Some(Message::Right),
                    KN::PageDown     => Some(Message::PageDown),
                    KN::PageUp       => Some(Message::PageUp),
                    KN::Space        => Some(Message::Space),
                    KN::Home         => Some(Message::Home),
                    KN::End          => Some(Message::End),
                    KN::Escape       => Some(Message::Quit),
                    KN::F11          => Some(Message::FullScreenToggle),
                    // KN::Alt          => { cprintln!("{:?}",event); None },
                    // a  => { cprintln!("~[c197]{a:?}"); None }
                    _ => None,
                },


                EV::KeyPressed { text: Some(ref v), modifiers,.. }
                    if  modifiers == keyboard::Modifiers::SHIFT ||
                        modifiers == keyboard::Modifiers::NONE      => match v.as_ref() {
                    "A" => { cprintln!("A"); None },
                    "!" => { cprintln!("!"); None },
                    "1" => { cprintln!("{v} "); None },
                    "f" => Some(Message::FullScreenToggle),
                    "q" => Some(Message::Quit),
                    "s" => Some(Message::Sort),
                    "S" => Some(Message::SortSize),
                    "d" => Some(Message::SortDate),
                    "h" => Some(Message::Shuffle),
                    "r" => Some(Message::RandomImage),
                    "[" => Some(Message::DecDelay),
                    "]" => Some(Message::IncDelay),
                     // a  => { cprintln!("~[c197]{a}"); None }
                     _  => None,
                },

                EV::KeyPressed { key: KK::Character(ref key), modifiers: keyboard::Modifiers::ALT, text:Some(_text),..} => match key.as_ref() {
                    "w" => {cprintln!("Alt W"); None },
                    "1" => {cprintln!("Alt 1"); None },
                    "d" => { Some(Message::LookAheadDisplayToggle) }
                    "e" => { Some(Message::ExifDisplayToggle) }
                     // a  => { cprintln!("~[c197]Alt {text}"); None }
                     _  => None,
                },
                EV::KeyPressed { key: KK::Character(ref key), modifiers: keyboard::Modifiers::CTRL, ..} => match key.as_ref() {
                    "w" => {cprintln!("Ctrl w"); None },
                    "-" => { Some(Message::FontDown) },
                    "+" => { Some(Message::FontUp) },
                    "=" => { Some(Message::FontUp) },
                     // a  => { cprintln!("~[c197]Ctrl {key}"); None }
                     _  => None,
                },
                EV::KeyPressed { key: KK::Character(ref key), modifiers: keyboard::Modifiers::SHIFT, ..} => match key.as_ref() {
                    "w" => {cprintln!("W {event:?}"); None },
                    "a" => {cprintln!("a {event:?}"); None },
                    "1" => {cprintln!("1 {event:?}"); None },
                     // a  => { cprintln!("4: {key:?}"); None }
                     _  => None,
                },


                // EV::KeyPressed { key: KK::Character(ref key), ..} => match key.as_ref() {
                //      // a  => { cprintln!("4: {event:?}"); None }
                //      _  => None,
                // },
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
        // Theme::Moonfly
        // Theme::Oxocarbon
        // Theme::Ferra
        // Theme::Dracula
        Theme::TokyoNight
        // Theme::KanagawaWave
        // Theme::Nightfly
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

