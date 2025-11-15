use hashbrown::HashMap;
use hashbrown::hash_map::RawEntryMut;
use rustc_hash::{FxBuildHasher, FxHasher};
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;

pub const STRING_POOL_CAPACITY: usize = 5000;

#[derive(Clone, Debug)]
pub struct BytestringPool(Rc<RefCell<ByteInterner>>);

impl BytestringPool {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(ByteInterner::with_capacity(
            STRING_POOL_CAPACITY,
        ))))
    }

    #[inline]
    pub fn get_or_intern(&mut self, bytes: &[u8]) -> Sym {
        self.0.borrow_mut().get_or_intern(bytes)
    }

    #[inline]
    pub fn resolve(&self, s: Sym) -> Arc<[u8]> {
        self.0.borrow().resolve(s)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Sym(NonZeroU32); // 0 reserved as "invalid"

#[derive(Debug)]
pub struct ByteInterner {
    map: HashMap<Arc<[u8]>, Sym, FxBuildHasher>,
    slab: Vec<Arc<[u8]>>, // index = Sym-1
}

impl ByteInterner {
    pub fn new() -> Self {
        Self {
            map: HashMap::with_hasher(FxBuildHasher::default()),
            slab: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(cap, FxBuildHasher::default()),
            slab: Vec::with_capacity(cap),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    #[inline]
    pub fn get_or_intern(&mut self, bytes: &[u8]) -> Sym {
        let mut h = FxHasher::default();
        bytes.hash(&mut h);
        let hash = h.finish();
        match self
            .map
            .raw_entry_mut()
            .from_key_hashed_nocheck(hash, bytes)
        {
            RawEntryMut::Occupied(o) => *o.get(),
            RawEntryMut::Vacant(v) => {
                let idx = self.slab.len() as u32 + 1;
                let sym = Sym(NonZeroU32::new(idx).unwrap());
                let owned: Arc<[u8]> = Arc::from(bytes);
                // reuse the hash we computed for the &[u8] (content-equal)
                v.insert_hashed_nocheck(hash, owned.clone(), sym);
                self.slab.push(owned);
                sym
            }
        }
    }

    #[inline]
    pub fn resolve(&self, s: Sym) -> Arc<[u8]> {
        self.slab[(s.0.get() - 1) as usize].clone()
    }
}

// Divide a bytestring into two at delim
#[inline]
pub fn bs_split_once(bytes: &[u8], delim: u8) -> Option<(&[u8], &[u8])> {
    let mut pair = bytes.splitn(2, |b| *b == delim);
    Some((pair.next()?, pair.next()?))
}

// Remove line-feed from end of a bytestring
#[inline]
pub fn bs_trim(bytes: &[u8]) -> &[u8] {
    if let Some(idx) = bytes.iter().position(|b| *b == b'\n') {
        &bytes[..idx]
    } else {
        bytes
    }
}

#[inline]
pub fn bs_atoi(bytes: &[u8]) -> Option<usize> {
    let mut n: usize = 0;

    // Fast path: empty slice -> Some(0)? (You can change this if you prefer None.)
    if bytes.is_empty() {
        return Some(0);
    }

    for &b in bytes {
        // Convert ASCII digit to value 0..9; reject non-digits.
        let d = (b.wrapping_sub(b'0')) as usize;
        if d > 9 {
            return None;
        }

        // n = n*10 + d, but detect overflow on both steps.
        let (n10, of_mul) = n.overflowing_mul(10);
        let (sum, of_add) = n10.overflowing_add(d);
        if of_mul || of_add {
            return None;
        }
        n = sum;
    }
    Some(n)
}
