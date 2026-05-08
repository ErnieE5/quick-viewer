// use ee_conio::{cprintln};

use crate::img_traits::ImageError;

use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug,Display,Formatter};

use crate::img_traits::{  ImageOrigin };
use std::path::{PathBuf};

#[derive(Clone)]
pub struct FileSystemImage {
    provider:   String,
    fqp:        PathBuf,
    origin:     String,
    group:      String,
    name:       String,
    pub(crate) size:       u64,
    pub(crate) ftime:      time::UtcDateTime,
}

impl Debug for FileSystemImage
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSystemImage")
            .field("provider", &self.provider)
            .field("fqp", &self.display().to_string() )
            .field("origin", &self.origin )
            .field("group", &self.group )
            .field("name", &self.name )
            .field("size", &self.size )
            .field("ftime", &self.ftime )

        .finish()
    }

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
            Some(r)   => r.display().to_string(),
            None      => { return Err(ImageError::Unexpected); }
        };

        Ok( FileSystemImage {
            provider:   provider.as_ref().to_string(),
            origin:     "".into(),
            group:      "".into(),
            name,
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


