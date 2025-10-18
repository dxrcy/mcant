use std::time::Instant;
use std::{collections::HashMap, time::Duration};

use mcrs::{Block, Coordinate, Size};

pub struct World {
    mc: mcrs::Connection,
    cache: HashMap<Coordinate, CacheEntry>,
}

struct CacheEntry {
    block: Block,
    expiration: Instant,
}

impl World {
    pub fn new(mc: mcrs::Connection) -> Self {
        Self {
            mc,
            cache: HashMap::new(),
        }
    }

    /// Each side of cube are length `CHUNK_RADIUS * 2 + 1`.
    const CHUNK_RADIUS: u32 = 8;
    /// Maximum lifetime for a cache entry.
    const MAX_LIFETIME: Duration = Duration::from_secs(8);

    pub fn get_mc(&mut self) -> &mut mcrs::Connection {
        &mut self.mc
    }

    pub fn get_block(&mut self, location: Coordinate) -> Result<Block, mcrs::Error> {
        if let Some(block) = self.get_cache(location) {
            return Ok(block);
        }

        let size_half = Size::new(Self::CHUNK_RADIUS, Self::CHUNK_RADIUS, Self::CHUNK_RADIUS);

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
        let entry = self.cache.get(&location)?;
        if Instant::now() > entry.expiration {
            self.cache.remove(&location);
            return None;
        }
        Some(entry.block)
    }

    fn insert_cache(&mut self, origin: Coordinate, location: Coordinate, block: Block) {
        self.cache.insert(
            location,
            CacheEntry {
                block,
                expiration: Self::calculate_expiration(origin, location),
            },
        );
    }

    fn calculate_expiration(origin: Coordinate, location: Coordinate) -> Instant {
        let dist = manhattan_distance(origin, location);
        let max_dist = Self::CHUNK_RADIUS * 3;
        let lifetime = (Self::MAX_LIFETIME * (max_dist - dist)) / max_dist;
        Instant::now() + lifetime
    }
}

fn manhattan_distance(a: Coordinate, b: Coordinate) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y) + a.z.abs_diff(b.z)
}
