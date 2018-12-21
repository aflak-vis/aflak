use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::Instant;

use dst::{Output, TransformIdx};
use timed::Timed;

use chashmap::CHashMap;

/// The Cache object used to run cached computations with cake
///
/// The cache is a concurrent hash-map, that will stay alive after
/// being dropped at least until the last worker using the cache releases it.
#[derive(Debug)]
pub struct Cache<T, E> {
    cache: CHashMap<TransformIdx, Option<CacheBox<T, E>>>,
    in_use: Arc<AtomicUsize>,
    scheduled_for_destruction: Arc<AtomicBool>,
}

impl<T, E> Default for Cache<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CacheRef<T, E> {
    inner: *const Cache<T, E>,
    in_use: Arc<AtomicUsize>,
    scheduled_for_destruction: Arc<AtomicBool>,
}

impl<T, E> Clone for CacheRef<T, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            in_use: self.in_use.clone(),
            scheduled_for_destruction: self.scheduled_for_destruction.clone(),
        }
    }
}

impl<T, E> CacheRef<T, E> {
    /// Compute and insert in cache *or* get from cache.
    /// Return None if the cache is scheduled for destruction.
    ///
    /// If cached value is present and newer than the providedd instant, then
    /// do not do the heavy computation and return the cached value.
    pub(crate) fn compute<F>(
        &self,
        t_idx: TransformIdx,
        t_instant: Instant,
        f: F,
    ) -> Option<Timed<Vec<Result<Arc<T>, Arc<E>>>>>
    where
        F: FnOnce() -> Vec<Result<Arc<T>, Arc<E>>>,
    {
        if self.scheduled_for_destruction.load(Ordering::Acquire) {
            None
        } else {
            self.in_use.fetch_add(1, Ordering::SeqCst);

            let ret = unsafe { (*self.inner).compute(t_idx, t_instant, f) };

            self.in_use.fetch_sub(1, Ordering::SeqCst);

            Some(ret)
        }
    }
}

unsafe impl<T, E> Sync for CacheRef<T, E> {}
unsafe impl<T, E> Send for CacheRef<T, E> {}

#[derive(Debug)]
struct CacheBox<T, E> {
    time: Instant,
    values: Vec<Result<Arc<T>, Arc<E>>>,
}

impl<T, E> Cache<T, E> {
    /// Initialize Cache
    pub fn new() -> Self {
        Self {
            cache: CHashMap::new(),
            in_use: Arc::new(AtomicUsize::new(0)),
            scheduled_for_destruction: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get currently cached value for given Input.
    /// The value may or may not have expired.
    pub fn get(&self, output: &Output) -> Option<Result<Arc<T>, Arc<E>>> {
        if let Some(some_cache_box) = self.cache.get(&output.t_idx) {
            if let Some(ref cache_box) = *some_cache_box {
                return cache_box.values.get(output.index()).cloned();
            }
        }
        None
    }

    pub(crate) fn get_ref(&self) -> CacheRef<T, E> {
        CacheRef {
            inner: self,
            in_use: self.in_use.clone(),
            scheduled_for_destruction: self.scheduled_for_destruction.clone(),
        }
    }

    pub(crate) fn init<I: Iterator<Item = TransformIdx>>(&mut self, ids: I) {
        for id in ids {
            if !self.cache.contains_key(&id) {
                self.cache.insert_new(id, None);
            }
        }
    }

    pub(crate) fn compute<F>(
        &self,
        t_idx: TransformIdx,
        t_instant: Instant,
        f: F,
    ) -> Timed<Vec<Result<Arc<T>, Arc<E>>>>
    where
        F: FnOnce() -> Vec<Result<Arc<T>, Arc<E>>>,
    {
        if let Some(some_cache_box) = self.cache.get(&t_idx) {
            if let Some(ref cache_box) = *some_cache_box {
                if cache_box.time >= t_instant {
                    return Timed::from_instant(cache_box.values.clone(), cache_box.time);
                }
            }
        }

        let result = f();

        let ret = result.clone();
        let mut some_cache_box = self.cache.get_mut(&t_idx).unwrap();
        *some_cache_box = Some(CacheBox {
            time: t_instant,
            values: result,
        });
        Timed::from_instant(ret, t_instant)
    }
}

impl<T, E> Drop for Cache<T, E> {
    fn drop(&mut self) {
        self.scheduled_for_destruction
            .store(true, Ordering::Release);
        while self.in_use.load(Ordering::Acquire) > 0 {
            thread::yield_now();
        }
    }
}
