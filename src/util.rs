use smallvec::{Array, SmallVec};

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
