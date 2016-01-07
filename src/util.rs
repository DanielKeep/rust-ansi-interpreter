use smallvec::{Array, SmallVec};

pub trait Counted {
    fn counted(&self) -> usize;
}

pub struct CountedIter<I>
where I: Iterator {
    iter: I,
    count: usize,
}

impl<I> CountedIter<I>
where I: Iterator {
    pub fn new(iter: I) -> Self {
        CountedIter {
            iter: iter,
            count: 0,
        }
    }
}

impl<I> Counted for CountedIter<I>
where I: Iterator {
    fn counted(&self) -> usize {
        self.count
    }
}

impl<I> Iterator for CountedIter<I>
where I: Iterator {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        match self.iter.next() {
            Some(v) => {
                self.count += 1;
                Some(v)
            },
            None => None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// FIXME: This sucks.
pub fn drop_front<A, T>(sv: &mut SmallVec<A>, n: usize)
where
    A: Array<Item=T>,
    T: Clone,
{
    assert!(n <= sv.len());

    let tmp = sv.iter().skip(n).cloned().collect();
    ::std::mem::replace(sv, tmp);
}
