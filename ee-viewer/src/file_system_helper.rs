// use ee_conio::{cprintln};

use crate::img_traits::ImageError;

// use std::cmp::Ordering;
// use std::fmt;
// use std::fmt::{Display,Formatter};
use std::num::NonZeroUsize;

use crate::img_traits::{  ImageDyn, SipProgress };
use crate::file_system_image::{FileSystemImage};
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

static EXTENSIONS: &'static [&'static str] = &[
    "jpg", "jpeg", "png", "gif", "heic", "webp",
    "JPG", "JPEG", "PNG", "GIF", "HEIC", "WEBP"
];


#[derive(Debug)]
pub struct FileSystemHelper {}

impl FileSystemHelper {

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





type FsiVec = VecDeque<FileSystemImage>;

fn drain(items:& mut FsiVec) -> SipProgress {
    let mut files = Vec::<Box<dyn ImageDyn>>::new();

    for i in items.drain(0..) {
        files.push( Box::new(i) );
    }

    SipProgress::SomeFiles(files)
}


impl FileSystemHelper {

    pub fn find_files_sipper(args: Vec<String>,max_depth:usize) -> impl Straw<(), SipProgress, ImageError> {
        sipper(async move |mut progress| {

            let mut items:FsiVec = VecDeque::new();

            use globset::{GlobMatcher,GlobBuilder};

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

                    dirs.push( ( pb_root, Some(glob) ) );
                }

                if path.is_dir() {
                    dirs.push( ( dir.into(), None ) );
                }
                else if path.is_file() {
                    match FileSystemImage::from_path("cl", path.as_path() ) {
                        Ok(i)  => { items.push_back( i ) },
                        Err(e) => { ee_conio::cprintln!("{e:?}"); continue; }
                    }
                }
            }

            if !items.is_empty() {
                progress.send( drain(& mut items) ).await;
            }


            for dp in dirs {
                progress.send( SipProgress::CurrentDir(dp.0.display().to_string()) ).await;

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

                        progress.send( SipProgress::CurrentDir(stat) ).await;
                        continue;
                    }

                    let ext = match entry.path().extension() {
                        Some(ext) => match ext.to_str() { None => { continue; }, Some(ext) => ext, }
                        None => { continue; }
                    };

                    if !EXTENSIONS.contains( &ext ) {
                        tokio::task::yield_now().await;
                        // ee_conio::cprintln!("~[c227]doink ~[c7]{:?}",entry);
                        continue;
                    }

                    if entry.file_type().is_file() {
                        match FileSystemImage::from_entry( "sip", &path, entry ) {
                            Ok(i)   => { items.push_back( i ) },
                            Err(_)  => { continue; }
                        }
                    }

                    let drain_it_when = match sent {
                            0           => 0,
                            1..100      => 98,
                            100..10000  => 99,
                            10000..     => 999,
                    };

                    if items.len() > drain_it_when {
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
