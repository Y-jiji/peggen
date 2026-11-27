/// Items in reading order. They are stored in push order; links let an item pushed later be read before earlier ones.
pub(crate) struct Chain<T> {
    /// Every item pushed and not rewound, in push order.
    stack: Vec<Node<T>>,
    /// The first item in reading order, or `NO_NODE`.
    first: usize,
    /// The last item in reading order, or `NO_NODE`.
    trail: usize,
}

/// One item and the item read after it.
struct Node<T> {
    next: usize,
    data: T,
}

/// A point in a `Chain`: how many items it held, and which one was last in reading order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Snapshot {
    stack: usize,
    trail: usize,
}

/// Stands for "no item" in `first`, `trail` and `next`.
const NO_NODE: usize = usize::MAX;

impl<T> Chain<T> {
    /// No items.
    pub(crate) fn new() -> Self {
        Chain { stack: Vec::new(), first: NO_NODE, trail: NO_NODE }
    }

    /// The current point.
    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot { stack: self.stack.len(), trail: self.trail }
    }

    /// Appends `data` and links it, in reading order, right before every item pushed since `snap`.
    /// With the current snapshot, that is the end of the reading order.
    /// Panics if `snap` holds more items than the chain.
    /// Caller obligation: `snap` was taken by a scope, repetition or call that is still running.
    pub(crate) fn push(&mut self, data: T, snap: Snapshot) {
        let index = self.stack.len();
        assert!(snap.stack <= index, "peggen: `Chain::push` got a snapshot from a later point");
        if snap.stack == index {
            self.stack.push(Node { next: NO_NODE, data });
            match self.trail {
                NO_NODE => self.first = index,
                trail => self.stack[trail].next = index,
            }
            self.trail = index;
            return;
        }
        let next = match snap.trail {
            NO_NODE => std::mem::replace(&mut self.first, index),
            trail => std::mem::replace(&mut self.stack[trail].next, index),
        };
        self.stack.push(Node { next, data });
    }

    /// Drops every item pushed since `snap`. Panics if `snap` holds more items than the chain.
    pub(crate) fn rewind(&mut self, snap: Snapshot) {
        assert!(snap.stack <= self.stack.len(), "peggen: `Chain::rewind` got a snapshot from a later point");
        self.stack.truncate(snap.stack);
        self.trail = snap.trail;
        if snap.trail == NO_NODE {
            self.first = NO_NODE;
        }
    }

    /// The item pushed last, if any.
    pub(crate) fn last(&self) -> Option<&T> {
        self.stack.last().map(|node| &node.data)
    }

    /// The items in reading order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        let mut place = self.first;
        let mut done = self.trail == NO_NODE;
        std::iter::from_fn(move || {
            if done {
                return None;
            }
            let node = &self.stack[place];
            done = place == self.trail;
            place = node.next;
            Some(&node.data)
        })
    }
}
