//! Stagger ordering shared by the per-letter effects.

/// Which piece animates first in a staggered sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StaggerFrom {
    /// First to last (left to right).
    #[default]
    Start,
    /// Last to first (right to left).
    End,
    /// From the middle outward.
    Center,
    /// Shuffled order.
    Random,
}

/// The stagger rank of each of `n` pieces (rank 0 goes first).
pub(crate) fn stagger_ranks(n: usize, from: StaggerFrom, seed: u64) -> Vec<usize> {
    let order: Vec<usize> = match from {
        StaggerFrom::Start => return (0..n).collect(),
        StaggerFrom::End => return (0..n).map(|i| n - 1 - i).collect(),
        StaggerFrom::Center => {
            let c = (n as f32 - 1.0) / 2.0;
            let mut order: Vec<usize> = (0..n).collect();
            order.sort_by(|a, b| (*a as f32 - c).abs().total_cmp(&(*b as f32 - c).abs()));
            order
        }
        StaggerFrom::Random => {
            let mut order: Vec<usize> = (0..n).collect();
            let mut rng = glowkit_core::Rng::new(seed);
            for i in (1..n).rev() {
                order.swap(i, rng.below(i + 1));
            }
            order
        }
    };
    let mut ranks = vec![0; n];
    for (r, i) in order.into_iter().enumerate() {
        ranks[i] = r;
    }
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_perm(r: &[usize]) -> bool {
        let mut s = r.to_vec();
        s.sort_unstable();
        s.iter().enumerate().all(|(i, v)| i == *v)
    }

    #[test]
    fn ranks_are_permutations() {
        for from in [StaggerFrom::Start, StaggerFrom::End, StaggerFrom::Center, StaggerFrom::Random] {
            for n in [0, 1, 2, 7, 20] {
                let r = stagger_ranks(n, from, 42);
                assert_eq!(r.len(), n);
                assert!(is_perm(&r), "{from:?} {n}");
            }
        }
    }

    #[test]
    fn rank_orders() {
        assert_eq!(stagger_ranks(4, StaggerFrom::Start, 0), vec![0, 1, 2, 3]);
        assert_eq!(stagger_ranks(4, StaggerFrom::End, 0), vec![3, 2, 1, 0]);
        let c = stagger_ranks(5, StaggerFrom::Center, 0);
        assert_eq!(c[2], 0);
        assert!(c[0] >= 3 && c[4] >= 3);
    }
}
