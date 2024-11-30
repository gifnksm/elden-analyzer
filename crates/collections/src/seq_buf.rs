use std::collections::BinaryHeap;

#[derive(Debug)]
struct Pri<T> {
    idx: usize,
    data: T,
}

impl<T> PartialEq for Pri<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T> Eq for Pri<T> {}

impl<T> PartialOrd for Pri<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Self::cmp(self, other))
    }
}

impl<T> Ord for Pri<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx).reverse()
    }
}

#[derive(Debug)]
pub struct SeqBuf<T> {
    buf: BinaryHeap<Pri<T>>,
    wants: usize,
}

impl<T> Default for SeqBuf<T> {
    fn default() -> Self {
        Self {
            buf: Default::default(),
            wants: 0,
        }
    }
}

impl<T> SeqBuf<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, idx: usize, data: T) {
        assert!(idx >= self.wants);
        self.buf.push(Pri { idx, data });
    }

    pub fn pop(&mut self) -> Option<(usize, T)> {
        if self.buf.peek().map(|p| p.idx) == Some(self.wants) {
            self.wants += 1;
            return self.buf.pop().map(|p| (p.idx, p.data));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::SeqBuf;

    #[test]
    fn basic() {
        let mut buf = SeqBuf::new();
        buf.push(1, "one");
        buf.push(3, "three");
        buf.push(2, "two");
        buf.push(5, "five");
        assert_eq!(buf.pop(), None);
        buf.push(0, "zero");
        assert_eq!(buf.pop(), Some((0, "zero")));
        assert_eq!(buf.pop(), Some((1, "one")));
        assert_eq!(buf.pop(), Some((2, "two")));
        assert_eq!(buf.pop(), Some((3, "three")));
        assert_eq!(buf.pop(), None);
        buf.push(4, "four");
        assert_eq!(buf.pop(), Some((4, "four")));
        assert_eq!(buf.pop(), Some((5, "five")));
    }
}
