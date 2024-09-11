#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neighbor {
    M,
    Z,
    P,
}

use Neighbor::*;

impl Neighbor {
    const DIFF: [(Self, Self); 8] = [
        (M, M),
        (Z, M),
        (P, M),
        (M, Z),
        (P, Z),
        (M, P),
        (Z, P),
        (P, P),
    ];

    pub fn comp(&self, n: usize, r: std::ops::Range<usize>) -> Option<usize> {
        let v = match self {
            M => n.checked_sub(1)?,
            Z => n,
            P => n.checked_add(1)?,
        };
        r.contains(&v).then_some(v)
    }

    pub fn neighbors_in(
        (x, y): (usize, usize),
        width: usize,
        height: usize,
    ) -> impl Iterator<Item = (usize, usize)> {
        Self::DIFF.into_iter().filter_map(move |(dx, dy)| {
            let fx = dx.comp(x, 0..width)?;
            let fy = dy.comp(y, 0..height)?;
            Some((fx, fy))
        })
    }
}
