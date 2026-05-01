use std::num::NonZeroUsize;

use std::time::{Duration};

use iced::widget::image::Handle as ImageHandle;

use std::hash::{Hash, Hasher};

#[derive(Debug,Clone)]
#[allow(unused)]
pub struct LoadData {
    pub id:         NonZeroUsize,
    pub handle:     ImageHandle,
    pub open:       Duration,
    pub read:       Duration,
    pub decode:     Duration,
}


impl PartialEq for LoadData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LoadData {}

impl Hash for LoadData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}


#[derive(Debug,Clone)]
pub enum ImageError
{
    NoImages,
    Uninitialized,
    Unexpected,
    // InvalidIndex,
    InvalidItemKey,
    IndexOverflow,
    ErrorOpeningImageFile(NonZeroUsize),
    ErrorReadingImageFile(NonZeroUsize),
    ErrorGuessingFormat(NonZeroUsize),
    ErrorDecodingImage(NonZeroUsize),
}


pub trait ImageOrigin {
    fn origin(&self)    -> &str;
    fn group(&self)     -> &str;
    fn name(&self)      -> &str;
    fn display(&self)   -> String;
    fn fqp(&self)       -> std::path::PathBuf;
    fn size(&self)      -> u64;
    fn ftime(&self)     -> time::UtcDateTime;
}


use dyn_clone::{clone_trait_object, DynClone};
use std::fmt::{Display,Debug};

pub trait ImageDyn: ImageOrigin+ Display+ Debug+ DynClone+ Send {
}

clone_trait_object!(ImageDyn);

impl <T:ImageOrigin+ Display+ Debug+ DynClone+ Send > ImageDyn for T {
}


