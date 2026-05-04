// use ee_conio::{cprintln};

use crate::img_traits::ImageError;

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display,Formatter};
use std::num::NonZeroUsize;

use crate::img_traits::{  ImageDyn, ImageOrigin, LoadData };
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
    fqp:        PathBuf,
    origin:     String,
    group:      String,
    name:       String,
    size:       u64,
    ftime:      time::UtcDateTime,
}


impl FileSystemImage {
    pub fn from_origin(origin:&PathBuf,fqp:PathBuf) -> Result<FileSystemImage,ImageError> {

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
            origin: origin.display().to_string(),
            group:  group.display().to_string(),
            name:   name.display().to_string(),
            fqp,
            size: 0,
            ftime: time::UtcDateTime::MIN,
        } )
    }

    pub fn from_cl(file:PathBuf) -> Result<FileSystemImage,ImageError>
    {
        let fqp  = file.canonicalize().unwrap();

        let name = match file.file_name() {
            Some(r)   => r,
            None      => { return Err(ImageError::Unexpected); }
        };

        Ok( FileSystemImage {
            origin: "".into(),
            group:  "".into(),
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



pub struct FileSystemHelper {

}


#[derive(Clone,Debug)]
pub enum ScanProgress {
    SomeFiles(Vec<Box<dyn ImageDyn>>),
    CurrentDir(String),
    // More(String),
}

// #[derive(Clone,Debug)]
// pub struct SomeFiles {
//     pub current_dir:    String,
//     pub files:          Vec<Box<dyn ImageDyn>>,
// }


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
            // let mut builder = GlobSetBuilder::new();

            let mut items:FsiVec = VecDeque::new();

            type DirGlob = Vec<(PathBuf,Option<GlobMatcher>)>;

            let mut dirs:DirGlob = Vec::new();

            for dir in &args {
                let path = PathBuf::from(&dir);

                // if path.is_relative() {
                //      ee_conio::cprint!("~[c178]relative ");
                // }

                if dir.contains(['*','?','[']) {
                    // ee_conio::cprintln!("glob ~[c70]{}",dir);

                    let mut pbj = PathBuf::new();
                    let mut pbh = PathBuf::new();

                    let mut got_root = false;

                    for c in path.components() {
                        // ee_conio::cprintln!("{c:?}");

                        let x1 = match c {
                            Component::Normal(c) => c.to_str().unwrap(),
                            _ => "",
                        };

                        pbh.push(c);

                        if !got_root && !x1.contains(['*','?','[']) {
                            pbj.push(c);
                        }
                        else {
                            got_root = true;
                        }
                    }

                    let glob = match GlobBuilder::new(pbh.to_str().unwrap()).literal_separator(false).build() { Ok(g) => g, Err(_) => continue }.compile_matcher();

                    // ee_conio::cprintln!("{}\n{}\n{glob:?}",pbj.display(),pbh.display());

                    dirs.push( (pbj,Some(glob) ) );
                }

                if path.is_dir() {
                    // ee_conio::cprintln!("dir ~[c208]{}",path.display());
                    dirs.push( (dir.into(),None) );
                }


                else if path.is_file() {
                    ee_conio::cprintln!("file ~[c51]{:?}",path);
                    match FileSystemImage::from_cl(path.to_path_buf() ) {
                        Ok(mut i) => {

                            let y = path.as_path();

                            let (size,ftime) = match y.metadata() {
                                Ok(s) => (s.len(),s.created().unwrap()),
                                Err(_) => (0,std::time::SystemTime::now())
                            };

                            i.size = size;
                            i.ftime = ftime.into();

                            ee_conio::cprintln!("i:?");

                            items.push_back( i )
                        },
                        Err(e) => { ee_conio::cprintln!("{e:?}"); continue; }
                    }
                }
            }

            progress.send( drain(& mut items) ).await;



            for dp in dirs {
                progress.send( ScanProgress::CurrentDir(dp.0.display().to_string()) ).await;

                let path = PathBuf::from(dp.0);

                let mut sent:usize = 0;

                use walkdir::{ DirEntry, Error };

                fn the_filter(gm:Option<GlobMatcher>) -> impl FnMut(Result<DirEntry,Error>) -> Option<DirEntry> {
                    move |e| {
                        match &gm {
                            Some(gm) => {
                                match e {
                                    Ok(e) => if gm.is_match(e.path().display().to_string()) { Some(e) } else {None} ,
                                    Err(_) => None
                                }
                            }
                            None => { e.ok() }
                        }
                    }
                }

                for entry in WalkDir::new(&path)
                                .max_depth(max_depth)
                                .into_iter()
                                .filter_map( the_filter(dp.1) )
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

                        match FileSystemImage::from_origin(&path, entry.path().to_path_buf() ) {
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
                        progress.send( drain(& mut items) ).await;
                    }
                }

                progress.send( drain(& mut items) ).await;
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


