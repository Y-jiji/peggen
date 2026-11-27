use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// One matched element. A variant's tag is pushed after its children and linked before them, so it is read first.
#[derive(Debug, Clone, Copy)]
pub enum Tag<'a> {
    /// A regex match: the matched slice.
    Text(&'a str),
    /// One variant's match: its `TVIx`. Read before its inner tags, pushed after them.
    Rule(TVIx),
    /// One `repeat`: how many items matched. Read before the items, pushed after them.
    Many(usize),
}

/// A peggen type's index: the high 16 bits of its `TVIx` and `TGIx`. Each type takes one from `TIX_COUNT` on its
/// first call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TIx(u16);

/// A variant's declaration index within its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VIx(pub u16);

/// A variant group's index within its type: 0 is the type's `#[parse]` variants in declaration order; any other list
/// gets a number from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GIx(pub u16);

/// A variant's number `tix<<16|vix`, pushed as the `Rule` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TVIx(u32);

/// A variant group's number `tix<<16|gix`; equal lists in equal order get the same one, wherever they are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TGIx(u32);

/// The next free `TIx`.
static TIX_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Every group handed out so far with a `GIx` from 1, with its list.
static GROUP_LIST: Mutex<Vec<(TGIx, Vec<VIx>)>> = Mutex::new(Vec::new());

impl TIx {
    /// Takes the next number from `TIX_COUNT`. Panics past 65,536 types.
    pub fn next() -> TIx {
        let number = TIX_COUNT.fetch_add(1, Ordering::Relaxed);
        TIx(u16::try_from(number).expect("peggen: more than 65,536 types"))
    }
}

impl TVIx {
    /// Variant `vix` of type `tix`.
    pub fn new(tix: TIx, vix: VIx) -> TVIx {
        TVIx(u32::from(tix.0) << 16 | u32::from(vix.0))
    }

    /// Its type.
    pub fn tix(self) -> TIx {
        TIx((self.0 >> 16) as u16)
    }

    /// Its variant within the type.
    pub fn vix(self) -> VIx {
        VIx((self.0 & 0xFFFF) as u16)
    }
}

impl TGIx {
    /// Group `gix` of type `tix`.
    pub fn new(tix: TIx, gix: GIx) -> TGIx {
        TGIx(u32::from(tix.0) << 16 | u32::from(gix.0))
    }

    /// The group of `list` in type `tix`: `GIx(0)` if it equals `parse`, else the `GIx` handed out before for the same
    /// list, or the next one. Panics past 65,536 lists for one type.
    pub fn of(tix: TIx, parse: &[VIx], list: &[VIx]) -> TGIx {
        if list == parse {
            return TGIx::new(tix, GIx(0));
        }
        let high = u32::from(tix.0) << 16;
        let mut known = GROUP_LIST.lock().expect("peggen: the group list was poisoned by an earlier panic");
        let same = known.iter().find(|(group, found)| group.0 & !0xFFFF == high && found.as_slice() == list);
        if let Some(&(group, _)) = same {
            return group;
        }
        let count = known.iter().filter(|(group, _)| group.0 & !0xFFFF == high).count() + 1;
        let number = u16::try_from(count).expect("peggen: more than 65,536 variant lists for one type");
        let group = TGIx::new(tix, GIx(number));
        known.push((group, list.to_vec()));
        group
    }
}
