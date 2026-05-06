use std::num::NonZeroUsize;



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



#[derive(Clone,Debug)]
pub enum ScanProgress {
    SomeFiles(Vec<Box<dyn ImageDyn>>),
    CurrentDir(String),
}



pub trait ImageOrigin {
    fn origin(&self)        -> &str;
    fn group(&self)         -> &str;
    fn name(&self)          -> &str;
    fn display(&self)       -> String;
    fn fqp(&self)           -> std::path::PathBuf;
    fn size(&self)          -> u64;
    fn ftime(&self)         -> time::UtcDateTime;
}


use dyn_clone::{clone_trait_object, DynClone};
use std::fmt::{Display,Debug};

pub trait ImageDyn: ImageOrigin+ Display+ Debug+ DynClone+ Send {
}

clone_trait_object!(ImageDyn);

impl <T:ImageOrigin+ Display+ Debug+ DynClone+ Send > ImageDyn for T {
}


