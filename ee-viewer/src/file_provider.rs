// use ee_conio::{cprintln};

use crate::img_traits::ImageError;

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display,Formatter};
use std::num::NonZeroUsize;

use crate::img_traits::{  ImageDyn, ImageOrigin, ScanProgress };

use crate::file_traits::{ LoadData };

use std::path::{PathBuf,Component};
use std::time::{Instant};

use image::ImageReader;
use std::fs::File;
use std::io::{ Seek, SeekFrom };

use iced::Size;
use iced::task::{Straw, sipper};
use std::collections::{VecDeque};

use walkdir::WalkDir;

use iced::widget::image::Handle as ImageHandle;

use exif::Reader as ExifReader;

#[derive(Debug, Clone)]
pub struct FileSystemImage {
    provider:   String,
    fqp:        PathBuf,
    origin:     String,
    group:      String,
    name:       String,
    size:       u64,
    ftime:      time::UtcDateTime,
}


impl FileSystemImage {

    pub fn from_origin<S:AsRef<str>>(provider:S,origin:&PathBuf,fqp:PathBuf) -> Result<FileSystemImage,ImageError> {

        let fqp     = fqp.clone();

        let sub = match fqp.strip_prefix(origin) {
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
            provider:   provider.as_ref().to_string(),
            origin:     origin.display().to_string(),
            group:      group.display().to_string(),
            name:       name.display().to_string(),
            fqp,
            size:       0,
            ftime:      time::UtcDateTime::MIN,
        } )
    }

    pub fn from_cl<S:AsRef<str>>(provider:S,file:PathBuf) -> Result<FileSystemImage,ImageError>
    {
        let fqp  = file.canonicalize().unwrap();

        let name = match file.file_name() {
            Some(r)   => r,
            None      => { return Err(ImageError::Unexpected); }
        };

        Ok( FileSystemImage {
            provider:   provider.as_ref().to_string(),
            origin:     "".into(),
            group:      "".into(),
            name:       name.display().to_string(),
            fqp,
            size:       0,
            ftime:      time::UtcDateTime::MIN,
        } )
    }

}


impl ImageOrigin for FileSystemImage {
    fn provider(&self) -> &str {
        &self.provider
    }

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
        self.ftime.truncate_to_second()
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


#[derive(Debug)]
pub struct FileSystemHelper {
}

impl FileSystemHelper {
    pub fn new() -> Self { Self{} }
    pub async fn load_image(fqp:PathBuf,id:NonZeroUsize)
        -> Result<LoadData, ImageError> {

        let open = Instant::now();
        let file = match File::open(&fqp) {
            Ok(f) => f,
            Err(_e) => { return Err(ImageError::ErrorOpeningImageFile(id)); }
        };
        let open = open.elapsed();

        tokio::task::yield_now().await;


        let read = Instant::now();

        let mut bufreader = std::io::BufReader::new(&file);

        let exifreader = ExifReader::new();

        let exif = match exifreader.read_from_container(&mut bufreader) {
            Ok(exif) => { Some(exif) },
            Err(_)   => { None }
        };

        match bufreader.seek(SeekFrom::Start(0)) {
            Ok(_)   => { },
            Err(_e) => { return Err(ImageError::ErrorReadingImageFile(id)); }
        }

        let read = read.elapsed();


        tokio::task::yield_now().await;

        let decode = Instant::now();
        let Ok(reader) = ImageReader::new(bufreader).with_guessed_format() else {
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

        let r = LoadData {
            id,
            handle: Some(handle),
            open,
            read,
            decode,
            dimensions:Size::new(width,height),
            exif
        };

        Ok( r )
    }

}


static EXTENSIONS: &'static [&'static str] = &[
    "jpg", "jpeg", "png",
    "gif", "JPG", "JPEG", "PNG",
    "heic", "HEIC",
    "GIF", "webp"
];



type FsiVec = VecDeque<FileSystemImage>;

fn drain(items:& mut FsiVec) -> ScanProgress {
    let mut files = Vec::<Box<dyn ImageDyn>>::new();

    'r: loop {
        match items.pop_front() {
            Some(i) => { files.push(Box::new(i)); }
            None    => { break 'r; }
        }
    }

    ScanProgress::SomeFiles(files)
}


impl FileSystemHelper {

    pub fn find_files_sipper(args: Vec<String>,max_depth:usize) -> impl Straw<(), ScanProgress, ImageError> {
        sipper(async move |mut progress| {

            use globset::{GlobMatcher,GlobBuilder};

            let mut items:FsiVec = VecDeque::new();

            type DirGlob = Vec<(PathBuf,Option<GlobMatcher>)>;

            let mut dirs:DirGlob = Vec::new();

            for dir in &args {
                let path = PathBuf::from(&dir);

                if dir.contains(['*','?','[']) {
                    let mut pb_root  = PathBuf::new();
                    let mut pb_match = PathBuf::new();
                    let mut got_root = false;

                    for c in path.components() {

                        let x1 = match c {
                            Component::Normal(c) => c.to_str().unwrap(),
                            _ => "",
                        };

                        pb_match.push(c);

                        if !got_root && !x1.contains(['*','?','[']) {
                            pb_root.push(c);
                        }
                        else {
                            got_root = true;
                        }
                    }

                    let glob = match GlobBuilder::new(pb_match.to_str().unwrap()).case_insensitive(true).build() { Ok(g) => g, Err(_) => continue }.compile_matcher();

                    dirs.push( (pb_root, Some(glob) ) );
                }

                if path.is_dir() {
                    dirs.push( (dir.into(),None) );
                }
                else if path.is_file() {
                    match FileSystemImage::from_cl("cl",path.to_path_buf() ) {
                        Ok(mut i) => {
                                let y = path.as_path();

                            let (size,ftime) = match y.metadata() {
                                Ok(s) => (s.len(),s.created().unwrap()),
                                Err(_) => (0,std::time::SystemTime::now())
                            };

                            i.size  = size;
                            i.ftime = ftime.into();

                            items.push_back( i )
                        },
                        Err(e) => { ee_conio::cprintln!("{e:?}"); continue; }
                    }
                }
            }

            progress.send( drain(& mut items) ).await;



            for dp in dirs {
                progress.send( ScanProgress::CurrentDir(dp.0.display().to_string()) ).await;

                use walkdir::{ DirEntry };

                fn walkdir_filter(gm:&Option<GlobMatcher>) -> impl FnMut(&DirEntry) -> bool {
                    |e| {
                        if e.depth() > 0 {
                            match gm {
                                Some(gm) => {
                                    let item = e.path().display().to_string();
                                    gm.is_match(item)
                                },
                                None     => true
                            }
                        }
                        else {
                            true
                        }
                    }
                }

                let path = PathBuf::from(dp.0);

                let mut sent:usize = 0;

                for entry in WalkDir::new(&path)
                                .max_depth(max_depth)
                                .into_iter()
                                .filter_entry( walkdir_filter(&dp.1) )
                                .filter_map( |e| e.ok()  )
                {
                    if entry.file_type().is_dir() && entry.depth()>0 {
                        let stat = match entry.path().strip_prefix(&path) {
                            Ok(p) => p.display().to_string(),
                            Err(_) => entry.path().display().to_string()
                        };

                        progress.send( ScanProgress::CurrentDir(stat) ).await;
                        continue;
                    }

                    let ext = match entry.path().extension() {
                        Some(ext) => match ext.to_str() { None => { continue; }, Some(ext) => ext, }
                        None => { continue; }
                    };

                    if !EXTENSIONS.contains( &ext ) {
                        tokio::task::yield_now().await;
                        continue;
                    }

                    if entry.file_type().is_file() {
                        let (size,ftime) = match entry.metadata() {
                            Ok(s) => (s.len(),s.created().unwrap()),
                            Err(_) => (0,std::time::SystemTime::now())
                        };

                        match FileSystemImage::from_origin("sip",&path, entry.path().to_path_buf() ) {
                            Ok(mut i) => {

                                i.size = size;
                                i.ftime = ftime.into();

                                items.push_back( i )
                            },
                            Err(_) => { continue; }
                        }
                    }

                    let drain_it = match sent {
                            0           => 0,
                            1..100      => 98,
                            100..10000  => 99,
                            10000..     => 999,
                    };

                    if items.len() > drain_it {
                        sent += items.len();
                        progress.send( drain(& mut items) ).await;
                    }
                }

                progress.send( drain(& mut items) ).await;
            }

            Ok(())
        })
    }

}


