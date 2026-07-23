
use std::fmt;
use std::fmt::{Display,Formatter};
use std::ops::{RangeBounds,Bound};
use std::collections::{HashMap};

use std::num::NonZeroUsize;
use std::cmp::Ordering;

use crate::img_traits::{ ImageDyn, ImageError, };

pub type ImageKey = NonZeroUsize;

//
// Index normalization: every position in this module and its public API is a
// 0-based usize into `list`. ImageKey (NonZeroUsize) is item IDENTITY only —
// positions and keys never share a type. The ONLY place a 1-based number is
// allowed to exist is display formatting ("current/total" does the +1 at the
// point of printing, nowhere else).
//

#[derive(Debug,Clone)]
pub struct ImageList {
    store:      HashMap<ImageKey,Box<dyn ImageDyn>>,
    list:       Vec<ImageKey>,
    list_index: usize,
    next_key:   ImageKey,
}

pub struct PeekWalker {
    pos:    usize,      // 0-based, last yielded (pre-incremented in next())
    len:    usize,
    count:  usize,
}

impl Display for PeekWalker
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        let p = self.pos;
        let t = self.len;
        let c = self.count;

        f.write_fmt(format_args!("{p}/{t} remaining:{c}"))
    }
}


impl Iterator for PeekWalker {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        self.count -= 1;
        self.pos = (self.pos + 1) % self.len;
        Some(self.pos)
    }
}



impl ImageList {

    /// Current position, 0-based.
    pub fn pos(&self) -> Result<usize,ImageError>
    {
        match self.list.is_empty() {
            true   => Err(ImageError::NoImages),
            false  => Ok( self.list_index )
        }
    }

    pub fn len(&self) -> usize { self.list.len() }

    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    pub fn first(&mut self) -> Result<usize,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages)
        }
        else
        {
            self.list_index = 0;
            Ok( self.list_index )
        }
    }

    pub fn last(&mut self) -> Result<usize,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages)
        }
        else
        {
            self.list_index = self.list.len()-1;
            Ok( self.list_index )
        }
    }

    /// Positions relative to the current one, wrapping modularly at both
    /// ends (circular). peek_range(-2..=2) yields the 5 positions centered
    /// on the current image; offsets larger than the list wrap all the way
    /// around ( (pos+offset) mod len ).
    pub fn peek_range<R>(&self,r: R) -> PeekWalker
    where
        R: RangeBounds<isize>
    {
        let len = self.list.len();

        // For the iterator to just signal None
        if len == 0 {
            return PeekWalker { pos:0, len:1, count:0 };
        }

        let s = match r.start_bound() {
            Bound::Included(i)  => *i,
            Bound::Excluded(_e) => { panic!(); },
            Bound::Unbounded    => 0 , // this
        };

        let count = match r.end_bound() {
            Bound::Included(i) => { (i+1)-s },
            Bound::Excluded(e) => { e-s     },
            Bound::Unbounded   => 0,  // error
        }.max(0) as usize;

        // Start one step before the first yielded position; next() advances
        // then yields, so the walker emits pos+s, pos+s+1, ... (mod len).
        let start = (self.list_index as isize + s - 1).rem_euclid(len as isize) as usize;

        PeekWalker { pos:start, len, count }
    }


    pub fn key(&self) -> Result<ImageKey,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }
        Ok( self.list[self.list_index] )
    }

    pub fn key_at(&self, pos:usize) -> Result<ImageKey,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }
        self.list.get(pos).copied().ok_or(ImageError::IndexOverflow)
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
    pub fn item_at(&self, pos:usize) -> Result<&dyn ImageDyn,ImageError> {

        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }

        let key = match self.list.get(pos) {
            Some(k) => *k,
            None    => { return Err(ImageError::IndexOverflow); }
        };

        match self.store.get( &key ) {
            Some(s) => Ok( &**s ),
            None    => Err(ImageError::InvalidItemKey)
        }
    }


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

    pub fn goto(&mut self,pos:usize) -> Result<usize,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }

        if pos >= self.list.len() {
            return Err(ImageError::IndexOverflow);
        }

        self.list_index = pos;

        Ok( self.list_index )
    }

    pub fn random(&mut self) -> Result<usize,ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::NoImages);
        }
        self.list_index = fastrand::usize(0..self.list.len());
        Ok( self.list_index )
    }

    pub fn find_index(&self,key:ImageKey) -> Result<usize,ImageError> {
        match self.list.iter().position(|&i| i==key) {
            Some(idx) => Ok( idx ),
            None => { Err(ImageError::InvalidItemKey) }
        }
    }

    pub fn shuffle(&mut self) -> Result<(),ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::Uninitialized);
        }
        let idx = self.list_index;
        let cur = self.list[idx];

        fastrand::shuffle(&mut self.list);

        self.list_index = match self.list.iter().position(|&i| i==cur) {
            Some(idx) => idx,
            None => { return Err(ImageError::Unexpected); }
        };

        Ok(())
    }

    pub fn sort_size(&mut self) -> Result<(),ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::Uninitialized);
        }
        let idx = self.list_index;
        let cur = self.list[idx];

        self.list.sort_by( |a,b| {

            let aa = match self.store.get(a) {
                Some(a) => a.size(),
                None => { todo!(); }
            };
            let bb = match self.store.get(b) {
                Some(b) => b.size(),
                None  => { todo!(); }
            };

            aa.cmp(&bb)
        });

        self.list_index = match self.list.iter().position(|&i| i==cur) {
            Some(idx) => idx,
            None => { return Err(ImageError::Unexpected); }
        };

        Ok(())
    }

    pub fn sort_date(&mut self) -> Result<(),ImageError> {
        if self.list.is_empty() {
            return Err(ImageError::Uninitialized);
        }
        let idx = self.list_index;
        let cur = self.list[idx];

        self.list.sort_by( |a,b| {

            let aa = match self.store.get(a) {
                Some(a) => a.ftime(),
                None => { todo!(); }
            };
            let bb = match self.store.get(b) {
                Some(b) => b.ftime(),
                None  => { todo!(); }
            };

            aa.cmp(&bb)
        });

        self.list_index = match self.list.iter().position(|&i| i==cur) {
            Some(idx) => idx,
            None => { return Err(ImageError::Unexpected); }
        };

        Ok(())
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

    #[allow(unused)]
    pub fn sort_by<F>(&mut self, compare: F) -> Result<(),ImageError>
    where
        F: FnMut(&ImageKey, &ImageKey)-> Ordering
    {

        if self.list.is_empty() {
            return Err(ImageError::Uninitialized);
        }
        let idx = self.list_index;
        let cur = self.list[idx];

        self.list.sort_by( compare );

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
            let c = self.list_index+1;   // display is 1-based; the +1 lives at print time only

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
            store:      HashMap::new(),
            list:       Vec::new(),
            list_index: 0,
            next_key:   NonZeroUsize::new(0x10000EE5).expect("reality to be consistent"),
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::img_traits::ImageOrigin;

    #[derive(Debug,Clone)]
    struct TImg(String);

    impl Display for TImg {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
    }

    impl ImageOrigin for TImg {
        fn provider(&self) -> &str                { "test" }
        fn origin(&self)   -> &str                { "" }
        fn group(&self)    -> &str                { "" }
        fn name(&self)     -> &str                { &self.0 }
        fn display(&self)  -> String              { self.0.clone() }
        fn fqp(&self)      -> std::path::PathBuf  { std::path::PathBuf::from(&self.0) }
        fn size(&self)     -> u64                 { 0 }
        fn ftime(&self)    -> time::UtcDateTime   { time::UtcDateTime::UNIX_EPOCH }
    }

    fn make(n:usize) -> ImageList {
        let mut l = ImageList::new();
        l.append( (0..n).map(|i| Box::new(TImg(format!("img{i:03}"))) as Box<dyn ImageDyn>).collect() );
        l
    }

    #[test]
    fn empty_list_errors_and_yields_nothing() {
        let mut l = make(0);
        assert!(l.pos().is_err());
        assert!(l.first().is_err());
        assert!(l.last().is_err());
        assert!(l.key().is_err());
        assert_eq!(l.peek_range(-5..=5).next(), None);
    }

    #[test]
    fn goto_bounds_are_exclusive_of_len() {
        let mut l = make(5);
        assert!(l.goto(4).is_ok());
        assert!(matches!(l.goto(5), Err(ImageError::IndexOverflow)));
        assert!(l.key_at(4).is_ok());
        assert!(matches!(l.key_at(5), Err(ImageError::IndexOverflow)));
        assert!(l.item_at(4).is_ok());
        assert!(matches!(l.item_at(5), Err(ImageError::IndexOverflow)));
    }

    #[test]
    fn peek_steps_forward_and_back() {
        let mut l = make(10);
        l.goto(4).unwrap();
        assert_eq!(l.peek_range( 1..= 1).collect::<Vec<_>>(), vec![5]);
        assert_eq!(l.peek_range(-1..=-1).collect::<Vec<_>>(), vec![3]);
    }

    #[test]
    fn peek_window_is_centered_and_wraps() {
        let mut l = make(10);
        l.goto(0).unwrap();
        assert_eq!(l.peek_range(-2..=2).collect::<Vec<_>>(), vec![8,9,0,1,2]);
        l.goto(9).unwrap();
        assert_eq!(l.peek_range(-2..=2).collect::<Vec<_>>(), vec![7,8,9,0,1]);
    }

    #[test]
    fn peek_wraps_at_both_ends() {
        let mut l = make(5);
        l.goto(0).unwrap();
        assert_eq!(l.peek_range(-1..=-1).collect::<Vec<_>>(), vec![4]);
        l.goto(4).unwrap();
        assert_eq!(l.peek_range( 1..= 1).collect::<Vec<_>>(), vec![0]);
    }

    #[test]
    fn peek_offsets_larger_than_len_are_modular() {
        let mut l = make(5);
        l.goto(2).unwrap();
        // (2+100) mod 5 == 2, (2-100) mod 5 == 2
        assert_eq!(l.peek_range( 100..= 100).collect::<Vec<_>>(), vec![2]);
        assert_eq!(l.peek_range(-100..=-100).collect::<Vec<_>>(), vec![2]);
        // (2+101) mod 5 == 3, (2-101) mod 5 == 1
        assert_eq!(l.peek_range( 101..= 101).collect::<Vec<_>>(), vec![3]);
        assert_eq!(l.peek_range(-101..=-101).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn sort_and_shuffle_preserve_current_key() {
        let mut l = make(20);
        l.goto(13).unwrap();
        let key = l.key().unwrap();

        l.shuffle().unwrap();
        assert_eq!(l.key().unwrap(), key);

        l.sort().unwrap();
        assert_eq!(l.key().unwrap(), key);
        assert_eq!(l.pos().unwrap(), 13);   // name sort restores append order
    }

    #[test]
    fn find_index_round_trips_with_goto() {
        let mut l = make(7);
        let key = l.key_at(5).unwrap();
        let pos = l.find_index(key).unwrap();
        assert_eq!(pos, 5);
        l.goto(pos).unwrap();
        assert_eq!(l.key().unwrap(), key);
    }
}
