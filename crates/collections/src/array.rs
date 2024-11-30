use std::array;

pub fn array_from_iter<T, const N: usize>(it: impl IntoIterator<Item = T>) -> [T; N] {
    let mut it = it.into_iter();
    let arr = array::from_fn(|_| it.next().unwrap());
    assert!(it.next().is_none());
    arr
}
