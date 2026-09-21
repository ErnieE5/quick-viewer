// #![allow(unused_imports)]
use ee_conio::{cprintln};
use ee_viewer::{QuickViewer,QVConfig,QVMsg,RenderMode,SipProgress,FileSystemHelper,ImageKey};

mod args;
#[rustfmt::skip]    // column-aligned geometry tables live in this module
mod geometry;

#[rustfmt::skip]    // column-aligned binding table lives in this module
mod keyboard_bindings;

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

use image::{
    self,
    ImageFormat
};

use iced::task::Handle as TaskHandle;
use std::path::PathBuf;



#[derive(Debug, Clone)]
#[rustfmt::skip]
enum Msg {

    Welcome,
    Huh,
    AltRightTest,       // TEST CODE: NamedMod chord exercise, prints "hi" — safe to remove
    ModifierStub(&'static str),     // stub for bare modifier presses — fill with real actions later
    WindowEvent( (Id,Event) ),
    FullScreenToggle,
    BorderlessToggle,

    #[allow(dead_code)]     // stub: handler is Task::none(), nothing sends it yet
    GetImageHandle(ImageKey,QVMsg),
    #[allow(dead_code)]     // stub: handler is Task::none(), nothing sends it yet
    GetImageAlloc(ImageKey,QVMsg),

    FileDropped(PathBuf),
    FindFilesOnPath,
    FindFilesProgress(SipProgress),
    FileFindComplete ,
    #[allow(dead_code)]     // live handler; its sender, the "stop" button in view(), is commented out
    CancelFileFind,


    Quit,
    Goodbye,
    Qv(QVMsg),
}

#[derive(Debug)]
#[rustfmt::skip]
struct App {
    args:                       Args,
    fullscreen:                 bool,
    qv:                         QuickViewer,
    sip_dir_task:               Option<TaskHandle>,
    current_scan_dir:           String,
}

#[rustfmt::skip]
impl App {
    fn from_args(args: Args, fullscreen: bool) -> Self {

        let mut config = QVConfig::default();

        config.dirs                 = args.dirs.clone();
        config.look_ahead           = args.look_ahead;
        config.look_behind          = args.look_behind;
        config.cache_size           = args.cache_size;
        config.no_empty_cat         = args.no_splash;
        config.slideshow            = args.slideshow;
        config.slide                = !args.no_slide;
        config.slide_ms             = args.slide_ms;
        config.font_size            = args.font_size;
        config.view_exif            = args.view_exif;
        config.delay                = args.delay;
        config.max_depth            = args.max_depth;
        config.time_forward_loop    = args.time_forward_loop;
        config.primary_render       = if args.no_canvas { RenderMode::Image } else { RenderMode::Canvas };

        Self {
            qv:                     QuickViewer::new(config),
            fullscreen,
            current_scan_dir:       "".into(),
            sip_dir_task:           None,

            args
        }
    }

    fn new(args: Args, fullscreen: bool) -> (Self, Task<Msg>) {

        let m = vec![
            Task::done( Msg::Welcome ),
            Task::done( Msg::Qv( QVMsg::Welcome ) ),
            Task::done( Msg::FindFilesOnPath ),
        ];

        //  The window is born full screen when --fs asks for it (window::Settings
        //  carries the flag), so there is nothing to toggle here and no windowed
        //  frame to flash.
        //
        //  `fullscreen` is what the window actually IS, not what was asked for:
        //  --fs --screen N and --span are covering borderless windows rather than
        //  a winit fullscreen, and F11 has to know the difference or its first
        //  press tries to leave a mode we were never in.
        let me = App::from_args(args, fullscreen);

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

            Msg::ModifierStub(name) => {
                cprintln!("~[c245]mod: {name}");
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

            Msg::BorderlessToggle => {
                use iced::window;

                window::latest().and_then(window::toggle_decorations)
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

            keyboard::listen().filter_map( |event| keyboard_bindings::lookup(&event) ),

            iced::window::events().map( Msg::WindowEvent ),
        ];
        Subscription::batch( m )
    }
}


//
//  Stop libheif scanning the root of the current drive for plugins.
//
//  This build of libheif has no plugin directory compiled in, so the empty path plus its
//  own "\*.dll" resolves to the root of whatever drive is current: C:\appverifUI.dll and
//  C:\vfcompat.dll are 32-bit Application Verifier DLLs, and LoadLibraryA on either from
//  a 64-bit process fails with 193 (ERROR_BAD_EXE_FORMAT). libheif prints
//  "LoadLibraryA error: 193" and moves on -- harmless, but it re-inits per decode, so a
//  directory of HEICs fills the console with it. Run from a drive whose root holds no
//  DLLs and it never appears; that is the whole difference.
//
//  All decoders here are built in, so there are no plugins to find. Point it at a path
//  that holds none. A path set by hand wins: this only fills the blank.
//
#[cfg(all(windows, feature = "heif"))]
fn quiet_heif_plugin_scan() {
    if std::env::var_os("LIBHEIF_PLUGIN_PATH").is_some() { return; }

    let Ok(exe) = std::env::current_exe() else { return; };
    let Some(dir) = exe.parent() else { return; };

    let path = dir.join("heif-plugins");

    //  SAFETY: single-threaded, at the very top of main, before anything reads the
    //  environment -- no other thread exists to race the write.
    unsafe { std::env::set_var("LIBHEIF_PLUGIN_PATH", &path); }

    //  ...and again through the CRT, which is the copy that matters here. Rust's set_var
    //  is SetEnvironmentVariableW; libheif asks getenv(), and the CRT answers from its own
    //  environment block, built at process start and NOT updated by the Win32 call. Set
    //  only one of the two and the scan still happens.
    {
        use std::ffi::CString;
        use std::os::raw::{c_char,c_int};

        unsafe extern "C" {
            fn _putenv_s(name: *const c_char, value: *const c_char) -> c_int;
        }

        let Ok(name) = CString::new("LIBHEIF_PLUGIN_PATH") else { return; };
        let Ok(val)  = CString::new(path.to_string_lossy().as_bytes()) else { return; };

        //  SAFETY: two valid, NUL-terminated C strings; _putenv_s copies both.
        unsafe { _putenv_s(name.as_ptr(), val.as_ptr()); }
    }
}

pub fn main() -> IcedResult {
    #[cfg(all(windows, feature = "heif"))]
    quiet_heif_plugin_scan();

    #[cfg(feature = "heif")]
    libheif_rs::integration::image::register_all_decoding_hooks();

    //  Before anything -- winit included -- looks at a monitor. See geometry.rs.
    geometry::become_dpi_aware();

    //  Parsed here rather than inside App::new, because window::Settings is consumed
    //  before any App exists -- see geometry.rs for why that matters.
    let args = args::do_args();

    if args.list_screens {
        geometry::print_screens();
        return Ok(());
    }

    let resolved = geometry::resolve(&args);

    let settings = geometry::apply(
        Settings {
            transparent:true,
            icon: Some(icon::from_file_data(include_bytes!("../../assets/icon.png"),Some(ImageFormat::Png)).expect("1")),
            ..Settings::default()
        },
        &resolved,
        args.overlay,
    );

    application::timed(
        move || App::new(args.clone(), resolved.fullscreen),
        App::update,
        App::subscription,
        App::view
    )
    .window(settings)
    .theme(Theme::TokyoNight)
    .title("Quick Viewer")
    .run()
}

