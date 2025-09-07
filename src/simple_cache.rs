use std::{
    collections::HashMap,
    hash::Hash,
    time::{Duration, SystemTime},
};

#[derive(Debug)]
pub struct SimpleCache<G, T> {
    cache: HashMap<G, CacheEntry<T>>,
    config: CacheConfig,
}

#[derive(Clone, Debug, Default)]
pub struct CacheConfig {
    pub ttl: Duration,
    pub max_size: usize,
}

#[derive(Clone, Debug)]
struct CacheEntry<T> {
    data: T,
    timestamp: SystemTime,
}

impl<G: Hash + Eq + Clone, T: Clone> SimpleCache<G, T> {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: HashMap::new(),
            config,
        }
    }

    pub fn get(&mut self, key: &G) -> Option<T> {
        let entry = (*self.cache.get(key)?).clone();
        if entry.timestamp.elapsed().expect("Should be always ok") < self.config.ttl {
            Some(entry.data)
        } else {
            self.cache.remove(key);
            None
        }
    }

    pub fn set(&mut self, key: G, data: T) {
        let entry = CacheEntry {
            data,
            timestamp: SystemTime::now(),
        };
        self.cache.insert(key, entry);
        if self.cache.len() > self.config.max_size {
            self.cleanup();
        }
    }
    pub fn cleanup(&mut self) {
        self.cache.retain(|_, entry| {
            entry.timestamp.elapsed().expect("Should be always ok") < self.config.ttl
        });
    }
}
