
// use ee_conio::cprintln as cprintf;
// use ee_conio::cprintln;
use std::fmt;
use std::fmt::{Display,Formatter};
use std::collections::{HashMap};
use std::iter::Rev;

use crate::img_traits::{ ImageDyn, ImageError, };




#[derive(Debug,Clone)]
pub struct ImageList {
    store:  HashMap<usize,Box<dyn ImageDyn>>,
    list:   Vec<usize>,
    index:  usize,
    next_key: usize,
}


pub struct PeekWalker {
    pos: usize,
    total: usize,
    count:usize,
}


impl PeekWalker {
    fn new(pos:usize,total:usize,count:usize) -> PeekWalker {
        PeekWalker { pos, total, count }
    }
}


impl Iterator for PeekWalker {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            if self.pos < self.total {
                self.pos += 1;
            } else {
                self.pos = 1;
            }

            self.count-=1;
            Some(self.pos)
        }
        else
        {
            None
        }
    }
}

impl DoubleEndedIterator for PeekWalker {

    fn next_back(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            if self.pos > 1 {
                self.pos -= 1;
            } else {
                self.pos = self.total;
            }

            self.count-=1;
            Some(self.pos)
        }
        else
        {
            None
        }
    }
}


impl ImageList {
    #[allow(unused)]
    pub fn next(&mut self) {
        if self.index < self.list.len() - 1 {
            self.index += 1;
        } else {
            self.first();
        }
    }

    #[allow(unused)]
    pub fn prev(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.last();
        };
    }

    pub fn first(&mut self) -> usize {
        self.index = 0;
        1
    }

    pub fn last(&mut self) -> usize {
        self.index = self.list.len() - 1;
        self.index+1
    }

    pub fn the_index(&self) -> usize { self.index+1 }

    pub fn total_items(&self) -> usize { self.list.len() }

    pub fn peek_foreward(&self, c:usize) -> PeekWalker {
        PeekWalker::new( self.index+1,self.list.len(),c  )
    }

    pub fn peek_backward(&self, c:usize) -> Rev<PeekWalker> {
        PeekWalker::new( self.index+1,self.list.len(),c ).rev()
    }

    pub fn key(&self) -> Result<usize,ImageError> {
        if self.list.is_empty() {
            Err(ImageError::Uninitialized)
        } else {
            if self.index < self.list.len() {
                let idx = self.list[self.index];
                match self.store.get( &idx ) {
                    Some(_) => Ok(idx),
                    None    => Err(ImageError::InvalidItemKey)
                }
            } else {
                Err(ImageError::IndexOverflow)
            }
        }
    }

    pub fn key_at(&self, index:usize) -> Result<usize,ImageError> {
        if index == 0 { return Err(ImageError::InvalidIndex) }
        if !self.list.is_empty() && index <= self.list.len() {
            let key = self.list[index-1];
            match self.store.get( &key ) {
                Some(_) => Ok(key),
                None    => Err(ImageError::InvalidItemKey)
            }
        } else {
            Err(ImageError::IndexOverflow)
        }
    }


    pub fn item_from_key(&self,key:usize) -> Result<&dyn ImageDyn,ImageError> {
        if self.list.is_empty() {
            Err(ImageError::Uninitialized)
        } else {
            match self.store.get( &key ) {
                Some(s) => Ok( &**s ),
                None    => Err(ImageError::InvalidItemKey)
            }
        }
    }




    pub fn item(&self) -> Result<&dyn ImageDyn,ImageError>     {
        if self.list.is_empty() {
            Err(ImageError::Uninitialized)
        } else {
            if self.index < self.list.len() {
                match self.store.get( &self.list[self.index] ) {
                    Some(s) => Ok( &**s ),
                    None    => Err(ImageError::InvalidItemKey)
                }
            } else {
                Err(ImageError::IndexOverflow)
            }
        }
    }

    #[allow(unused)]
    pub fn item_at(&self, index:usize) -> Result<&dyn ImageDyn,ImageError> {
        if index == 0 { return Err(ImageError::InvalidIndex) }
        if !self.list.is_empty() && index <= self.list.len() {
            match self.store.get( &self.list[index-1] ) {
                Some(s) => Ok( &**s ),
                None    => Err(ImageError::InvalidItemKey)
            }
        } else {
            Err(ImageError::IndexOverflow)
        }
    }

    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    pub fn append(& mut self,i:Vec<Box<dyn ImageDyn>>) {
        for a in i.into_iter() {
            let key = self.next_key;
            self.next_key+=1;
            self.store.insert(key,a);
            self.list.push( key );
        }
    }

    pub fn goto(&mut self,idx:usize) -> Result<usize,ImageError> {
        if idx == 0 { return Err(ImageError::InvalidIndex) }
        if !self.list.is_empty() && idx <= self.list.len() {
            self.index = idx-1
        }
        Ok(idx)
    }

    pub fn random(&mut self) -> usize {
        self.index = fastrand::usize(1..self.list.len());
        self.index
    }

    pub fn find_index(&mut self,key:usize) -> Result<usize,ImageError> {
        match self.list.iter().position(|&i| i==key) {
            Some(idx) => Ok(idx+1),
            None => { return Err(ImageError::InvalidItemKey); }
        }
    }


    pub fn sort(&mut self) {

        let cur = self.list[self.index];

        self.list.sort_by( |a,b| {

            let aa = match self.store.get(a) {
                Some(a) => a.name(),
                None => { todo!(); }
            };
            let bb = match self.store.get(b) {
                Some(b) => b.name(),
                None  => { todo!(); }
            };

            aa.cmp(bb)
        });

        self.index = match self.list.iter().position(|&i| i==cur) {
            Some(i) => i,
            None => { panic!(); }
        }
    }

}

impl Display for ImageList
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        if self.list.is_empty() {
            f.write_fmt(format_args!("Unitilized"))
        }
        else
        {
            let p = self.index+1;
            let m = self.list.len();
            let s = match self.item() {
                Ok(s) => (*s).name(),
                Err(_)  => "<err>",
            };

            f.write_fmt(format_args!("{p:>9}/{m:<9} {s}"))
        }
    }

}

impl ImageList {

    pub fn new() -> ImageList {
        ImageList {
            store:HashMap::new(),
            list:Vec::new(),
            index:0,
            next_key:0x10000EE5,
        }
    }

}

