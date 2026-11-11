//! Snapshot-based undo / redo.

/// A bounded snapshot undo stack. Call [`History::record`] with the state *before* a change,
/// then [`History::undo`] / [`History::redo`] swap snapshots with the live value.
///
/// ```
/// let mut doc = vec![1];
/// let mut h = glowkit_canvas::History::new(50);
/// h.record(&doc);
/// doc.push(2);
/// assert!(h.undo(&mut doc));
/// assert_eq!(doc, vec![1]);
/// assert!(h.redo(&mut doc));
/// assert_eq!(doc, vec![1, 2]);
/// ```
#[derive(Clone, Debug)]
pub struct History<T: Clone> {
    undo: Vec<T>,
    redo: Vec<T>,
    limit: usize,
}

impl<T: Clone> Default for History<T> {
    fn default() -> Self {
        Self::new(100)
    }
}

impl<T: Clone> History<T> {
    /// A history keeping at most `limit` undo steps.
    pub fn new(limit: usize) -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), limit: limit.max(1) }
    }

    /// Record `before` (the state prior to a change). Clears the redo stack.
    pub fn record(&mut self, before: &T) {
        self.undo.push(before.clone());
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Restore the previous snapshot into `current`. Returns `false` if there was nothing to undo.
    pub fn undo(&mut self, current: &mut T) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(current, prev));
                true
            }
            None => false,
        }
    }

    /// Re-apply the last undone snapshot. Returns `false` if there was nothing to redo.
    pub fn redo(&mut self, current: &mut T) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(current, next));
                true
            }
            None => false,
        }
    }

    /// Can [`History::undo`] do something?
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Can [`History::redo`] do something?
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Number of undo steps available.
    pub fn len(&self) -> usize {
        self.undo.len()
    }

    /// No undo steps available?
    pub fn is_empty(&self) -> bool {
        self.undo.is_empty()
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_cycle() {
        let mut v = 0;
        let mut h = History::new(10);
        for i in 1..=3 {
            h.record(&v);
            v = i;
        }
        assert!(h.undo(&mut v));
        assert_eq!(v, 2);
        assert!(h.undo(&mut v));
        assert_eq!(v, 1);
        assert!(h.redo(&mut v));
        assert_eq!(v, 2);
        // a new change clears redo
        h.record(&v);
        v = 42;
        assert!(!h.can_redo());
        assert!(h.undo(&mut v));
        assert_eq!(v, 2);
    }

    #[test]
    fn limit_is_respected() {
        let mut v = 0;
        let mut h = History::new(3);
        for i in 1..=10 {
            h.record(&v);
            v = i;
        }
        assert_eq!(h.len(), 3);
        while h.undo(&mut v) {}
        assert_eq!(v, 7);
    }
}
