use std::time::Instant;
use std::{collections::HashMap, time::Duration};

use mcrs::{Block, Coordinate, Size};

pub struct World {
    mc: mcrs::Connection,
    cache: HashMap<Coordinate, (Instant, Block)>,
}

impl World {
    pub fn new(mc: mcrs::Connection) -> Self {
        Self {
            mc,
            cache: HashMap::new(),
        }
    }

    const CACHE_DURATION: Duration = Duration::from_secs(8);
    const CHUNK_SIZE: Size = Size::new(10, 10, 10);

    pub fn get_block(&mut self, location: Coordinate) -> Result<Block, mcrs::Error> {
        if let Some(block) = self.get_cache(location) {
            return Ok(block);
        }

        let origin = location - Self::CHUNK_SIZE / 2;
        let bound = origin + Self::CHUNK_SIZE;

        let chunk = self.mc.get_blocks(origin, bound)?;
        for entry in &chunk {
            self.insert_cache(entry.position_worldspace(), entry.block());
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

        self.insert_cache(location, block);
        self.mc.set_block(location, block)
    }

    fn get_cache(&self, location: Coordinate) -> Option<Block> {
        let (created, block) = *self.cache.get(&location)?;
        let now = Instant::now();
        if now.duration_since(created) >= Self::CACHE_DURATION {
            return None;
        }
        Some(block)
    }

    fn insert_cache(&mut self, location: Coordinate, block: Block) {
        let now = Instant::now();
        self.cache.insert(location, (now, block));
    }

    pub fn get_mc(&mut self) -> &mut mcrs::Connection {
        &mut self.mc
    }
}
