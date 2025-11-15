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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== BytestringPool / ByteInterner Tests =====

    #[test]
    fn test_interner_basic() {
        let mut pool = BytestringPool::new();
        let sym1 = pool.get_or_intern(b"hello");
        let sym2 = pool.get_or_intern(b"world");
        let sym3 = pool.get_or_intern(b"hello"); // Same as sym1

        assert_eq!(sym1, sym3); // Same string gets same Sym
        assert_ne!(sym1, sym2); // Different strings get different Syms
    }

    #[test]
    fn test_interner_resolve() {
        let mut pool = BytestringPool::new();
        let sym = pool.get_or_intern(b"test");
        let resolved = pool.resolve(sym);

        assert_eq!(*resolved, *b"test");
    }

    #[test]
    fn test_interner_empty_string() {
        let mut pool = BytestringPool::new();
        let sym1 = pool.get_or_intern(b"");
        let sym2 = pool.get_or_intern(b"");

        assert_eq!(sym1, sym2);
        assert_eq!(*pool.resolve(sym1), *b"");
    }

    #[test]
    fn test_interner_unicode() {
        let mut pool = BytestringPool::new();
        let sym1 = pool.get_or_intern("hello".as_bytes());
        let sym2 = pool.get_or_intern("café".as_bytes());
        let sym3 = pool.get_or_intern("你好".as_bytes());

        assert_ne!(sym1, sym2);
        assert_ne!(sym2, sym3);
        assert_eq!(*pool.resolve(sym2), *"café".as_bytes());
        assert_eq!(*pool.resolve(sym3), *"你好".as_bytes());
    }

    #[test]
    fn test_interner_multiple_strings() {
        let mut pool = BytestringPool::new();
        let strings: Vec<&[u8]> = vec![b"one", b"two", b"three", b"four", b"five"];
        let mut syms = Vec::new();

        // Intern all strings
        for s in &strings {
            syms.push(pool.get_or_intern(*s));
        }

        // Verify all are different
        for i in 0..syms.len() {
            for j in (i + 1)..syms.len() {
                assert_ne!(syms[i], syms[j]);
            }
        }

        // Verify resolution works
        for (sym, orig) in syms.iter().zip(strings.iter()) {
            assert_eq!(*pool.resolve(*sym), **orig);
        }
    }

    #[test]
    fn test_interner_clone() {
        let mut pool1 = BytestringPool::new();
        let sym1 = pool1.get_or_intern(b"test");

        let mut pool2 = pool1.clone();
        let sym2 = pool2.get_or_intern(b"test");

        // Cloned pool shares the same interner (Rc)
        assert_eq!(sym1, sym2);
    }

    // ===== bs_split_once Tests =====

    #[test]
    fn test_split_once() {
        // Basic split
        assert_eq!(bs_split_once(b"key=value", b'='), Some((b"key" as &[u8], b"value" as &[u8])));

        // No delimiter found
        assert_eq!(bs_split_once(b"nodelimiter", b'='), None);
        assert_eq!(bs_split_once(b"", b'='), None);

        // Delimiter at boundaries
        assert_eq!(bs_split_once(b"=value", b'='), Some((b"" as &[u8], b"value" as &[u8])));
        assert_eq!(bs_split_once(b"key=", b'='), Some((b"key" as &[u8], b"" as &[u8])));

        // Multiple delimiters (splits at first)
        assert_eq!(bs_split_once(b"a:b:c", b':'), Some((b"a" as &[u8], b"b:c" as &[u8])));

        // Tab delimiter (CoNLL-U use case)
        assert_eq!(bs_split_once(b"field1\tfield2", b'\t'), Some((b"field1" as &[u8], b"field2" as &[u8])));
    }

    // ===== bs_trim Tests =====

    #[test]
    fn test_trim() {
        // With newline at end
        assert_eq!(bs_trim(b"hello\n"), b"hello");
        assert_eq!(bs_trim(b"\n"), b"");

        // Without newline
        assert_eq!(bs_trim(b"hello"), b"hello");
        assert_eq!(bs_trim(b""), b"");

        // Newline in middle (truncates at first \n)
        assert_eq!(bs_trim(b"hello\nworld"), b"hello");
        assert_eq!(bs_trim(b"hello\n\n"), b"hello");

        // Carriage return (only removes \n, not \r)
        assert_eq!(bs_trim(b"hello\r\n"), b"hello\r");
    }

    // ===== bs_atoi Tests =====

    #[test]
    fn test_atoi_valid() {
        // Valid numbers
        assert_eq!(bs_atoi(b"0"), Some(0));
        assert_eq!(bs_atoi(b"1"), Some(1));
        assert_eq!(bs_atoi(b"42"), Some(42));
        assert_eq!(bs_atoi(b"123456"), Some(123456));

        // Empty string returns Some(0)
        assert_eq!(bs_atoi(b""), Some(0));

        // Leading zeros
        assert_eq!(bs_atoi(b"007"), Some(7));
        assert_eq!(bs_atoi(b"00000"), Some(0));

        // Large numbers
        assert_eq!(bs_atoi(b"18446744073709551615"), Some(usize::MAX));
    }

    #[test]
    fn test_atoi_invalid() {
        // Letters and mixed
        assert_eq!(bs_atoi(b"abc"), None);
        assert_eq!(bs_atoi(b"12a"), None);
        assert_eq!(bs_atoi(b"a12"), None);

        // Punctuation and signs
        assert_eq!(bs_atoi(b"1.23"), None);
        assert_eq!(bs_atoi(b"-42"), None);
        assert_eq!(bs_atoi(b"+42"), None);
        assert_eq!(bs_atoi(b"12,345"), None);
        assert_eq!(bs_atoi(b"0x42"), None);
        assert_eq!(bs_atoi(b"!@#$"), None);

        // Whitespace
        assert_eq!(bs_atoi(b" 42"), None);
        assert_eq!(bs_atoi(b"42 "), None);
        assert_eq!(bs_atoi(b" "), None);

        // Overflow
        assert_eq!(bs_atoi(b"18446744073709551616"), None); // usize::MAX + 1
        assert_eq!(bs_atoi(b"99999999999999999999"), None);
    }
}
