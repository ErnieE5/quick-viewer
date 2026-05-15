// use ee_conio::{cprintln};

use crate::img_traits::ImageError;

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug,Display,Formatter};

use crate::img_traits::{  ImageOrigin };
use std::path::{Path,PathBuf};

use walkdir::DirEntry;

#[derive(Clone)]
pub struct FileSystemImage {
    provider:           String,
    source_path:        PathBuf,
    origin:             String,
    group:              String,
    name:               String,
    pub(crate) size:    u64,
    pub(crate) ftime:   time::UtcDateTime,
}

impl Debug for FileSystemImage
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSystemImage")
            .field("provider", &self.provider)
            .field("source_path", &self.source_path.display().to_string() )
            .field("origin", &self.origin )
            .field("group", &self.group )
            .field("name", &self.name )
            .field("size", &self.size )
            .field("ftime", &self.ftime )

        .finish()
    }

}

use dunce;

impl FileSystemImage {

    pub fn from_entry(provider:&str,origin:&PathBuf,de:DirEntry) -> Result<FileSystemImage,ImageError> {

        let source_path = de.path();

        let sub = match source_path.strip_prefix(origin) {
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

        let (size,ftime) = match de.metadata() {
            Ok(s)   => (s.len(),s.created().unwrap()),
            Err(_)  => (0,std::time::SystemTime::now())
        };


        Ok( FileSystemImage {
            provider:   provider.to_string(),
            origin:     origin.display().to_string(),
            group:      group.display().to_string(),
            name:       name.display().to_string(),
            source_path:source_path.to_path_buf(),
            size,
            ftime:ftime.into(),
        } )
    }

    pub fn from_path(provider:&str,file:&Path) -> Result<FileSystemImage,ImageError>
    {
        let source_path = match dunce::canonicalize(&file) {
            Ok(p)  => p,
            Err(_) => { return Err(ImageError::Unexpected); }
        };

        let name = match file.file_name() {
            Some(r)   => r.display().to_string(),
            None      => { return Err(ImageError::Unexpected); }
        };

        let (size,ftime) = match file.metadata() {
            Ok(s) => (s.len(),s.created().unwrap()),
            Err(_) => (0,std::time::SystemTime::now())
        };

        Ok( FileSystemImage {
            provider:   provider.to_string(),
            origin:     "".into(),
            group:      "".into(),
            name,
            source_path,
            size,
            ftime:      ftime.into(),
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
        match dunce::canonicalize(&self.source_path) {
            Ok(p)  => p,
            Err(_) => self.source_path.clone()
        }
    }

    fn display(&self) -> String {
        let s = match self.group.len() > 0 {
         true  => PathBuf::new().join(&self.group).join(&self.name),
         false => PathBuf::new().join(&self.origin).join(&self.name),
        };
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
        self.ftime //.truncate_to_second()
    }
}

impl Ord for FileSystemImage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.source_path.cmp(&other.source_path)
    }
}

impl PartialOrd for FileSystemImage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FileSystemImage {
    fn eq(&self, other: &Self) -> bool {
        self.source_path == other.source_path
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


