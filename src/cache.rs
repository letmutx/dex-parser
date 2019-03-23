extern crate lru;

use std::cell::RefCell;
use std::cmp::Eq;
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

use lru::LruCache;

pub struct Ref<V>(Rc<V>);

impl<V> Ref<V> {
    pub fn new(value: V) -> Self {
        Ref(Rc::new(value))
    }
}

impl<V> Deref for Ref<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &*(self.0)
    }
}

impl<V> Clone for Ref<V> {
    fn clone(&self) -> Self {
        Ref(self.0.clone())
    }
}

pub(crate) struct Cache<K, V> {
    inner: Rc<RefCell<LruCache<K, Ref<V>>>>,
}

impl<K: Hash + Eq, V> Cache<K, V> {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            inner: Rc::new(RefCell::new(LruCache::new(cap))),
        }
    }

    pub(crate) fn get(&self, key: &K) -> Option<Ref<V>> {
        self.inner.borrow_mut().get(key).map(|v| v.clone())
    }

    pub(crate) fn put(&self, key: K, value: V) {
        self.inner.borrow_mut().put(key, Ref::new(value));
    }
}

impl<K, V> Clone for Cache<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
