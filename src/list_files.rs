// use ee_conio::{cprintln};

use crate::img_traits::ImageError;

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display,Formatter};
use std::num::NonZeroUsize;

use crate::img_traits::{  ImageDyn, ImageOrigin, LoadData };
use std::path::{PathBuf};
use std::time::{Instant};

use image::ImageReader;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;

use iced::task::{Straw, sipper};
use std::collections::{VecDeque};

use walkdir::WalkDir;

use iced::widget::image::Handle as ImageHandle;


#[derive(Debug, Clone)]
pub struct FileSystemImage {
    fqp:        PathBuf,
    origin:     String,
    group:      String,
    name:       String,
    size:       u64,
    ftime:      time::UtcDateTime,
}


impl FileSystemImage {
    pub fn new(origin:&PathBuf,file:PathBuf) -> Result<FileSystemImage,ImageError> {

        let fqp     = file.clone();

        let sub = match file.strip_prefix(origin) {
            Ok(r)   => r,
            Err(_)  => { return Err(ImageError::Unexpected); }
        };

        let group = match sub.parent() {
            Some(r)   => r,
            None      => { return Err(ImageError::Unexpected); }
        };

        let name = match sub.file_name() {
            Some(r)   => r,
            None      => { return Err(ImageError::Unexpected); }
        };

        Ok( FileSystemImage {
            origin: origin.display().to_string(),
            group:  group.display().to_string(),
            name:   name.display().to_string(),
            fqp,
            size: 0,
            ftime: time::UtcDateTime::MIN,
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
        let s = PathBuf::new().join(&self.group).join(&self.name);
        let o = match s.to_str() {
            Some(s) => s,
            None => ""
        };
        o.to_string()
    }

    fn group(&self) -> &str {
        self.group.as_str()
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn ftime(&self) -> time::UtcDateTime {
        time::macros::utc_datetime!(1970-01-05 10:11)
    }
}


impl Ord for FileSystemImage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fqp.cmp(&other.fqp)
    }
}

impl PartialOrd for FileSystemImage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FileSystemImage {
    fn eq(&self, other: &Self) -> bool {
        self.fqp == other.fqp
    }
}

impl Eq for FileSystemImage {}

impl Display for FileSystemImage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        let s = PathBuf::new().join(&self.group).join(&self.name);
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

    pub fn find_files_sipper(items: Vec<String>,max_depth:usize) -> impl Straw<(), SomeFiles, ImageError> {
        sipper(async move |mut progress| {

            for dir in items {
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
                        Some(s) => { msg.current_dir = s.group().to_string() },
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
                    let (size,ftime) = match entry.metadata() {
                        Ok(s) => (s.len(),s.created().unwrap()),
                        Err(_) => (0,std::time::SystemTime::now())
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
                            Ok(mut i) => {

                                i.size = size;
                                i.ftime = ftime.into();

                                items.push_back( i )
                            },
                            Err(_) => { continue; }
                        }
                    }

                    if items.len()> if sent < 10000 { 99 } else { 999 } {
                        sent += items.len();
                        drain(& mut items).await;
                    }
                }

                drain(& mut items).await;
            }

            Ok(())
        })
    }

    pub async fn load_image(fqp:PathBuf,id:NonZeroUsize)
        -> Result<LoadData, ImageError> {

        let _ext = match fqp.as_path().extension() {
            Some(ext) => match ext.to_str() { None => { "" }, Some(ext) => ext, }
            None => { "" }
        };

        let open = Instant::now();
        let mut file = match File::open(&fqp) {
            Ok(f) => f,
            Err(_e) => { return Err(ImageError::ErrorOpeningImageFile(id)); }
        };
        let open = open.elapsed();

        tokio::task::yield_now().await;

        let read = Instant::now();
        let mut buffer = Vec::new();
        let Ok(_) = file.read_to_end(&mut buffer) else {
            return Err(ImageError::ErrorReadingImageFile(id));
        };
        let read = read.elapsed();

        tokio::task::yield_now().await;

        let decode = Instant::now();
        let Ok(reader) = ImageReader::new(Cursor::new(buffer)).with_guessed_format() else {
            return Err(ImageError::ErrorGuessingFormat(id));
        };

        let image = match reader.decode() {
            Ok(i) => i,
            Err(_e) => {
                return Err(ImageError::ErrorDecodingImage(id));
            }
        };
        let decode = decode.elapsed();

        tokio::task::yield_now().await;

        let width   = image.width();
        let height  = image.height();
        let data    = image.to_rgba8().into_raw();
        let handle  = ImageHandle::from_rgba(width, height, data);

        Ok( LoadData { id, handle, open, read, decode } )
    }

}


