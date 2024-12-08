use std::{cell::RefCell, cmp::Eq, hash::Hash, num::NonZeroUsize, rc::Rc};

use lru::LruCache;

/// LRU cache that provides interior mutability
#[derive(Debug)]
pub(crate) struct Cache<K: Hash + Eq, V> {
    inner: Rc<RefCell<LruCache<K, V>>>,
}

impl<K: Hash + Eq, V: Clone> Cache<K, V> {
    /// Get a new instance of cache with the given capacity
    pub(crate) fn new(cap: NonZeroUsize) -> Self {
        Self {
            inner: Rc::new(RefCell::new(LruCache::new(cap))),
        }
    }

    /// Get a reference to the value at key from the cache, if found
    pub(crate) fn get(&self, key: &K) -> Option<V> {
        self.inner
            .borrow_mut()
            .get(key)
            .map(std::clone::Clone::clone)
    }

    /// Insert a new key value pair into the cache
    pub(crate) fn put(&self, key: K, value: V) {
        self.inner.borrow_mut().put(key, value);
    }
}

impl<K: Hash + Eq, V> Clone for Cache<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
