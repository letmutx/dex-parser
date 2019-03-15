extern crate lru;
use std::cell::RefCell;
use std::cmp::Eq;
use std::hash::Hash;
use std::rc::Rc;

use lru::LruCache;

pub(crate) struct Cache<K, V> {
    inner: RefCell<LruCache<K, Rc<V>>>,
}

impl<K: Hash + Eq, V> Cache<K, V> {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            inner: RefCell::new(LruCache::new(cap)),
        }
    }

    pub(crate) fn get<'a>(&'a self, key: &K) -> Option<Rc<V>> {
        self.inner.borrow_mut().get(key).map(|v| v.clone())
    }

    pub(crate) fn put(&self, key: K, value: V) {
        self.inner.borrow_mut().put(key, Rc::new(value));
    }
}
