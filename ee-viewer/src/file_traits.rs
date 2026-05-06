use std::num::NonZeroUsize;
use std::time::{Duration};
use iced::widget::image::Handle as ImageHandle;
use iced::Size;
use std::hash::{Hash, Hasher};
use std::fmt::{self,Debug};


// pub trait FileProvider
// {
// pub fn find_files_sipper(args: Vec<String>,max_depth:usize) -> impl Straw<(), ScanProgress, ImageError>
//     fn sip_them(args: Vec<String>,max_depth:usize) ->
// }



#[allow(unused)]
pub struct LoadData {
    pub id:         NonZeroUsize,
    pub handle:     Option<ImageHandle>,
    pub open:       Duration,
    pub read:       Duration,
    pub decode:     Duration,
    pub dimensions: Size<u32>,
    pub exif:       Option<exif::Exif>,
}


impl Debug for LoadData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoadData")
            .field("id", &self.id)
            .field("handle", if self.handle.is_some() { &true } else { &false } )
            .field("open", &self.open)
            .field("read", &self.read)
            .field("decode", &self.decode)
            .field("dimensions", &self.dimensions)
            .field("exif", if self.exif.is_some() { &true } else { &false })

        .finish()
    }
}

impl Clone for LoadData {
    fn clone(&self) -> Self { todo!() }
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
