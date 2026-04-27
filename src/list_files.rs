// use ee_conio::{cprintln};

use crate::img_traits::ImageError;

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display,Formatter};
use std::num::NonZeroUsize;

use crate::img_traits::{  ImageDyn, ImageOrigin };
use std::path::{PathBuf};

use image::ImageReader;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;

use iced::task::{Straw, sipper};
use std::collections::{VecDeque};

use walkdir::WalkDir;


#[derive(Debug, Clone)]
pub struct FileSystemImage {
    fqp:        PathBuf,
    origin:     String,
    set:        String,
    name:       String,
    index:      u64,
}


impl FileSystemImage {
    pub fn new(origin:&PathBuf,file:PathBuf) -> Result<FileSystemImage,ImageError> {

        let fqp     = file.clone();

        let sub = match file.strip_prefix(origin) {
            Ok(r)   => r,
            Err(_)  => { return Err(ImageError::Unexpected); }
        };

        let set = match sub.parent() {
            Some(r)   => r,
            None      => { return Err(ImageError::Unexpected); }
        };

        let name = match sub.file_name() {
            Some(r)   => r,
            None      => { return Err(ImageError::Unexpected); }
        };

        Ok( FileSystemImage {
            origin: origin.display().to_string(),
            set:    set.display().to_string(),
            name:   name.display().to_string(),
            fqp,
            index:0
        } )
    }


}


impl ImageOrigin for FileSystemImage {
    fn origin(&self) -> &str {
        &self.origin
    }

    fn fqp(&self) -> PathBuf {
        return self.fqp.clone();
    }

    fn display(&self) -> String {
        let s = PathBuf::new().join(&self.set).join(&self.name);
        let o = match s.to_str() {
            Some(s) => s,
            None => ""
        };
        o.to_string()
    }

    fn set(&self) -> &str {
        self.set.as_str()
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn get_index(&self) -> u64 {
        self.index
    }
}


impl Ord for FileSystemImage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl PartialOrd for FileSystemImage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FileSystemImage {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for FileSystemImage {}

impl Display for FileSystemImage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        let s = PathBuf::new().join(&self.set).join(&self.name);
        let o = match s.to_str() {
            Some(s) => s,
            None => ""
        };
        f.write_fmt(format_args!("{o}"))
    }
}



pub struct FileSystemHelper {

}

#[derive(Clone,Debug)]
pub struct SomeFiles {
    pub current_dir:    String,
    pub files:          Vec<Box<dyn ImageDyn>>,
}


static EXTENSIONS: &'static [&'static str] = &[
    "jpg", "jpeg", "png",
    "gif", "JPG", "JPEG", "PNG",
    "heic", "HEIC",
    "GIF", "webp"
];

type FsiVec = VecDeque<FileSystemImage>;

impl FileSystemHelper {

    pub fn find_files_sipper(dir: String,max_depth:usize) -> impl Straw<(), SomeFiles, ImageError> {
        sipper(async move |mut progress| {
            let path = PathBuf::from(&dir);
            let mut sent:usize = 0;

            let mut drain = async |items:& mut FsiVec| {
                let mut msg = SomeFiles {
                    current_dir:    String::from(""),
                    files:          Vec::<Box<dyn ImageDyn>>::new()
                };

                'r: loop {
                    match items.pop_front() {
                        Some(i) => { msg.files.push(Box::new(i)); }
                        None    => { break 'r; }
                    }
                }

                match msg.files.last() {
                    Some(s) => { msg.current_dir = s.set().to_string() },
                    None    => ()
                };

                progress.send(msg).await;
            };

            let mut items:FsiVec = VecDeque::new();

            for entry in WalkDir::new(&path)
                .max_depth(max_depth)
                .into_iter()
                .filter_map( |e| { e.ok() } )
            {
                let _size = match entry.metadata() {
                    Ok(s) => s.len(),
                    Err(_) => 0
                };

                let ext = match entry.path().extension() {
                    Some(ext) => match ext.to_str() { None => { continue; }, Some(ext) => ext, }
                    None => { continue; }
                };

                if !EXTENSIONS.contains( &ext ) {
                    continue;
                }

                if entry.file_type().is_file() {
                    match FileSystemImage::new(&path, entry.path().to_path_buf() ) {
                        Ok(i) => { items.push_back( i ) },
                        Err(_) => { continue; }
                    }
                }

                if items.len()> if sent < 10000 { 99 } else { 999 } {
                    sent += items.len();
                    drain(& mut items).await;
                }
            }

            drain(& mut items).await;

            Ok(())
        })
    }

    pub async fn load_image(fqp:PathBuf,id: NonZeroUsize)
        -> Result<(NonZeroUsize, iced::widget::image::Handle), ImageError> {

        let ext = match fqp.as_path().extension() {
            Some(ext) => match ext.to_str() { None => { "" }, Some(ext) => ext, }
            None => { "" }
        };

        let mut file = match File::open(&fqp) {
            Ok(f) => f,
            Err(_e) => { return Err(ImageError::ErrorOpeningImageFile(id)); }
        };

        let mut buffer = Vec::new();
        let Ok(_) = file.read_to_end(&mut buffer) else {
            return Err(ImageError::ErrorReadingImageFile(id));
        };

        let Ok(reader) = ImageReader::new(Cursor::new(buffer)).with_guessed_format() else {
            return Err(ImageError::ErrorGuessingFormat(id));
        };

        let image = match reader.decode() {
            Ok(i) => i,
            Err(_e) => {
                return Err(ImageError::ErrorDecodingImage(id));
            }
        };

        let w = image.width();
        let h = image.height();
        let d = image.to_rgba8().into_raw();

        use iced::widget::image::Handle;
        Ok((id, Handle::from_rgba(w, h, d)))
    }

}


