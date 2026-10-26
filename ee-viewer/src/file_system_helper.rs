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

use image::{ImageReader,ImageDecoder,DynamicImage};
use image::metadata::Orientation;
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

        //  Not reader.decode(): that ignores EXIF orientation, so a portrait JPEG shot
        //  sideways shows sideways. Going through the decoder lets us ask it first.
        //  HEIC is already upright -- libheif applies its own irot/imir -- and its
        //  image-crate hook reports no orientation, so it is not turned twice.
        //
        //  In its own block because the decoder is not Send: it must be gone before the
        //  yield_now().await below, or the whole load future stops being Send.
        let image = {
            let mut decoder = match reader.into_decoder() {
                Ok(d) => d,
                Err(_e) => {
                    return Err(ImageError::ErrorDecodingImage(id));
                }
            };

            //  An unreadable orientation is not a reason to fail the image: show it as stored.
            let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);

            let mut image = match DynamicImage::from_decoder(decoder) {
                Ok(i) => i,
                Err(_e) => {
                    return Err(ImageError::ErrorDecodingImage(id));
                }
            };

            image.apply_orientation(orientation);   // a 90/270 turn swaps width and height
            image
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

    //  `check` false is --no-check: a named file with an image extension is taken on trust,
    //  with no round trip to the disk -- over SMB that is most of the cost of a long list.
    pub fn find_files_sipper(args: Vec<String>,max_depth:usize,check:bool) -> impl Straw<(), SipProgress, ImageError> {
        sipper(async move |mut progress| {

            let mut items:FsiVec = VecDeque::new();

            use globset::{GlobMatcher,GlobBuilder};

            type DirGlob = Vec<(PathBuf,Option<GlobMatcher>)>;

            let mut dirs:DirGlob = Vec::new();

            //  Named files go out in batches, in the order given (a --file-list's order is
            //  deliberate), so the first shows while a long list is still being checked.
            let mut sent:usize = 0;

            let trusted = |dir:&str| -> bool {
                !check && !dir.contains(['*','?','['])
                       && std::path::Path::new(dir).extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.contains(&e))
            };

            //  One stat per path, STATS_AT_ONCE in flight, handed back in the order given. Over
            //  SMB a stat is a round trip (~7 ms here), so 1,104 of them one after another was
            //  most of 18 s; in flight together they overlap.
            const STATS_AT_ONCE: usize = 32;

            use iced::futures::StreamExt;

            let mut stats = iced::futures::stream::iter( args.clone() ).map( |dir| {
                let skip = trusted(&dir);
                tokio::task::spawn_blocking( move || if skip { None } else { Some( std::fs::metadata(&dir) ) } )
            }).buffered(STATS_AT_ONCE);

            for dir in &args {
                let path = PathBuf::from(&dir);

                let meta = stats.next().await.expect("one stat per arg").expect("stat task panicked");

                if !items.is_empty() && items.len() > match sent { 0 => 0, 1..100 => 98, _ => 99 } {
                    sent += items.len();
                    progress.send( drain(& mut items) ).await;
                }

                let globbed = dir.contains(['*','?','[']);

                if trusted(dir) {
                    match FileSystemImage::from_trusted("cl", path.as_path() ) {
                        Ok(i)  => { items.push_back( i ) },
                        Err(e) => { ee_conio::cprintln!("~[c196]{e:?} ~[c7]{dir}"); }
                    }
                    continue;
                }

                if globbed {
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

                match meta.expect("only trusted paths go unstatted") {
                    Ok(m) if m.is_dir() => {
                        dirs.push( ( dir.into(), None ) );
                    }
                    Ok(m) if m.is_file() => {
                        match FileSystemImage::from_meta("cl", path.as_path(), &m ) {
                            Ok(i)  => { items.push_back( i ) },
                            Err(e) => { ee_conio::cprintln!("~[c196]{e:?} ~[c7]{dir}"); continue; }
                        }
                    }
                    Ok(_)               => { }
                    Err(e) if !globbed  => { ee_conio::cprintln!("~[c196]skipped ~[c7]{dir}: {e}"); }
                    Err(_)              => { }
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


#[cfg(test)]
mod tests {
    use super::*;
    use iced::task::Sipper;

    //  The names that come out of the sipper, in the order they come out.
    fn sip_names(args:Vec<String>,check:bool) -> Vec<String> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on( async {
            let mut names = Vec::new();
            let mut s = FileSystemHelper::find_files_sipper(args,1,check).pin();
            while let Some(p) = s.sip().await {
                if let SipProgress::SomeFiles(l) = p { names.extend( l.iter().map(|i| i.name().to_string()) ); }
            }
            names
        })
    }

    //  A list out of alphabetical order, with a file missing from the middle of it: the
    //  order holds and the missing one is dropped, not fatal. Unchecked, nothing is dropped.
    #[test]
    fn named_files_keep_their_order() {
        let dir = std::env::temp_dir().join(format!("qv-file-list-{}",std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let order:Vec<String> = (0..250).rev().map(|n| format!("img_{n:03}.jpg")).collect();
        for n in &order { std::fs::write(dir.join(n),b"").unwrap(); }

        let mut args:Vec<String> = order.iter().map(|n| dir.join(n).display().to_string()).collect();
        args.insert(100, dir.join("missing.jpg").display().to_string());

        assert_eq!( sip_names(args.clone(),true), order );

        let mut trusted = order.clone();
        trusted.insert(100,"missing.jpg".into());
        assert_eq!( sip_names(args,false), trusted );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
