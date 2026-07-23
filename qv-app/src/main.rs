// #![allow(unused_imports)]
use ee_conio::{cprintln};
use ee_viewer::{QuickViewer,QVConfig,QVMsg,RenderMode,SipProgress,FileSystemHelper,ImageKey};

mod args;

const MIN_DELAY: u64 = 1;

use crate::args::{Args};

use iced::time::Instant;

use iced::widget::{
    Theme,
    container,
    row,
};

use iced::{
    application,
    keyboard,
    Element,
    Subscription,
    Task,
    window::{Id,Event,Settings,icon},
    Result as IcedResult
};

use iced::keyboard::{Key as KK, Modifiers, key::Named as KN};
use iced::mouse::{ScrollDelta};
use iced_core::keyboard::key::Physical as KP;

use image::{
    self,
    ImageFormat
};

use iced::task::Handle as TaskHandle;
use std::path::PathBuf;



#[derive(Debug, Clone)]
enum Msg {

    Welcome,
    Huh,
    AltRightTest,       // TEST CODE: NamedMod chord exercise, prints "hi" — safe to remove
    WindowEvent( (Id,Event) ),
    FullScreenToggle,

    GetImageHandle(ImageKey,QVMsg),
    GetImageAlloc(ImageKey,QVMsg),

    FileDropped(PathBuf),
    FindFilesOnPath,
    FindFilesProgress(SipProgress),
    FileFindComplete ,
    CancelFileFind,


    Quit,
    Goodbye,
    Qv(QVMsg),
}

//
// Keyboard/mouse binding table. Dispatch (App::subscription) and any help
// display are both derived from this one list, so they can't drift apart.
// Every chord is exact — a key event fires a binding only if it matches the
// key AND the modifier set, so nothing dispatches by fall-through. First
// match still wins, but shapes are disjoint enough that order is cosmetic.
//
#[derive(Debug)]
enum Chord {
    Named(KN),                          // named key + physical code; NO modifiers
    NamedMod(KN, Modifiers),            // named key + exact modifier set
    Text(&'static str),                 // text from a Character key; NONE or SHIFT only
    Char(&'static str, Modifiers),      // Character key + exact modifier set
    Mouse,                              // display only, never matches a key event
}

#[allow(dead_code)] // label/group/help are consumed by the help panel
struct Binding {
    chord:  Chord,
    label:  &'static str,
    group:  &'static str,
    help:   &'static str,
    msg:    Msg,
}

use Chord::{Named,NamedMod,Text,Char,Mouse};

const BINDINGS: &[Binding] = &[
    // Navigation
    Binding{ chord:Named(KN::ArrowLeft),          label:"←",            group:"Navigation", help:"previous image",                     msg:Msg::Qv(QVMsg::Left)                    },
    // TEST CODE: exercises the NamedMod chord shape — safe to remove
    Binding{ chord:NamedMod(KN::ArrowRight,Modifiers::ALT),
                                                  label:"Alt+→",        group:"Navigation", help:"test binding (prints hi)",           msg:Msg::AltRightTest                       },
    Binding{ chord:Named(KN::ArrowRight),         label:"→",            group:"Navigation", help:"next image",                         msg:Msg::Qv(QVMsg::Right)                   },
    Binding{ chord:Named(KN::PageUp),             label:"PgUp",         group:"Navigation", help:"jump back 100",                      msg:Msg::Qv(QVMsg::PageUp)                  },
    Binding{ chord:Named(KN::PageDown),           label:"PgDn",         group:"Navigation", help:"jump forward 100",                   msg:Msg::Qv(QVMsg::PageDown)                },
    Binding{ chord:Named(KN::Home),               label:"Home",         group:"Navigation", help:"first image",                        msg:Msg::Qv(QVMsg::Home)                    },
    Binding{ chord:Named(KN::End),                label:"End",          group:"Navigation", help:"last image",                         msg:Msg::Qv(QVMsg::End)                     },
    Binding{ chord:Text("r"),                     label:"r",            group:"Navigation", help:"random image",                       msg:Msg::Qv(QVMsg::RandomImage)             },

    // Order
    Binding{ chord:Text("s"),                     label:"s",            group:"Order",      help:"sort by name",                       msg:Msg::Qv(QVMsg::Sort)                    },
    Binding{ chord:Text("S"),                     label:"S",            group:"Order",      help:"sort by file size",                  msg:Msg::Qv(QVMsg::SortSize)                },
    Binding{ chord:Text("d"),                     label:"d",            group:"Order",      help:"sort by file date",                  msg:Msg::Qv(QVMsg::SortDate)                },
    Binding{ chord:Text("h"),                     label:"h",            group:"Order",      help:"shuffle",                            msg:Msg::Qv(QVMsg::Shuffle)                 },

    // Slideshow
    Binding{ chord:Named(KN::Space),              label:"Space",        group:"Slideshow",  help:"toggle slideshow",                   msg:Msg::Qv(QVMsg::Space)                   },
    Binding{ chord:Text("["),                     label:"[",            group:"Slideshow",  help:"less delay (faster)",                msg:Msg::Qv(QVMsg::DecDelay)                },
    Binding{ chord:Text("]"),                     label:"]",            group:"Slideshow",  help:"more delay (slower)",                msg:Msg::Qv(QVMsg::IncDelay)                },
    Binding{ chord:Char("s",Modifiers::ALT),      label:"Alt+S",        group:"Slideshow",  help:"cycle direction fwd/rev/random",     msg:Msg::Qv(QVMsg::SlideModeToggle)         },

    // Display
    Binding{ chord:Named(KN::F11),                label:"F11",          group:"Display",    help:"toggle fullscreen",                  msg:Msg::FullScreenToggle                   },
    Binding{ chord:Text("f"),                     label:"f",            group:"Display",    help:"toggle fullscreen",                  msg:Msg::FullScreenToggle                   },
    Binding{ chord:Char("e",Modifiers::ALT),      label:"Alt+E",        group:"Display",    help:"toggle EXIF panel",                  msg:Msg::Qv(QVMsg::ExifDisplayToggle)       },
    Binding{ chord:Char("d",Modifiers::ALT),      label:"Alt+D",        group:"Display",    help:"toggle pending-load overlay",        msg:Msg::Qv(QVMsg::LookAheadDisplayToggle)  },
    Binding{ chord:Char("-",Modifiers::CTRL),     label:"Ctrl+-",       group:"Display",    help:"smaller UI text",                    msg:Msg::Qv(QVMsg::FontDown)                },
    Binding{ chord:Char("+",Modifiers::CTRL),     label:"Ctrl++",       group:"Display",    help:"larger UI text",                     msg:Msg::Qv(QVMsg::FontUp)                  },
    Binding{ chord:Char("=",Modifiers::CTRL),     label:"Ctrl+=",       group:"Display",    help:"larger UI text",                     msg:Msg::Qv(QVMsg::FontUp)                  },

    // App
    Binding{ chord:Named(KN::Escape),             label:"Esc",          group:"App",        help:"quit",                               msg:Msg::Quit                               },
    Binding{ chord:Text("q"),                     label:"q",            group:"App",        help:"quit",                               msg:Msg::Quit                               },
    Binding{ chord:Text("?"),                     label:"?",            group:"App",        help:"dump args (debug)",                  msg:Msg::Huh                                },

    // Mouse — display only; actual dispatch is the mouse_area in ee-viewer
    Binding{ chord:Mouse,                         label:"left click",   group:"Mouse",      help:"previous image",                     msg:Msg::Qv(QVMsg::Left)                    },
    Binding{ chord:Mouse,                         label:"right click",  group:"Mouse",      help:"next image",                         msg:Msg::Qv(QVMsg::Right)                   },
    Binding{ chord:Mouse,                         label:"middle click", group:"Mouse",      help:"toggle zoom (Viewer mode)",          msg:Msg::Qv(QVMsg::Swap)                    },
    Binding{ chord:Mouse,                         label:"scroll",       group:"Mouse",      help:"step images",                        msg:Msg::Qv(QVMsg::Scrolled(ScrollDelta::Lines{x:0.0,y:0.0})) },
];

impl Chord {
    fn matches(&self, event: &keyboard::Event) -> bool {
        use keyboard::Event as EV;

        match (self,event) {
            ( Named(n), EV::KeyPressed{ key:KK::Named(k), physical_key:KP::Code(_), modifiers, .. } )
                => k == n && modifiers.is_empty(),

            ( NamedMod(n,m), EV::KeyPressed{ key:KK::Named(k), physical_key:KP::Code(_), modifiers, .. } )
                => k == n && modifiers == m,

            ( Text(t), EV::KeyPressed{ key:KK::Character(_), text:Some(v), modifiers, .. } )
                if *modifiers == Modifiers::SHIFT || *modifiers == Modifiers::NONE
                => v.as_ref() == *t,

            ( Char(c,m), EV::KeyPressed{ key:KK::Character(k), modifiers, text, .. } )
                => modifiers == m
                && k.as_ref() == *c
                && ( *m != Modifiers::ALT || text.is_some() ),  // old ALT arm required text:Some(_)

            _ => false,
        }
    }
}

// The exploratory cprintln! arms from the old subscription match — not real
// bindings, just key discovery. Runs only when nothing in BINDINGS matched.
fn probe(event: &keyboard::Event) -> Option<Msg> {
    use keyboard::Event as EV;

    match event {
        EV::KeyPressed { text: Some(v), modifiers,.. }
            if  *modifiers == Modifiers::SHIFT ||
                *modifiers == Modifiers::NONE      => match v.as_ref() {
            "A" => { cprintln!("A"); None },
            "!" => { cprintln!("!"); None },
            "1" => { cprintln!("{v} "); None },
            // a  => { cprintln!("~[c197]{a}"); None }
            _  => None,
        },

        EV::KeyPressed { key: KK::Character(key), modifiers: Modifiers::ALT, text:Some(_text),..} => match key.as_ref() {
            "w" => {cprintln!("Alt W"); None },
            "1" => {cprintln!("Alt 1"); None },
            // a  => { cprintln!("~[c197]Alt {text}"); None }
            _  => None,
        },
        EV::KeyPressed { key: KK::Character(key), modifiers: Modifiers::CTRL, ..} => match key.as_ref() {
            "w" => {cprintln!("Ctrl w"); None },
            // a  => { cprintln!("~[c197]Ctrl {key}"); None }
            _  => None,
        },
        EV::KeyPressed { key: KK::Character(key), modifiers: Modifiers::SHIFT, ..} => match key.as_ref() {
            "w" => {cprintln!("W {event:?}"); None },
            "a" => {cprintln!("a {event:?}"); None },
            "1" => {cprintln!("1 {event:?}"); None },
            // a  => { cprintln!("4: {key:?}"); None }
            _  => None,
        },

        _ => None,
    }
}

#[derive(Debug)]
struct App {
    args:                       Args,
    fullscreen:                 bool,
    qv:                         QuickViewer,
    sip_dir_task:               Option<TaskHandle>,
    current_scan_dir:           String,
}

impl App {
    fn default() -> Self {
        let args = args::do_args();

        let mut config = QVConfig::default();

        config.dirs                 = args.dirs.clone();
        config.look_ahead           = args.look_ahead;
        config.look_behind          = args.look_behind;
        config.cache_size           = args.cache_size;
        config.no_empty_cat         = args.no_splash;
        config.slideshow            = args.slideshow;
        config.font_size            = args.font_size;
        config.view_exif            = args.view_exif;
        config.delay                = args.delay;
        config.max_depth            = args.max_depth;
        config.time_forward_loop    = args.time_forward_loop;
        config.primary_render       = if args.no_canvas { RenderMode::Image } else { RenderMode::Canvas };

        Self {
            qv:                     QuickViewer::new(config),
            fullscreen:             args.fullscreen,
            current_scan_dir:       "".into(),
            sip_dir_task:           None,

            args
        }
    }

    fn new() -> (Self, Task<Msg>) {

        let mut m = vec![
            Task::done( Msg::Welcome ),
            Task::done( Msg::Qv( QVMsg::Welcome ) ),
            Task::done( Msg::FindFilesOnPath ),
        ];

        let mut me = App::default();

        if me.fullscreen {
            me.fullscreen = false;
            m.push( Task::done(Msg::FullScreenToggle) );
        }

        ( me, Task::batch(m) )
    }

    fn update(&mut self, event: Msg, now: Instant) -> Task<Msg> {
        match event {
            Msg::Qv(m)      => { self.qv.update(m,now).map(Msg::Qv) },
            Msg::Welcome    => { Task::none() }
            Msg::Huh        => {
                cprintln!("{:#?}",self.args);
                Task::none()
            }

            Msg::AltRightTest => {     // TEST CODE — safe to remove
                cprintln!("hi");
                Task::none()
            }

            Msg::WindowEvent( (_id,Event::FileDropped(f)) ) => {
                Task::done(Msg::FileDropped(f))
            },
            Msg::WindowEvent( _ ) => { Task::none() },

            Msg::FileDropped(f) => {
                cprintln!("{:?}~[c51]{} ",now,f.display());

                let (m,h) = Task::sip(
                    FileSystemHelper::find_files_sipper(vec![f.display().to_string()],self.args.max_depth),
                    Msg::FindFilesProgress,
                    | _e | { Msg::FileFindComplete }
                ).abortable();

                self.sip_dir_task = Some(h);

                m
            }


            Msg::FullScreenToggle => {
                use iced::window::{self,Mode};

                self.fullscreen=!self.fullscreen;

                let mode = if self.fullscreen { Mode::Fullscreen } else { Mode::Windowed };

                window::latest().and_then(move |id| window::set_mode(id, mode) )
            }

            Msg::FindFilesProgress(p) => {

                let list = match p {
                    SipProgress::CurrentDir(d) => { self.current_scan_dir=d; return Task::none(); }
                    SipProgress::SomeFiles(l)  => l,
                };

                self.qv.update( QVMsg::AddFiles(list), now ).map(Msg::Qv)
            },

            Msg::FileFindComplete => {
                self.sip_dir_task = None;
                self.qv.update( QVMsg::UpdateCache, now ).map(Msg::Qv)
            },

            Msg::CancelFileFind => {
                match &self.sip_dir_task {
                    None => { },
                    Some(h) => {
                        h.abort();
                        self.sip_dir_task = None;
                    }
                }
                self.qv.update( QVMsg::UpdateCache, now ).map(Msg::Qv)
            },


            Msg::FindFilesOnPath => {
                let (m,h) = Task::sip(
                    FileSystemHelper::find_files_sipper(self.args.dirs.clone(),self.args.max_depth),
                    Msg::FindFilesProgress,
                    | _e | { Msg::FileFindComplete }
                ).abortable();

                self.sip_dir_task = Some(h);

                m
            },


            Msg::Quit => {
                use iced::window;

                let m: Vec<Task<Msg>> = vec![
                    window::latest().and_then(move |id| iced::window::minimize(id, true)),
                    Task::done(Msg::Goodbye),
                ];

                Task::batch(m)
            },

            Msg::GetImageHandle(_key,_msg) => { Task::none() },
            Msg::GetImageAlloc(_key,_msg) => { Task::none() },


            Msg::Goodbye  => {
                iced::exit()
            },
        }
    }

    fn view(&self) -> Element<'_, Msg> {

        // let scan_dir_progress = if self.sip_dir_task.is_some() {
        //     container(
        //         container(
        //             row![
        //                 button( text("stop").size(max(self.config.font_size,12)-2 )).padding([0,2]).height(iced::Length::Shrink).on_press(QVMsg::CancelFileFind),
        //                 container(
        //                     text(self.current_scan_dir.clone())
        //                         .size(max(self.config.font_size,12)-2)
        //                         .color(color!(0xFFFFFF))
        //                         .width(iced::Length::Fill)
        //                         .height(iced::Length::Fill)
        //                         .align_x(text::Alignment::Left)
        //                         .align_y(Vertical::Center)
        //                         .wrapping(Wrapping::None)
        //                 )
        //                 .width(Length::Shrink)

        //                 ,
        //             ].spacing(10).padding([0,10]).height(iced::Length::Shrink).width(iced::Length::Fill)
        //         )
        //         .style( |_| {
        //             CStyle {
        //                 background: Some(iced::Background::Color(iced::Color::from_rgba8(0, 0, 0,0.65))),
        //                 ..CStyle::default()
        //             }
        //         })
        //     )
        //     .width(Length::Fill)
        //     .height(Length::Fill)
        //     .align_x(text::Alignment::Left)
        //     .align_y(Vertical::Bottom)

        // } else { container( row![] ) };

        // float( scan_dir_progress ),


        container(
            row![
                self.qv.view().map(Msg::Qv)
            ]
        ).into()
    }

    fn subscription(&self) -> Subscription<Msg> {

        let m = vec![
            self.qv.subscription().map(Msg::Qv),

            keyboard::listen().filter_map( |event|
                match BINDINGS.iter().find( |b| b.chord.matches(&event) ) {
                    Some(b) => Some(b.msg.clone()),
                    None    => probe(&event),
                }
            ),

            iced::window::events().map( Msg::WindowEvent ),
        ];
        Subscription::batch( m )
    }
}


pub fn main() -> IcedResult {
    #[cfg(feature = "heif")]
    libheif_rs::integration::image::register_all_decoding_hooks();

    let settings = Settings {
        transparent:true,
        icon: Some(icon::from_file_data(include_bytes!("../../assets/icon.png"),Some(ImageFormat::Png)).expect("1")),
        ..Settings::default()
    };

    application::timed(
        App::new,
        App::update,
        App::subscription,
        App::view
    )
    .window(settings)
    .theme(Theme::TokyoNight)
    .title("Quick Viewer")
    .centered()
    .run()
}

