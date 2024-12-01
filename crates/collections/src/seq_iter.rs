use crate::seq_buf::SeqBuf;

#[derive(Debug)]
pub struct SeqIter<T, I> {
    iter: I,
    buf: SeqBuf<T>,
}

impl<I, T> Iterator for SeqIter<T, I>
where
    I: Iterator<Item = (usize, T)>,
{
    type Item = (usize, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((idx, item)) = self.buf.pop() {
                return Some((idx, item));
            }
            if let Some((idx, item)) = self.iter.next() {
                self.buf.push(idx, item);
                continue;
            }
            return None;
        }
    }
}

impl<T, I> SeqIter<T, I> {
    pub fn new(iter: impl IntoIterator<IntoIter = I, Item = (usize, T)>) -> Self {
        Self {
            iter: iter.into_iter(),
            buf: SeqBuf::default(),
        }
    }
}
