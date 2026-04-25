
// use ee_conio::cprintln as cprintf;
// use ee_conio::cprintln;
use std::fmt;
use std::fmt::{Display,Formatter};
use std::ops::{RangeBounds,Bound};
use std::collections::{HashMap};
use std::iter::Rev;

use std::num::NonZero;
use std::num::NonZeroUsize;

use crate::img_traits::{ ImageDyn, ImageError, };

type ImageKey = NonZeroUsize;


#[derive(Debug,Clone)]
pub struct ImageList {
    store:      HashMap<ImageKey,Box<dyn ImageDyn>>,
    list:       Vec<ImageKey>,
    list_index: usize,
    next_key:   ImageKey,
}


pub struct PeekWalker {
    pos:    NonZeroUsize,
    total:  NonZeroUsize,
    count:  usize,
}


impl PeekWalker {
    fn new(pos:NonZeroUsize,total:NonZeroUsize,count:usize) -> PeekWalker {
        PeekWalker { pos, total, count }
    }
}


impl Iterator for PeekWalker {
    type Item = NonZeroUsize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            // ee_conio::cprintln!("pos {}",self.pos);
            if self.pos < self.total {
                self.pos = self.pos.checked_add(1).expect("reality");
            } else {
                self.pos = NonZero::new(1).expect("one must not be zero");
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
    // #[allow(unused)]
    // pub fn next(&mut self) {
    //     if self.index < self.list.len() - 1 {
    //         self.index += 1;
    //     } else {
    //         self.first();
    //     }
    // }

    // #[allow(unused)]
    // pub fn prev(&mut self) {
    //     if self.index > 0 {
    //         self.index -= 1;
    //     } else {
    //         self.last();
    //     };
    // }

    pub fn to_external_index(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.list_index+1).expect("one must not be zero")
    }

    pub fn first(&mut self) -> Result<NonZeroUsize,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages)
        }
        else
        {
            self.list_index = 0;
            Ok( self.to_external_index() )
        }
    }

    pub fn last(&mut self) -> Result<NonZeroUsize,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages)
        }
        else
        {
            self.list_index = self.list.len()-1;
            Ok( self.to_external_index() )
        }
    }

    pub fn the_index(&self) -> Result<NonZeroUsize,ImageError>
    {
        match self.list.is_empty() {
            true   => Err(ImageError::NoImages),
            false  => Ok( self.to_external_index() )
        }
    }

    pub fn total_items(&self) -> Result<NonZeroUsize,ImageError>
    {
        match self.list.len() > 0 {
            false => Err(ImageError::NoImages),
            true  => Ok( NonZeroUsize::new(self.list.len()).expect("") )
        }
    }

    pub fn peek_range<R>(&self,r: R) -> PeekWalker
    where
        R: RangeBounds<isize>
    {
        // For the iterator to just signal None
        if self.list.is_empty() {
            let one = NonZero::new(1).expect("reality");
            return PeekWalker::new( one, one, 0 );
        };

        let s = match r.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(e) => { panic!(); },
            Bound::Unbounded   => 0 , // this
        };

        let mut count = 0;
        let e = match r.end_bound() {
            Bound::Included(i) => { count=(i+1)-s; *i+1 },
            Bound::Excluded(e) => *e,
            Bound::Unbounded   => 1,  // error
        };

        // ee_conio::cprintln!("\n~[c208] {s} {e} li:{}",self.list_index as isize);

        let mut pos = (self.list_index as isize)+s;

        // ee_conio::cprintln!("~[c200] un {pos}");

        if pos <= 0 {
            pos = self.list.len() as isize + pos;
        } else if pos > self.list.len() as isize {
            pos = pos-(self.list.len() as isize);
        }

        // ee_conio::cprintln!("~[c204] be {pos}");


        let pos = NonZeroUsize::new(pos as usize).expect("value must not be zero");

        // Calculate Starting Position
        // Calculate Steps
        // ee_conio::cprintln!("~[c227]{pos}  ~[c51]s:{s:?} ~[c77]e:{e:?} count:{count}");

        let t = match self.total_items() {
            Ok(i) => i,
            Err(_) => { count=0;  NonZero::new(1).expect("reality") }
        };

        PeekWalker::new( pos ,t, count.try_into().unwrap()  )
    }


    pub fn key(&self) -> Result<ImageKey,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }
        Ok( self.list[self.list_index] )
    }

    pub fn key_at(&self, index:NonZeroUsize) -> Result<ImageKey,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }

        let internal = index.get()-1;

        if internal <= self.list.len() {
            Ok( self.list[internal] )
        }
        else
        {
            Err(ImageError::IndexOverflow)
        }
    }


    pub fn item_from_key(&self,key:ImageKey) -> Result<&dyn ImageDyn,ImageError> {
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
            Err(ImageError::NoImages)
        } else {

            let key = self.list[self.list_index];

            match self.store.get( &key ) {
                Some(s) => Ok( &**s ),
                None    => Err(ImageError::InvalidItemKey)
            }
        }
    }

    #[allow(unused)]
    pub fn item_at(&self, index:NonZeroUsize) -> Result<&dyn ImageDyn,ImageError> {

        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }

        let internal = index.get()-1;

        if internal  > self.list.len() {
            return Err(ImageError::IndexOverflow);
        }

        let key = self.list[internal];

        match self.store.get( &key ) {
            Some(s) => Ok( &**s ),
            None    => Err(ImageError::InvalidItemKey)
        }
    }

    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    pub fn append(& mut self,i:Vec<Box<dyn ImageDyn>>) {
        let init = self.list.is_empty();
        for a in i.into_iter() {
            let key = self.next_key;
            self.next_key = self.next_key.checked_add(1).expect("reality");
            self.store.insert(key,a);
            self.list.push( key );
        }
        if init {
            self.list_index = 0;
        }
    }

    pub fn goto(&mut self,idx:NonZeroUsize) -> Result<NonZeroUsize,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }

        let internal = idx.get()-1;

        if internal > self.list.len() {
            return Err(ImageError::IndexOverflow);
        }

        self.list_index = internal;

        Ok( self.to_external_index() )
    }

    pub fn random(&mut self) -> Result<NonZeroUsize,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }
        self.list_index = fastrand::usize(0..self.list.len());
        Ok( self.to_external_index() )
    }

    pub fn find_index(&mut self,key:NonZeroUsize) -> Result<NonZeroUsize,ImageError> {
        match self.list.iter().position(|&i| i==key) {
            Some(idx) => Ok( NonZeroUsize::new(idx+1).expect("bad stuff happened") ),
            None => { return Err(ImageError::InvalidItemKey); }
        }
    }


    pub fn sort(&mut self) -> Result<(),ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::Uninitialized);
        }
        let idx = self.list_index;
        let cur = self.list[idx];

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

        self.list_index = match self.list.iter().position(|&i| i==cur) {
            Some(idx) => idx,
            None => { return Err(ImageError::Unexpected); }
        };

        Ok(())
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
            let t = self.list.len();
            let c = self.list_index+1;

            let k = self.list[self.list_index];
            let s = match self.item() {
                Ok(s) => (*s).name(),
                Err(_)  => "<err>",
            };

            f.write_fmt(format_args!("{c:>12}/{t:<12} {k:x} {s}"))
        }
    }

}

impl ImageList {

    pub fn new() -> ImageList {
        ImageList {
            store:HashMap::new(),
            list:Vec::new(),
            list_index:0,
            next_key:NonZeroUsize::new(0x10000EE5).expect("math working"),
        }
    }

}

