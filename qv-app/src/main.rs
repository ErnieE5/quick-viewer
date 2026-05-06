// #![allow(unused_imports)]
use ee_conio::{cprintln};
use ee_viewer::{QuickViewer,QVConfig,QVMsg,RenderMode,ScanProgress,FileSystemHelper};



mod args;

const MIN_DELAY: u64 = 5;

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
    WindowEvent( (Id,Event) ),
    FullScreenToggle,

    FileDropped(PathBuf),
    FindFilesOnPath,
    FindFilesProgress(ScanProgress),
    FileFindComplete ,
    CancelFileFind,


    Quit,
    Goodbye,
    Qv(QVMsg),
}

#[derive(Debug)]
struct App {
    args:                       Args,
    fullscreen:                 bool,
    qv:                         QuickViewer,
    scan_dir_task:              Option<TaskHandle>,
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
            fullscreen:             false,
            current_scan_dir:       "".into(),
            scan_dir_task:          None,

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

                self.scan_dir_task = Some(h);

                m
            }


            Msg::FullScreenToggle => {
                use iced::window::{self,Mode};

                let mode = if self.fullscreen  {
                    self.fullscreen = false; Mode::Windowed

                } else {
                    self.fullscreen = true;  Mode::Fullscreen
                };

                window::latest().and_then(move |id| window::set_mode(id, mode))
            }

            Msg::FindFilesProgress(p) => {

                let list = match p {
                    ScanProgress::CurrentDir(d) => { self.current_scan_dir=d; return Task::none(); }
                    ScanProgress::SomeFiles(l)  => l,
                };

                self.qv.update( QVMsg::AddFiles(list), now ).map(Msg::Qv)
            },

            Msg::FileFindComplete => {
                self.scan_dir_task = None;
                self.qv.update( QVMsg::UpdateCache, now ).map(Msg::Qv)
            },

            Msg::CancelFileFind => {
                match &self.scan_dir_task {
                    None => { },
                    Some(h) => {
                        h.abort();
                        self.scan_dir_task = None;
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

                self.scan_dir_task = Some(h);

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

            Msg::Goodbye  => {
                iced::exit()
            },
        }
    }

    fn view(&self) -> Element<'_, Msg> {

        // let scan_dir_progress = if self.scan_dir_task.is_some() {
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
        use keyboard::Event         as EV;
        use keyboard::Key           as KK;
        use keyboard::key::Named    as KN;
        use iced_core::keyboard::key::Physical as KP;

        let qvh = |qvm| Some(Msg::Qv(qvm));

        let m = vec![
            self.qv.subscription().map(Msg::Qv),

            keyboard::listen().filter_map(move |event|
                match event {
                    EV::KeyPressed { key: KK::Named(key), physical_key:KP::Code(_physical_key), ..} => match key {

                        KN::ArrowLeft    => qvh(QVMsg::Left),
                        KN::ArrowRight   => qvh(QVMsg::Right),
                        KN::PageDown     => qvh(QVMsg::PageDown),
                        KN::PageUp       => qvh(QVMsg::PageUp),
                        KN::Space        => qvh(QVMsg::Space),
                        KN::Home         => qvh(QVMsg::Home),
                        KN::End          => qvh(QVMsg::End),
                        KN::Escape       => Some(Msg::Quit),
                        KN::F11          => Some(Msg::FullScreenToggle),
                        // KN::Alt          => { cprintln!("{:?}",physical_key); None },
                        // a  => { cprintln!("{:?}",physical_key); None }
                        _ => None,
                    },


                    EV::KeyPressed { text: Some(ref v), modifiers,.. }
                        if  modifiers == keyboard::Modifiers::SHIFT ||
                            modifiers == keyboard::Modifiers::NONE      => match v.as_ref() {
                        "A" => { cprintln!("A"); None },
                        "!" => { cprintln!("!"); None },
                        "1" => { cprintln!("{v} "); None },
                        "f" => Some(Msg::FullScreenToggle),
                        "q" => Some(Msg::Quit),
                        "s" => qvh(QVMsg::Sort),
                        "S" => qvh(QVMsg::SortSize),
                        "d" => qvh(QVMsg::SortDate),
                        "h" => qvh(QVMsg::Shuffle),
                        "r" => qvh(QVMsg::RandomImage),
                        "[" => qvh(QVMsg::DecDelay),
                        "]" => qvh(QVMsg::IncDelay),
                        "?" => Some(Msg::Huh),
                        // a  => { cprintln!("~[c197]{a}"); None }
                        _  => None,
                    },

                    EV::KeyPressed { key: KK::Character(ref key), modifiers: keyboard::Modifiers::ALT, text:Some(_text),..} => match key.as_ref() {
                        "w" => {cprintln!("Alt W"); None },
                        "1" => {cprintln!("Alt 1"); None },
                        "d" => { qvh(QVMsg::LookAheadDisplayToggle) }
                        "e" => { qvh(QVMsg::ExifDisplayToggle) }
                        "s" => { qvh(QVMsg::SlideModeToggle) }
                        // a  => { cprintln!("~[c197]Alt {text}"); None }
                        _  => None,
                    },
                    EV::KeyPressed { key: KK::Character(ref key), modifiers: keyboard::Modifiers::CTRL, ..} => match key.as_ref() {
                        "w" => {cprintln!("Ctrl w"); None },
                        "-" => { qvh(QVMsg::FontDown) },
                        "+" => { qvh(QVMsg::FontUp) },
                        "=" => { qvh(QVMsg::FontUp) },
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

