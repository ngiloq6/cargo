use std::fmt;
use std::sync::RwLock;
use std::collections::HashSet;
use std::slice;
use std::str;
use std::mem;
use std::cmp::Ordering;

pub fn leek(s: String) -> &'static str {
    let boxed = s.into_boxed_str();
    let ptr = boxed.as_ptr();
    let len = boxed.len();
    mem::forget(boxed);
    unsafe {
        let slice = slice::from_raw_parts(ptr, len);
        str::from_utf8_unchecked(slice)
    }
}

lazy_static! {
    static ref STRING_CASHE: RwLock<HashSet<&'static str>> =
        RwLock::new(HashSet::new());
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub struct InternedString {
    ptr: *const u8,
    len: usize,
}

impl InternedString {
    pub fn new(str: &str) -> InternedString {
        let mut cache = STRING_CASHE.write().unwrap();
        if let Some(&s) = cache.get(str) {
            return InternedString { ptr: s.as_ptr(), len: s.len() };
        }
        let s = leek(str.to_string());
        cache.insert(s);
        InternedString { ptr: s.as_ptr(), len: s.len() }
    }
    pub fn to_inner(&self) -> &'static str {
        unsafe {
            let slice = slice::from_raw_parts(self.ptr, self.len);
            str::from_utf8_unchecked(slice)
        }
    }
}

impl fmt::Debug for InternedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InternedString {{ {} }}", self.to_inner())
    }
}

impl Ord for InternedString {
    fn cmp(&self, other: &InternedString) -> Ordering {
        self.to_inner().cmp(&other.to_inner())
    }
}

impl PartialOrd for InternedString {
    fn partial_cmp(&self, other: &InternedString) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

unsafe impl Send for InternedString {}
unsafe impl Sync for InternedString {}