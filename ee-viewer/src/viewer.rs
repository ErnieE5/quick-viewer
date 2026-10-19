
use ee_conio::{cprintln};


const MIN_DELAY: u64 = 1;

use crate::toast::{self,Toast};
// use crate::args;


use crate::img_traits::{ImageError,ImageDyn};
use crate::file_traits::{LoadData};
use crate::file_system_helper::{FileSystemHelper};

use crate::img_list::{ImageList};
use crate::strip::{Strip};
use crate::scale::{ScaleMode};
use crate::cross::{Cross,Motion};


use iced::mouse::{ ScrollDelta };
use iced::time::{Duration,Instant};
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
    Padding,
    Renderer,
    border,
    color,
    clipboard,
    alignment::Vertical,
    alignment::Horizontal,
};

use std::collections::{ HashMap };
// use std::path::PathBuf;
use std::cmp::max;

use lru::LruCache;
use std::num::NonZeroUsize as ImageKey;

use iced::advanced::image::Allocation as ImageAllocation;
use iced::advanced::image::Error      as AllocError;

#[derive(Debug, Clone, PartialEq)]
pub enum SlideMode {
    Forward,
    Reverse,
    Random,
}

impl SlideMode {
    #[allow(dead_code)]     // Alt+S cycled it; the last move decides the direction now, as in ee_feh
    pub fn next(&self) -> Self {
        match self {
            SlideMode::Forward => SlideMode::Reverse,
            SlideMode::Reverse => SlideMode::Random,
            SlideMode::Random  => SlideMode::Forward,
        }
    }
}

#[derive(Debug, Clone,Copy)]
pub enum RenderMode {
    Viewer,
    Image,
    Canvas,
}

#[derive(Debug, Clone)]
pub enum ImageCacheItem {
    Alloc(ImageAllocation,Duration),
    Handle(ImageHandle),
}

#[derive(Debug, Clone)]
pub enum QVMsg {
    Welcome,

    //  The splash, decoded off-thread. See load_splash().
    SplashLoaded(ImageHandle),

    Left,
    Right,
    Slide(Instant),
    Frame(Instant),     // a display frame, subscribed to only while the strip slides or a cross runs
    ScaleFactor(f32),   // the window's DPI scale: 1.5 at 150%. What 1:1 is measured in.

    DecDelay,
    IncDelay,

    FontUp,
    FontDown,

    LookAheadDisplayToggle,
    ExifDisplayToggle,
    ScaleModeNext,
    ScaleModePrev,
    MotionNext,
    MotionPrev,

    Space,
    Sort,
    SortSize,
    SortDate,
    Shuffle,
    Swap,
    Scrolled(ScrollDelta),
    RandomImage,
    PageDown,
    PageUp,
    Home,
    End,
    Clip(String),
    ClipFqp(ImageKey),
    #[allow(unused)]
    ClipResult(Result<(), std::fmt::Error>),
    ByeToaster(usize),

    AddFiles(Vec<Box<dyn ImageDyn>>),

    RequestAnImage(ImageKey),
    ImageLoaded( Result<LoadData, ImageError>),
    ImageCached( ImageKey, Instant, Result<ImageAllocation,AllocError> ),

    UpdateCache,

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
pub struct QVConfig
{
    pub look_ahead:         isize,
    pub look_behind:        isize,
    pub cache_size:         usize,
    pub no_empty_cat:       bool,
    pub slideshow:          bool,
    pub font_size:          u32,
    pub view_exif:          bool,
    pub delay:              u64,
    pub dirs:               Vec<String>,
    pub max_depth:          usize,
    pub time_forward_loop:  bool,
    pub primary_render:     RenderMode,
    pub slide_ms:           u64,
    pub motion:             Motion,
    pub fade_ms:            u64,
    pub wipe_ms:            u64,
    pub scale_mode:         ScaleMode,
    pub surface:            iced::Color,    // what is behind the canvas; a cross fades the uncovered parts to it
}


impl QVConfig {
    pub fn default() -> Self {
        Self {
            look_ahead:         5,
            look_behind:        5,
            cache_size:         10,
            no_empty_cat:       false,
            slideshow:          false,
            font_size:          12,
            view_exif:          false,
            delay:              1000,
            dirs:               Vec::new(),
            max_depth:          10000,
            time_forward_loop:  false,
            primary_render:     RenderMode::Canvas,
            slide_ms:           crate::strip::SLIDE_MS,
            motion:             Motion::Wipe,
            fade_ms:            crate::cross::FADE_MS,
            wipe_ms:            crate::cross::WIPE_MS,
            scale_mode:         ScaleMode::Fit,
            surface:            iced::Color::BLACK,
        }
    }
}


#[derive(Debug)]
pub struct QuickViewer {
    config:                 QVConfig,
    render_mode:            RenderMode,
    #[allow(unused)]
    start:                  Instant,
    loop_start:             Instant,
    now:                    Instant,
    img_list:               ImageList,




    toasts: Vec<Toast>,

    view_cache_look_ahead:      bool,
    show_when_loaded:           Option<ImageKey>,
    current_image_handle:       Option<ImageHandle>,
    loaded_image_info:          HashMap<ImageKey,LoadData>,
    image_cache:                LruCache< ImageKey, ImageCacheItem >,
    pending_image_requests:     HashMap<ImageKey,CacheData>,

    zoom:                       bool,
    slide_mode:                 SlideMode,
    strip:                      Strip,

    motion:                     Motion,
    cross:                      Cross,
    departed:                   Option<ImageHandle>,        // what a cross is crossing from
    cross_when_loaded:          Option<(ImageHandle,bool)>, // a cross waiting on its arriving image: (departed, forward)
    scale_mode:                 ScaleMode,
    scale_factor:               f32,
    surface:                    ImageHandle,                // 1x1 of config.surface; images draw in order, fills do not

    empty_image:                ImageHandle,
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

//  How long to leave jasper alone for. Long enough that a directory with images in it
//  has already put one on screen, so the decode never competes with the one decode that
//  matters. Nothing is waiting on this: the window is already up and drawing.
const SPLASH_DELAY_MS: u64 = 250;

//
//  The splash, off the startup path and out of the way.
//
//  whd_from_asset! is a full PNG decode plus a to_rgba8 copy -- 23.4ms for jasper,
//  measured -- and it used to run inside QuickViewer::new, where nothing could proceed
//  until it finished. That put it in front of the first frame every single launch.
//
//  Moving it to a worker thread alone was not enough. It then raced the first real
//  image for CPU and cost first-image ~14ms, which is the wrong trade for a viewer: it
//  bought a window sooner and paid with the picture later. So it waits first.
//
//  The reasoning behind waiting: if a real image turns up promptly then the splash was
//  never going to be seen, and if the list is empty -- the case it exists for -- nothing
//  is competing and it arrives right after the delay. Either way it is transparent until
//  then, which is a window that is up and honest rather than one that is blocked.
//
//  spawn_blocking because this is CPU-bound, not IO-bound, and saying so keeps it off
//  the async executor as well as off the render path.
//
async fn load_splash() -> ImageHandle {
    tokio::time::sleep( Duration::from_millis(SPLASH_DELAY_MS) ).await;

    tokio::task::spawn_blocking(|| {
        let (w,h,d) = whd_from_asset!("../../assets/jasper.png");
        ImageHandle::from_rgba(w, h, d)
    })
    .await
    .expect("reality")
}



impl QuickViewer {
    pub fn new(config: QVConfig) -> Self {
        Self {
            image_cache: LruCache::new(ImageKey::new(config.cache_size).unwrap()),

            render_mode:  config.primary_render,

            //  Jasper is 1355x2169 of PNG and cost 23.4ms to decode here, on the
            //  way to the first frame, every launch. He arrives later via
            //  SplashLoaded; until then this 1x1 zero-opacity pixel stands in,
            //  which is also exactly what --ns leaves in place permanently.
            empty_image: ImageHandle::from_rgba(1, 1, vec![0,0,0,0]),

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
            view_cache_look_ahead:  false,
            show_when_loaded:       None,
            pending_image_requests: HashMap::new(),

            zoom:                   false,
            slide_mode:             SlideMode::Forward,
            strip:                  Strip::new(config.slide_ms),

            motion:                 config.motion,
            cross:                  Cross::new(),
            departed:               None,
            cross_when_loaded:      None,
            scale_mode:             config.scale_mode,
            scale_factor:           1.0,
            surface:                ImageHandle::from_rgba(1, 1, config.surface.into_rgba8().to_vec()),

            config
        }
    }

    fn showit(&mut self) -> bool {
        let key = match self.img_list.key() {
            Ok(k) => k,
            Err(_) => { return false; }
        };

        match self.image_cache.get(&key) {
            None => false,
            Some(ImageCacheItem::Handle(h)) => {
                if Some(h) != self.current_image_handle.as_ref() {
                    self.current_image_handle   = Some(h.clone());
                    self.show_when_loaded       = None;
                }
                true
            },
            Some(ImageCacheItem::Alloc(a,_d)) => {
                if Some(a.handle()) != self.current_image_handle.as_ref() {
                    self.current_image_handle   = Some(a.handle().clone());
                    self.show_when_loaded       = None;
                }
                true
            }
        }
    }

    fn goto_image_task(&mut self, pos:usize) -> Task<QVMsg> {
        if self.img_list.is_empty() { return Task::none(); }

        let key = match self.img_list.key_at(pos) {
            Ok(k)   => k,
            Err(e)  => { cprintln!("~[c196]{e:?} ~[c255]{pos}"); return Task::none(); }
        };

        match self.img_list.goto(pos) {
            Ok(i)   => { assert_eq!(pos,i); },
            Err(e)  => { cprintln!("~[c196]{e:?} ~[c255]{pos}"); return Task::none(); }
        };

        //  Every move lands by default. move_task opts in to a motion after this returns.
        self.strip.jump();
        self.cross_when_loaded = None;

        if !self.showit() {
            self.show_when_loaded = Some(key);
        }

        self.preload_task(false)
    }

    //  Left/Right and the scroll wheel: a move of `step` along the list. The direction
    //  sticks -- the slideshow carries on the way the last step went, as ee_feh's does.
    fn step_task(&mut self, step:isize, now:Instant) -> Task<QVMsg> {
        let Some(idx) = self.img_list.peek_range(step..=step).next() else {
            return Task::none();
        };

        self.slide_mode = if step < 0 { SlideMode::Reverse } else { SlideMode::Forward };

        self.move_task(idx, Some(step), now)
    }

    //  A goto by the current motion. The goto lands (see goto_image_task); then a slide
    //  poses the strip back over the old image and eases it onto the new one, and a fade
    //  or wipe crosses from the old image to the new. `step` is how far along the list
    //  this went, when that is a step at all: a slide needs one, a jump (Home, random)
    //  only crosses, and a cross sweeps the way the step went.
    fn move_task(&mut self, pos:usize, step:Option<isize>, now:Instant) -> Task<QVMsg> {
        let carry   = self.strip.offset();      // goto lands the strip; a slide resumes from here
        let before  = self.current_image_handle.clone();
        let from    = self.img_list.pos().ok();

        let t = self.goto_image_task(pos);

        if !matches!(self.render_mode, RenderMode::Canvas) || self.img_list.len() < 2 || from == Some(pos) {
            return t;
        }

        let forward = match step {
            Some(s) => s >= 0,
            None    => from.is_none_or(|f| pos >= f),
        };

        match self.motion {
            Motion::Instant => {
                self.cross.land();
                self.departed = None;
            }

            //  Only onto an image that is already up. While it is still loading the canvas
            //  is showing the old one, and sliding that in from the side would be a lie.
            Motion::Slide => {
                self.cross.land();
                self.departed = None;
                if let Some(s) = step && self.show_when_loaded.is_none() {
                    self.strip.go_from(carry, s as i32, now);
                }
            }

            //  The cross starts when the arriving image is up: now, or when it loads.
            Motion::Fade | Motion::Wipe => {
                if let Some(b) = before {
                    if self.show_when_loaded.is_none() {
                        self.cross_task(b, forward, now);
                    } else {
                        self.cross_when_loaded = Some((b, forward));
                    }
                }
            }
        }

        t
    }

    fn cross_task(&mut self, before:ImageHandle, forward:bool, now:Instant) {
        let ms = match self.motion { Motion::Fade => self.config.fade_ms, _ => self.config.wipe_ms };

        //  Mid-cross the departed image stays the departed image; the mix runs on.
        if !self.cross.go(ms, forward, now) || self.departed.is_none() {
            self.departed = Some(before);
        }
    }

    //  The image show_when_loaded was waiting for is up: start the cross that waited on it.
    fn arrived(&mut self, now:Instant) {
        if let Some((b,f)) = self.cross_when_loaded.take() {
            self.cross_task(b, f, now);
        }
    }

    //  Toast `s`, replacing whatever toast is up rather than stacking under it.
    fn say(&mut self, s:String) {
        if !self.toasts.is_empty() {
            self.toasts[0].message = s;
        }
        else
        {
            self.toasts.push( Toast { message: s } );
        }
    }

    //  The handle for the image `slot` places from the current one, if it is cached.
    //  peek, not get: drawing a neighbour must not reorder the LRU.
    fn slot_handle(&self, slot:i32) -> Option<&ImageHandle> {
        if slot == 0 {
            return self.current_image_handle.as_ref();
        }
        let pos = self.img_list.peek_range(slot as isize..=slot as isize).next()?;
        let key = self.img_list.key_at(pos).ok()?;
        match self.image_cache.peek(&key)? {
            ImageCacheItem::Handle(h)       => Some(h),
            ImageCacheItem::Alloc(a,_d)     => Some(a.handle()),
        }
    }

    fn preload_task(&mut self,only_one:bool) -> Task<QVMsg> {

        if self.img_list.is_empty() { return Task::none(); }

        let mut m: Vec<Task<QVMsg>> = vec![ ];

        // If it isn't already in the cache, request that it be loaded in
        // anticipation that we will want it soon.
        let mut add_to_batch = |key| {
            if self.image_cache.get(&key).is_none() {
                if !self.pending_image_requests.contains_key(&key) {
                    match self.pending_image_requests.insert(key,CacheData::new(key)) {
                        None => { m.push(Task::done(QVMsg::RequestAnImage(key))); }
                        Some(_) => { panic!(); }
                    }
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
            for i in il.peek_range(-1*self.config.look_behind..=self.config.look_ahead) {
                match il.key_at(i) {
                    Ok(k) => add_to_batch(k),
                    Err(_) => todo!(),
                };
            }
        }

        Task::batch(m)
    }

    fn handle_error_task(&mut self,key:ImageKey,t:&str,i:&[u8]) -> Task<QVMsg> {

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

            if self.config.slideshow {
                self.config.slideshow = false;
            }

            return Task::none();

        }

        Task::none()
    }


    pub fn update(&mut self, event: QVMsg, now: Instant) -> Task<QVMsg> {
        self.now = now;
        // cprintln!("~[c7]{:10.4?}    {:80.80}",now-self.start,format!("{:?}",event) );
        match event {
            QVMsg::Welcome          => {
                if self.config.no_empty_cat { Task::none() }
                else { Task::perform( load_splash(), QVMsg::SplashLoaded ) }
            }

            QVMsg::SplashLoaded(h)  => { self.empty_image = h; Task::none() }

            QVMsg::ClipFqp(key)     => {
                match self.img_list.item_from_key(key) {
                    Ok(i)  => clipboard::write( i.fqp().display().to_string() ),
                    Err(_) => { return Task::none(); }
                }
            },
            QVMsg::Clip(s)          => { clipboard::write(s) },
            QVMsg::ClipResult(_)    => { Task::none() },
            // QVMsg::ClipResult(Ok(_))  => { Task::none() },
            // QVMsg::ClipResult(Err(e)) => { cprintln!("{e:?}"); Task::none() },


            QVMsg::ByeToaster(idx) => {
                self.toasts.remove(idx);
                Task::none()
            },


            QVMsg::FontDown => {
                if self.config.font_size > 6
                {
                    self.config.font_size -= 1;
                }
                else
                {
                    self.config.font_size = 0;
                }
                Task::none()
            },
            QVMsg::FontUp => {
                if self.config.font_size == 0 {
                    self.config.font_size = 6;
                }
                else if self.config.font_size < 50
                {
                    self.config.font_size += 1;
                }
                Task::none()
            },

            QVMsg::LookAheadDisplayToggle => {
                self.view_cache_look_ahead = !self.view_cache_look_ahead;
                Task::none()
            },

            QVMsg::ExifDisplayToggle => {
                self.config.view_exif = !self.config.view_exif;
                Task::none()
            }

            QVMsg::ScaleModeNext | QVMsg::ScaleModePrev => {
                self.scale_mode = if matches!(event, QVMsg::ScaleModeNext) { self.scale_mode.next() } else { self.scale_mode.prev() };
                self.say( self.scale_mode.name().into() );
                cprintln!("~[c240]scale {}", self.scale_mode.name());
                Task::none()
            }

            QVMsg::MotionNext | QVMsg::MotionPrev => {
                self.motion = if matches!(event, QVMsg::MotionNext) { self.motion.next() } else { self.motion.prev() };

                //  Each motion keeps its own pace, so cycling onto the fade does not run it at the wipe's.
                let ms = match self.motion {
                    Motion::Fade    => Some(self.config.fade_ms),
                    Motion::Wipe    => Some(self.config.wipe_ms),
                    Motion::Slide   => Some(self.config.slide_ms),
                    Motion::Instant => None,
                };
                let said = match ms { Some(ms) => format!("{} {ms} ms", self.motion.name()), None => self.motion.name().into() };

                self.say( said.clone() );
                cprintln!("~[c240]motion {said}");
                Task::none()
            }

            QVMsg::ScaleFactor(f) => {
                self.scale_factor = if f > 0.0 { f } else { 1.0 };
                cprintln!("~[c240]scale factor {}", self.scale_factor);
                Task::none()
            }

            QVMsg::IncDelay =>      {
                self.config.delay += 5;

                self.config.delay = if self.config.delay%5 == 0 { self.config.delay } else {
                    (self.config.delay/5)*5
                };

                if self.toasts.len() > 0 {
                    self.toasts[0].message = self.config.delay.to_string();
                }
                else
                {
                    self.toasts.push( Toast { message: self.config.delay.to_string() } );
                }
                Task::none()
            },

            QVMsg::DecDelay =>    {
                if self.config.delay > MIN_DELAY+5 {
                    self.config.delay -= 5;
                }
                else {
                    self.config.delay = MIN_DELAY;
                }
                self.config.delay = if self.config.delay%5 == 0 { self.config.delay } else {
                    ((self.config.delay+1)/5)*5
                };

                if self.toasts.len() > 0 {
                    self.toasts[0].message = self.config.delay.to_string();
                }
                else
                {
                    self.toasts.push( Toast { message: self.config.delay.to_string() } );
                }

                Task::none()
            },


            QVMsg::RequestAnImage(key) => {
                // cprintln!("RAI ~[c61]{:x}",key);

                let Some(r) = self.pending_image_requests.get_mut(&key) else {
                    cprintln!("RAI ~[c196]{:x}",key);
                    return Task::none();
                };

                match self.img_list.item_from_key(key) {
                    Ok(ic) => {
                        let (m,h) = Task::perform(FileSystemHelper::load_image(ic.fqp(), key), QVMsg::ImageLoaded).abortable();

                        r.request = Some(h.abort_on_drop());

                        m
                    },

                    Err(x) => {
                        cprintln!("~[c197]{x:?}");
                        Task::none()
                    }
                }
            }

            QVMsg::ImageLoaded(Err(ImageError::ErrorOpeningImageFile(key))) => {
                self.handle_error_task(key,"ErrorOpeningImageFile",include_bytes!("../../assets/open_error.png"))
            },

            QVMsg::ImageLoaded(Err(ImageError::ErrorReadingImageFile(key))) => {
                self.handle_error_task(key,"ErrorReadingImageFile",include_bytes!("../../assets/read_error.png"))
            },

            QVMsg::ImageLoaded(Err(ImageError::ErrorGuessingFormat(key))) => {
                self.handle_error_task(key,"ErrorGuessingFormat",include_bytes!("../../assets/format_error.png"))
            },

            QVMsg::ImageLoaded(Err(ImageError::ErrorDecodingImage(key))) => {
                self.handle_error_task(key,"ErrorDecodingImage",include_bytes!("../../assets/decode_error.png"))
            },


            QVMsg::ImageLoaded(Err(e)) => {
                cprintln!("QVMsg::ImageLoaded {:?}",e);
                Task::none()
            },

            QVMsg::ImageLoaded( Ok(mut ls) ) => {
                let key     = ls.id;

                let handle  = match ls.handle { Some(ref h) => h.clone(), None => { return Task::none(); } } ;

                // handle is held by the LruCache
                ls.handle = None;

                self.loaded_image_info.insert(ls.id,ls);

                match self.render_mode {
                    RenderMode::Canvas => {
                        self.image_cache.push(key, ImageCacheItem::Handle(handle));

                        let Some(_) = self.pending_image_requests.remove(&key) else {
                            // cprintln!("{k} missed   ");
                            return Task::none();
                        };

                        if Some(key) == self.show_when_loaded {

                            let _ = match self.img_list.find_index(key)
                            {
                                Ok(i) => match self.img_list.goto(i) { Ok(i) => i, Err(_) => { panic!(); } },
                                Err(_) => { panic!(); }
                            };

                            if self.showit() {
                                self.show_when_loaded = None;
                                self.arrived(now);
                            }
                        }
                        Task::none()
                    },
                    RenderMode::Image |
                    RenderMode::Viewer => {
                        iced_image::allocate(handle).map(move |alloc| { QVMsg::ImageCached(key,now,alloc) } )
                    }
                }

                //


            },

            QVMsg::ImageCached(k,f,Ok(a) ) => {
                // cprintln!("~[c255]{:?},~[c51]{k:x}  {:?}",self.now-self.start,a.handle());
                let Some(_) = self.pending_image_requests.remove(&k) else {
                    // cprintln!("{k} missed   ");
                    return Task::none();
                };

                self.image_cache.push( k, ImageCacheItem::Alloc(a,Instant::now()-f));

                if Some(k) == self.show_when_loaded {

                    let _ = match self.img_list.find_index(k)
                    {
                        Ok(i) => match self.img_list.goto(i) { Ok(i) => i, Err(_) => { panic!(); } },
                        Err(_) => { panic!(); }
                    };

                    if self.showit() {
                        self.show_when_loaded = None;
                        self.arrived(now);
                    }
                }

                Task::none()
            }

            QVMsg::ImageCached( _k,_f,Err(_e) ) => {
                // cprintln!("~[c255]{:?},QVMsg::ImageCached {k:x} -- {e:?}",self.now-self.start);
                Task::none()
            }

            QVMsg::UpdateCache => {
                self.preload_task(false)
            }

            QVMsg::AddFiles(list) => {

                if self.img_list.is_empty() && !list.is_empty() {

                    self.img_list.append(list);

                    let next_image = self.img_list.first().expect("at least one new item appended to list");

                    self.goto_image_task( next_image )
                }
                else
                {
                    self.img_list.append(list);
                    Task::none()
                }

            }

            QVMsg::Sort => {
                let _ = self.img_list.sort();
                self.preload_task(false)
            }
            QVMsg::SortSize => {
                let _ = self.img_list.sort_size();
                self.preload_task(false)
            }

            QVMsg::SortDate => {
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


            QVMsg::Shuffle => {
                let _ = self.img_list.shuffle();

                self.pending_image_requests.drain();
                self.preload_task(false)
            }


            QVMsg::RandomImage => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                } else {
                    let next_image = match self.img_list.random() {
                        Ok(i)   => i,
                        Err(_)  => { return Task::none(); }
                    };
                    self.slide_mode = SlideMode::Random;     // the slideshow carries on the way the last move went
                    self.pending_image_requests.drain();
                    self.move_task( next_image, None, now )
                }
            }

            QVMsg::PageUp => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                }
                else
                {
                    let idx = match self.img_list.peek_range(-100..=-100).next() {
                        Some(i) => i,
                        None    => { return Task::none(); }
                    };

                    self.pending_image_requests.drain();
                    self.move_task(idx, None, now)
                }
            }

            QVMsg::PageDown => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                }
                else
                {
                    let idx = match self.img_list.peek_range(100..=100).next() {
                        Some(i) => i,
                        None    => { return Task::none(); }
                    };

                    self.pending_image_requests.drain();
                    self.move_task(idx, None, now)
                }
            }

            QVMsg::Home => {
                let next_image = match self.img_list.first() {
                    Ok(i) => i,
                    Err(_) => { return Task::none(); }
                };

                self.pending_image_requests.drain();
                self.move_task(next_image, None, now)
            },

            QVMsg::End => {
                let next_image = match self.img_list.last() {
                    Ok(i) => i,
                    Err(_) => { return Task::none(); }
                };

                self.pending_image_requests.drain();
                self.move_task( next_image, None, now )
            },

            QVMsg::Swap => {
                self.zoom=!self.zoom;
                if self.zoom {
                    self.render_mode = RenderMode::Viewer;
                }
                else {
                    self.render_mode = self.config.primary_render;
                }
                self.image_cache = LruCache::new(ImageKey::new(self.config.cache_size).unwrap());

                self.preload_task(false)
            }

            QVMsg::Scrolled(ScrollDelta::Pixels{x: _, y: _}) => { Task::none() }
            QVMsg::Scrolled(ScrollDelta::Lines{x:_,y}) => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                }
                else
                {
                    let delta:isize = -y as isize;

                    if delta == 0 { return Task::none(); }

                    self.pending_image_requests.drain();
                    self.step_task(delta, now)
                }
            }

            QVMsg::Left => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                } else {
                    self.step_task(-1, now)
                }
            }

            QVMsg::Frame(at) => {
                self.strip.tick(at);
                self.cross.tick(at);
                if !self.cross.active() {
                    self.departed = None;
                }
                Task::none()
            }

            QVMsg::Slide(_tick) => {
                if self.show_when_loaded.is_some() {
                    Task::none()
                } else {
                    let step = match self.slide_mode {
                        SlideMode::Forward => Some( 1),
                        SlideMode::Reverse => Some(-1),
                        SlideMode::Random  => None,
                    };

                    let Some(next_idx) = (match self.slide_mode {
                        SlideMode::Forward => { self.img_list.peek_range( 1..= 1).next() },
                        SlideMode::Reverse => { self.img_list.peek_range(-1..=-1).next() },
                        SlideMode::Random  => {
                            self.pending_image_requests.drain();
                            match self.img_list.random() {
                                Ok(i) => Some(i),
                                Err(_) => { return Task::none(); }
                            }
                        },
                    })
                    else {
                        return Task::none();
                    };

                    if self.config.time_forward_loop && self.slide_mode == SlideMode::Forward {

                        if next_idx == 0
                        {
                            cprintln!("{:?}",now-self.loop_start);
                            self.loop_start = Instant::now();
                            self.config.time_forward_loop = false;
                            return iced::exit();
                        }
                    }

                    self.move_task(next_idx, step, now)
                }
            }

            QVMsg::Right => {

                if self.show_when_loaded.is_some() {
                    Task::none()
                } else {
                    self.step_task(1, now)
                }
            }

            QVMsg::Space => {
                self.config.slideshow = !self.config.slideshow;
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


    pub fn view(&self) -> Element<'_, QVMsg> {

        let view_start = Instant::now();

        // display is 1-based; this +1 is the only one allowed outside img_list
        let c = match self.img_list.pos() {
            Ok(i) => i+1,
            Err(_) => 0
        };

        let t = self.img_list.len();

        let (_origin,group,fname) = match self.img_list.item() {
            Ok(n)   => (n.origin(),n.group(),n.name()),
            Err(_)  => ("".into(),"".into(),"".into())
        };

        let key = match self.img_list.key() {
            Ok(k) => k,
            Err(_) => ImageKey::MIN
        };

        let prefix = if group.len() > 0 {
                    row![
                        text(group)
                            .size(self.config.font_size)
                            .color(color!(0xaDaD96))
                            .wrapping(Wrapping::None),
                        text("/")
                            .color(color!(0xffffff))
                            .size(self.config.font_size)
                    ]
        }
        else
        {
            row![]
        };


        let file_name =
                container(
                    row![
                        prefix,
                        text(fname)
                            .size(self.config.font_size)
                            .color(color!(0xFDFD96))
                            .wrapping(Wrapping::None)
                    ]
                );

        let file_name_over =
                container(
                    row![
                        button( text("fqp").size(max(self.config.font_size,10)).color(color!(0x000000)) )
                            .padding([0,5]).height(iced::Length::Fill).on_press(QVMsg::ClipFqp(key)),
                        button( text("fn").size(max(self.config.font_size,10)).color(color!(0x000000)) )
                            .padding([0,5]).height(iced::Length::Fill).on_press(QVMsg::Clip(fname.to_string()))
                    ].spacing(5)
                );

        let file_name = hover( file_name, file_name_over );


        let image_size = match self.img_list.item() {
            Ok(n) =>    format!("{:<10}",humansize::format_size( n.size(), humansize::DECIMAL )),
            Err(_) =>   format!("{:<10}","")
        };

        let image_size = text(image_size).size(self.config.font_size).color(color!(0xa368a8)).font(Font::MONOSPACE);

        let image_dim = match key {
            ImageKey::MIN =>          format!("{0:>6}***{0:>6}",""),
            key => {
                match self.loaded_image_info.get(&key) {
                    Some(i) =>  format!("{: >6} x {: <6}",num(i.dimensions.width),num(i.dimensions.height)),
                    None    =>  format!("{0:>6}!!!{0:>6}","")
                }
            },

        };

        let image_dim = text(image_dim)
                            .wrapping(Wrapping::None)
                            .size(self.config.font_size)
                            .color(color!(0xFD5E53))
                            .font(Font::MONOSPACE);

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

        let image_dt = text(image_dt.0)
                        .wrapping(Wrapping::None)
                        .size(self.config.font_size)
                        .color(image_dt.1);

        let clr = match self.show_when_loaded {
            None    => { color!(0xF5F5F5)  }
            Some(_) => { color!(0xFF00FF)  }
        };

        // Shows the pending cache load
        let cache_status = if self.pending_image_requests.len() > 0 {
            let c = self.pending_image_requests.len();
            let b = (self.config.look_ahead + self.config.look_behind) as f32;
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
        if self.config.font_size > 0 {
        if t == 0 {
            row![ text!("No images").size(self.config.font_size).color(color!(0xC5B358)) ].height(20)
        } else {
            row![
                row!(
                    text!("{:>12}", num(c))
                        .size(self.config.font_size)
                        .color(clr)
                        .font(Font::MONOSPACE),
                    text!("/")
                        .size(self.config.font_size)
                        .color(color!(0xafafaf))
                        .font(Font::MONOSPACE),
                    text!("{:<12}", num(t))
                        .size(self.config.font_size)
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
        type Tbl<'a> = Container<'a, QVMsg, Theme, Renderer>;
        let exif_info:Tbl =
        if self.config.view_exif {
            match self.has_exif() {
                None      => { container( row![]) },
                Some(exf) => {
                    container(
                    scrollable(
                    table(
                        [
                            // table::column(text!("").height(1), |f:&exif::Field| text!("{}",f.ifd_num).size(self.config.font_size).color(color!(0x7f7f7f)) ),
                            table::column(text!("").height(1), |f:&exif::Field| text!("{}",f.tag    ).size(max(self.config.font_size,10)).color(color!(0xafafaf)) ),
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
                                        (color!(0xff7f7f),format!("{}...",f.display_value().with_unit(f).to_string().chars().take(40).collect::<String>())),

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

                                mouse_area(text!("{}",d.1).color(d.0).size(max(self.config.font_size,10))).on_press(QVMsg::Clip(f.display_value().with_unit(f).to_string()))
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


        let cache_load_display = if self.view_cache_look_ahead {
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
                            } )
                        )
                        .padding( Padding::default().bottom( std::cmp::min(self.config.font_size,20) as f32 ) )
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

        let img = match self.render_mode {
            RenderMode::Viewer => container( viewer(h)      .width(Fill).height(Fill) ),
            RenderMode::Image  => container( iced_image(h)  .width(Fill).height(Fill) ),
            RenderMode::Canvas => container( canvas(self)   .width(Fill).height(Fill) ),
        };

        let content =
            stack![
                column![
                    stack![
                        img,
                        float( exif_info ),
                        float( cache_load_display ),
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
            container( toast::Manager::new(content, &self.toasts, QVMsg::ByeToaster)
                .timeout(5) )
        }
        else {
            container(content)
        };


        let ret = mouse_area(
            center(stuff).width(Fill).height(Fill)
        )
        .on_press(QVMsg::Left)
        .on_right_press(QVMsg::Right)
        .on_middle_press(QVMsg::Swap)
        .on_scroll(|delta| { QVMsg::Scrolled(delta) } )
        .into();

        let view_elapsed = view_start.elapsed();
        if view_elapsed > Duration::from_micros(500) {
            ee_conio::cprintln!("~[C196 c227]{:>10.3?}  view time warning~[XK]",view_elapsed);
        }

        ret
    }

    pub fn subscription(&self) -> Subscription<QVMsg> {
        let mut s = vec![
        ];

        use iced::time;
        if self.config.slideshow {
            s.push( time::every(time::Duration::from_millis(self.config.delay)).map(QVMsg::Slide) );
        }

        //  The frame clock runs only while something is moving: an idle viewer pays nothing.
        if self.strip.active() || self.cross.active() {
            s.push( iced::window::frames().map(QVMsg::Frame) );
        }

        Subscription::batch(s)
    }

}

use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::{ Program, Frame };

use iced::{ Rectangle, Size, Point, };
use iced_core::image::Renderer as CoreRenderer;
use iced_core::image::Image as CoreImage;

//  A wipe's soft edge, as a share of the window's width -- ee_feh's _feather -- and how
//  many steps of opacity draw it. The canvas has no gradient mask for an image, so the
//  ramp is bands, each clipped and drawn at its own opacity.
const FEATHER:  f32 = 0.12;
const BANDS:    usize = 24;

impl QuickViewer {

    //  The logical size of one physical pixel: what 1:1 is measured in.
    fn px(&self) -> f32 { 1.0 / self.scale_factor }

    //  Where an image of w x h goes, by the current scale mode. See scale.rs.
    fn place(&self, size: Size, w: f32, h: f32) -> Rectangle {
        self.scale_mode.place(size, w, h, self.px())
    }

    //  The splash is always fitted, whatever the mode.
    fn fit(&self, size: Size, w: f32, h: f32) -> Rectangle {
        ScaleMode::Fit.place(size, w, h, self.px())
    }

    //  The arriving image of a cross, `a` of the way in, inside `clip`. The window it
    //  does not cover goes to the surface by the same amount, so where the two images'
    //  shapes differ the old one fades out rather than popping off at the end.
    fn arriving(&self, frame: &mut Frame, window: Rectangle, clip: Rectangle, a: f32, r: Rectangle, han: &ImageHandle) {
        let Some(clip) = clip.intersection(&window) else { return; };

        let (ww, wh)    = (window.width, window.height);
        let top         = r.y.clamp(0.0, wh);
        let bottom      = (r.y + r.height).clamp(0.0, wh);
        let left        = r.x.clamp(0.0, ww);
        let right       = (r.x + r.width).clamp(0.0, ww);

        let around = [
            Rectangle::new( Point::new(0.0,   0.0   ), Size::new(ww,         top         ) ),
            Rectangle::new( Point::new(0.0,   bottom), Size::new(ww,         wh - bottom ) ),
            Rectangle::new( Point::new(0.0,   top   ), Size::new(left,       bottom - top) ),
            Rectangle::new( Point::new(right, top   ), Size::new(ww - right, bottom - top) ),
        ];

        frame.with_clip( clip, |frame| {
            for g in around {
                if g.width > 0.0 && g.height > 0.0 {
                    frame.draw_image( g, CoreImage::new(self.surface.clone()).opacity(a) );
                }
            }
            frame.draw_image( r, CoreImage::new(han.clone()).opacity(a) );
        });
    }
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

        let size    = bounds.size();
        let window  = Rectangle::new(Point::ORIGIN, size);

        //  Mid-slide: the current image and its neighbour, each placed in its own slot and
        //  laid side by side (see strip.rs). The canvas does not clip, so this does -- each
        //  slot to itself, since Fill and 1:1 can be wider than a slot.
        //  At rest the strip is 0..=0 at offset 0 and this is skipped for the plain draw.
        if self.strip.active() {
            let dx   = self.strip.offset();

            for slot in self.strip.slots() {
                let Some(han) = self.slot_handle(slot) else { continue; };

                if renderer.load_image(han).is_err() { continue; }

                let Some(g) = renderer.measure_image(han) else { continue; };

                let shift   = (slot as f32 + dx) * size.width;
                let mut r   = self.place(size, g.width as f32, g.height as f32);
                r.x += shift;

                let Some(clip) = Rectangle::new(Point::new(shift, 0.0), size).intersection(&window) else { continue; };

                frame.with_clip( clip, |frame| { frame.draw_image( r, han ); });
            }

            return vec![frame.into_geometry()];
        }

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

            let r = self.place(size, w, h);

            //  Mid-cross: the image it left, in full, and the arriving one over it. See cross.rs.
            let departed = match (&self.departed, self.cross.active()) {
                (Some(d), true) if renderer.load_image(d).is_ok() => renderer.measure_image(d).map(|g| (d, g)),
                _ => None,
            };

            match departed {
                None => {
                    frame.with_clip( window, |frame| { frame.draw_image( r, &han ); });
                }

                Some((d, g)) => {
                    let dr = self.place(size, g.width as f32, g.height as f32);
                    frame.with_clip( window, |frame| { frame.draw_image( dr, d ); });

                    let mix = self.cross.mix();

                    if self.motion != Motion::Wipe {
                        self.arriving(&mut frame, window, window, mix, r, &han);
                    } else {
                        //  A soft edge sweeping the way the slide would have gone: forward,
                        //  the new image is to the right on the strip, so it comes in from the
                        //  right; back, from the left. The edge starts wholly off one side and
                        //  ends wholly off the other.
                        let ww = size.width;
                        let f  = ww * FEATHER;
                        let fw = f / BANDS as f32;

                        //  `solid` is where the arriving image is wholly in; the ramp is the
                        //  feather beside it, from `ramp` for one feather, fading to nothing
                        //  away from the solid side.
                        let (solid, ramp, forward) = if self.cross.forward() {
                            let e = ww + f - mix * (ww + 2.0 * f);
                            ( Rectangle::new(Point::new(e, 0.0), Size::new((ww - e).max(0.0), size.height)), e - f, true )
                        } else {
                            let e = mix * (ww + 2.0 * f) - f;
                            ( Rectangle::new(Point::new(0.0, 0.0), Size::new(e.max(0.0), size.height)), e, false )
                        };

                        self.arriving(&mut frame, window, solid, 1.0, r, &han);

                        for i in 0..BANDS {
                            let x = ramp + i as f32 * fw;
                            //  Opacity at the middle of the band: 1 beside the solid, 0 at the far end.
                            let t = (i as f32 + 0.5) / BANDS as f32;
                            let a = if forward { t } else { 1.0 - t };
                            let band = Rectangle::new(Point::new(x, 0.0), Size::new(fw, size.height));
                            self.arriving(&mut frame, window, band, a, r, &han);
                        }
                    }
                }
            }
        } else {

            let (w, h) = match renderer.measure_image(&self.empty_image) {
                Some(g) => (g.width as f32, g.height as f32),
                None    => (0.0, 0.0),
            };

            frame.draw_image( self.fit(size, w , h), &self.empty_image.clone());
        }

        vec![frame.into_geometry()]
    }
}


