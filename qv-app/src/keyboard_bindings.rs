//
// Keyboard/mouse binding table. Dispatch (App::subscription via lookup())
// and any help display are both derived from this one list, so they can't
// drift apart. Every chord is exact — a key event fires a binding only if it
// matches the key AND the modifier set, so nothing dispatches by
// fall-through. First match still wins, but shapes are disjoint enough that
// order is cosmetic.
//
// NOTE: the mod declaration in main.rs carries #[rustfmt::skip] for this
// whole file — the table's column alignment is the point.
//

use crate::Msg;
use ee_viewer::QVMsg;

use iced::keyboard::{self, Key as KK, Modifiers, key::Named as KN};
use iced::mouse::{ScrollDelta};
use iced_core::keyboard::key::Physical as KP;

#[derive(Debug)]
pub(crate) enum Chord {
    Named(KN),                          // named key + physical code; NO modifiers
    NamedMod(KN, Modifiers),            // named key + exact modifier set
    Text(&'static str),                 // text from a Character key; NONE or SHIFT only
    Char(&'static str, Modifiers),      // Character key + exact modifier set
    Mouse,                              // display only, never matches a key event
}

#[allow(dead_code)] // label/group/help are consumed by the help panel
pub(crate) struct Binding {
    pub(crate) chord:  Chord,
    pub(crate) label:  &'static str,
    pub(crate) group:  &'static str,
    pub(crate) help:   &'static str,
    pub(crate) msg:    Msg,
}

use Chord::{Named,NamedMod,Text,Char,Mouse};

pub(crate) const BINDINGS: &[Binding] = &[
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
    Binding{ chord:Char("b",Modifiers::ALT),      label:"Alt+B",        group:"Display",    help:"toggle borderless",                  msg:Msg::BorderlessToggle                   },
    Binding{ chord:Char("e",Modifiers::ALT),      label:"Alt+E",        group:"Display",    help:"toggle EXIF panel",                  msg:Msg::Qv(QVMsg::ExifDisplayToggle)       },
    Binding{ chord:Char("d",Modifiers::ALT),      label:"Alt+D",        group:"Display",    help:"toggle pending-load overlay",        msg:Msg::Qv(QVMsg::LookAheadDisplayToggle)  },
    Binding{ chord:Char("-",Modifiers::CTRL),     label:"Ctrl+-",       group:"Display",    help:"smaller UI text",                    msg:Msg::Qv(QVMsg::FontDown)                },
    Binding{ chord:Char("+",Modifiers::CTRL),     label:"Ctrl++",       group:"Display",    help:"larger UI text",                     msg:Msg::Qv(QVMsg::FontUp)                  },
    Binding{ chord:Char("=",Modifiers::CTRL),     label:"Ctrl+=",       group:"Display",    help:"larger UI text",                     msg:Msg::Qv(QVMsg::FontUp)                  },

    // App
    Binding{ chord:Named(KN::Escape),             label:"Esc",          group:"App",        help:"quit",                               msg:Msg::Quit                               },
    Binding{ chord:Text("q"),                     label:"q",            group:"App",        help:"quit",                               msg:Msg::Quit                               },
    Binding{ chord:Text("?"),                     label:"?",            group:"App",        help:"dump args (debug)",                  msg:Msg::Huh                                },

    // Bare modifier presses — stubs, print only for now. NB: a modifier's own
    // press event already carries its bit, so these need NamedMod, not Named.
    Binding{ chord:NamedMod(KN::Shift,Modifiers::SHIFT),
                                                  label:"Shift",        group:"Mods",       help:"stub (prints)",                      msg:Msg::ModifierStub("Shift")              },
    Binding{ chord:NamedMod(KN::Control,Modifiers::CTRL),
                                                  label:"Ctrl",         group:"Mods",       help:"stub (prints)",                      msg:Msg::ModifierStub("Ctrl")               },
    Binding{ chord:NamedMod(KN::Alt,Modifiers::ALT),
                                                  label:"Alt",          group:"Mods",       help:"stub (prints)",                      msg:Msg::ModifierStub("Alt")                },
    Binding{ chord:NamedMod(KN::Super,Modifiers::LOGO),
                                                  label:"Win",          group:"Mods",       help:"stub (prints)",                      msg:Msg::ModifierStub("Win")                },

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

/// The one dispatch entry point: first matching row wins, unbound keys are None.
pub(crate) fn lookup(event: &keyboard::Event) -> Option<Msg> {
    BINDINGS.iter().find( |b| b.chord.matches(event) ).map( |b| b.msg.clone() )
}
