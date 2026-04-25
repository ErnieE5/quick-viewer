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


pub trait ImageOrigin {
    fn origin(&self) -> &str;
    fn fqp(&self) -> std::path::PathBuf;
    fn set(&self) -> &str;
    fn name(&self) -> &str;
    fn display(&self) -> String;
    fn get_index(&self) -> u64;
}




use dyn_clone::{clone_trait_object, DynClone};

pub trait ImageDyn:
                ImageOrigin +
                std::fmt::Display+
                std::fmt::Debug+
                DynClone+
                Send+
                Sync+
                {}

clone_trait_object!(ImageDyn);

impl <T:ImageOrigin +
        std::fmt::Display+
        std::fmt::Debug+
        DynClone+
        Send+
        Sync+
        > ImageDyn for T {}


