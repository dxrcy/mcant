use std::collections::HashMap;
use std::time::{Duration, Instant};

use mcrs::{Block, Coordinate, Size};

pub struct World {
    mc: mcrs::Connection,
    cache: Cache,
}

struct Cache {
    entries: HashMap<Coordinate, CacheEntry>,
    next_clean: Instant,
    /// Each side of cube are length `cache_size * 2 + 1`.
    chunk_radius: u32,
    /// Maximum lifetime for a cache entry.
    max_lifetime: Duration,
}

struct CacheEntry {
    block: Block,
    expiration: Instant,
}

impl World {
    pub fn new(mc: mcrs::Connection, cache_size: u32, cache_time: Duration) -> Self {
        Self {
            mc,
            cache: Cache::new(cache_size, cache_time),
        }
    }

    pub fn get_mc(&mut self) -> &mut mcrs::Connection {
        &mut self.mc
    }

    pub fn get_block(&mut self, location: Coordinate) -> Result<Block, mcrs::Error> {
        if !self.cache.enabled() {
            return self.mc.get_block(location);
        }

        if let Some(block) = self.cache.get(location) {
            return Ok(block);
        }

        self.cache.clean(location);

        let (origin, bound) = self.cache.get_chunk(location);
        let chunk = self.mc.get_blocks(origin, bound)?;
        for entry in &chunk {
            self.cache
                .insert(location, entry.position_worldspace(), entry.block());
        }

        Ok(chunk
            .get_worldspace(location)
            .expect("block should be in chunk"))
    }

    pub fn set_block(&mut self, location: Coordinate, block: Block) -> Result<(), mcrs::Error> {
        if self
            .cache
            .get(location)
            .is_some_and(|cached| cached == block)
        {
            return Ok(());
        }

        self.cache.clean(location);
        self.cache.insert(location, location, block);
        self.mc.set_block(location, block)
    }
}

impl Cache {
    const CACHE_COOLDOWN: Duration = Duration::from_secs(4);
    const MAX_DISTANCE: u32 = 16;

    pub fn new(chunk_radius: u32, max_lifetime: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            next_clean: Instant::now() + Self::CACHE_COOLDOWN,
            chunk_radius,
            max_lifetime,
        }
    }

    pub fn get_chunk(&self, location: Coordinate) -> (Coordinate, Coordinate) {
        debug_assert!(self.enabled());
        let size_half = Size::new(self.chunk_radius, self.chunk_radius, self.chunk_radius);
        (location - size_half, location + size_half)
    }

    /// Call before inserting cache.
    pub fn clean(&mut self, origin: Coordinate) {
        if !self.enabled() {
            return;
        }

        let now = Instant::now();
        if now < self.next_clean {
            return;
        }

        self.entries.retain(|location, entry| {
            now <= entry.expiration && manhattan_distance(origin, *location) <= Self::MAX_DISTANCE
        });

        self.next_clean = now + Self::CACHE_COOLDOWN;
    }

    pub fn get(&mut self, location: Coordinate) -> Option<Block> {
        if !self.enabled() {
            return None;
        }

        let entry = self.entries.get(&location)?;
        if Instant::now() > entry.expiration {
            self.entries.remove(&location);
            return None;
        }
        Some(entry.block)
    }

    pub fn insert(&mut self, origin: Coordinate, location: Coordinate, block: Block) {
        if !self.enabled() {
            return;
        }

        self.entries.insert(
            location,
            CacheEntry {
                block,
                expiration: self.calculate_expiration(origin, location),
            },
        );
    }

    pub fn enabled(&self) -> bool {
        self.chunk_radius > 0 && self.max_lifetime.as_millis() > 0
    }

    fn calculate_expiration(&self, origin: Coordinate, location: Coordinate) -> Instant {
        let dist = manhattan_distance(origin, location);
        let max_dist = self.chunk_radius * 3;
        let lifetime = (self.max_lifetime * (max_dist - dist)) / max_dist;
        Instant::now() + lifetime
    }
}

fn manhattan_distance(a: Coordinate, b: Coordinate) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y) + a.z.abs_diff(b.z)
}
