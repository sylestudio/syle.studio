//! Generic undo/redo history. Pure and dependency-free so it is unit-tested on
//! the host even though its only consumer — the CRM block editor — compiles to
//! wasm. It holds a `present` value flanked by past/future stacks. `record` is
//! a no-op when the next state equals the present, so a debounced checkpoint
//! that fires with no real change never clobbers the redo stack.

/// Default number of undo steps retained.
const DEFAULT_LIMIT: usize = 100;

/// An undo/redo stack over snapshots of `T`.
pub struct History<T> {
    past: Vec<T>,
    present: T,
    future: Vec<T>,
    limit: usize,
}

impl<T> History<T> {
    /// A history seeded with `initial` as the present and the default depth.
    pub fn new(initial: T) -> Self {
        Self::with_limit(initial, DEFAULT_LIMIT)
    }

    /// A history retaining at most `limit` (≥ 1) undo steps.
    pub fn with_limit(initial: T, limit: usize) -> Self {
        History {
            past: Vec::new(),
            present: initial,
            future: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// The current state.
    pub fn present(&self) -> &T {
        &self.present
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Restore the previous state, returning it; `None` if nothing to undo. The
    /// state left behind becomes redoable.
    pub fn undo(&mut self) -> Option<&T> {
        let prev = self.past.pop()?;
        let cur = std::mem::replace(&mut self.present, prev);
        self.future.push(cur);
        Some(&self.present)
    }

    /// Re-apply the next state, returning it; `None` if nothing to redo.
    pub fn redo(&mut self) -> Option<&T> {
        let next = self.future.pop()?;
        let cur = std::mem::replace(&mut self.present, next);
        self.past.push(cur);
        Some(&self.present)
    }
}

impl<T: PartialEq> History<T> {
    /// Make `next` the present, pushing the old present onto the undo stack and
    /// discarding the redo stack. A `next` equal to the present is ignored — so
    /// a redundant checkpoint never clears redo — and the undo stack is capped
    /// at `limit`, dropping the oldest step.
    pub fn record(&mut self, next: T) {
        if next == self.present {
            return;
        }
        self.future.clear();
        let prev = std::mem::replace(&mut self.present, next);
        self.past.push(prev);
        if self.past.len() > self.limit {
            self.past.remove(0);
        }
    }
}
