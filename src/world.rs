use std::collections::HashMap;
use std::time::{Duration, Instant};

use mcrs::{Block, Coordinate, Size};

pub struct World {
    mc: mcrs::Connection,
    cache: HashMap<Coordinate, CacheEntry>,

    /// Each side of cube are length `cache_size * 2 + 1`.
    cache_size: u32,
    /// Maximum lifetime for a cache entry.
    cache_time: Duration,
}

struct CacheEntry {
    block: Block,
    expiration: Instant,
}

// FIXME: Clean expired cache

impl World {
    pub fn new(mc: mcrs::Connection, cache_size: u32, cache_time: Duration) -> Self {
        Self {
            mc,
            cache: HashMap::new(),
            cache_size,
            cache_time,
        }
    }

    pub fn get_mc(&mut self) -> &mut mcrs::Connection {
        &mut self.mc
    }

    pub fn get_block(&mut self, location: Coordinate) -> Result<Block, mcrs::Error> {
        if let Some(block) = self.get_cache(location) {
            return Ok(block);
        }

        let size_half = Size::new(self.cache_size, self.cache_size, self.cache_size);

        let origin = location - size_half;
        let bound = location + size_half;

        let chunk = self.mc.get_blocks(origin, bound)?;
        for entry in &chunk {
            self.insert_cache(location, entry.position_worldspace(), entry.block());
        }

        Ok(chunk
            .get_worldspace(location)
            .expect("block should be in chunk"))
    }

    pub fn set_block(&mut self, location: Coordinate, block: Block) -> Result<(), mcrs::Error> {
        if self
            .get_cache(location)
            .is_some_and(|cached| cached == block)
        {
            return Ok(());
        }

        self.insert_cache(location, location, block);
        self.mc.set_block(location, block)
    }

    fn get_cache(&mut self, location: Coordinate) -> Option<Block> {
        if !self.cache_enabled() {
            return None;
        }

        let entry = self.cache.get(&location)?;
        if Instant::now() > entry.expiration {
            self.cache.remove(&location);
            return None;
        }
        Some(entry.block)
    }

    fn insert_cache(&mut self, origin: Coordinate, location: Coordinate, block: Block) {
        if !self.cache_enabled() {
            return;
        }

        self.cache.insert(
            location,
            CacheEntry {
                block,
                expiration: self.calculate_expiration(origin, location),
            },
        );
    }

    fn cache_enabled(&self) -> bool {
        self.cache_size > 0 && self.cache_time.as_millis() > 0
    }

    fn calculate_expiration(&self, origin: Coordinate, location: Coordinate) -> Instant {
        let dist = manhattan_distance(origin, location);
        let max_dist = self.cache_size * 3;
        let lifetime = (self.cache_time * (max_dist - dist)) / max_dist;
        Instant::now() + lifetime
    }
}

fn manhattan_distance(a: Coordinate, b: Coordinate) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y) + a.z.abs_diff(b.z)
}
