extern crate base32;
extern crate regex;

use std::{borrow::Borrow, collections::VecDeque};
use std::{collections::HashMap};
use std::convert::TryInto;
use regex::Regex;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

const RFC4648_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn checksum(header: &str) -> bool {
    let mut check: usize = 0;
    for h in header.chars() {
        match RFC4648_ALPHABET.iter().position(|c| h == *c as char) {
            Some(i) => { check ^= i; }
            None => { return false; }
        }
    }
    println!("check {:?}", check);
    if check == 0 { return true }
    false
}

#[derive(Debug)]
pub struct MessageChunk {
    pub source: String,
    pub idx: u8,
    pub version: char,
    pub last: bool,
}

impl MessageChunk {

    pub fn from(raw_question: &str, domain: &str) -> Result<Self> {
        let domain_rg = Regex::new(&format!("{}$", domain))?;
        let period_rg = Regex::new(r"\.+")?;
        let raw_message = domain_rg.replace(raw_question, "");
        let source = period_rg.replace_all(raw_message.borrow(), "").to_ascii_uppercase();

        if source.len() < 16 {
            return Err("Invalid Message: too short".into())
        }

        let version = source[0..1].chars().next().unwrap();
        if !(version == 'A' || version == 'B') {
            return Err("Not a valid message".into())
        }

        let idx_byte = source[1..2].chars().next().unwrap() as usize;
        let idx: u8 = RFC4648_ALPHABET.iter().position(|c| idx_byte == *c as usize)
            .ok_or("Unable to decode index")?
            .try_into().unwrap();

        if !checksum(&source[0..16]) {
            return Err("Not a valid message".into())
        }

        let mut last = false;
        if version == 'B' {
            last = true;
        }

        Ok(MessageChunk {
            source,
            idx,
            last,
            version,
        })
    }

    pub fn id(&self) -> String {
        self.source[2..15].to_string()
    }

    pub fn content(&self) -> String {
        self.source[16..].to_string()
    }

}

#[derive(Debug)]
pub struct MessageBuffer {
    message_parts: HashMap<u8, MessageChunk>,
    message_parts_total_len: u8,
}

impl MessageBuffer {

    pub fn new() -> Self {
        MessageBuffer {
            message_parts: HashMap::new(),
            message_parts_total_len: 0,
        }
    }

    pub fn insert(&mut self, message_chunk: MessageChunk) -> bool {
        let idx = message_chunk.idx;
        if message_chunk.last {
            self.message_parts_total_len = idx + 1;
        }
        self.message_parts.insert(idx, message_chunk);
        self.is_complete()
    }

    pub fn is_complete(&self) -> bool {
        self.message_parts_total_len as usize == self.message_parts.len()
    }

    pub fn get_message(&self) -> String {
        let mut keys = self.message_parts.keys().copied().collect::<Vec<u8>>();
        keys.sort_unstable();
        let mut content = String::new();
        for key in keys {
            let val = self.message_parts.get(&key).unwrap();
            content.push_str(&val.content());
        }
        content
    }

}

#[derive(Debug)]
pub struct MessageBufferCache {
    message_buffers: HashMap<String, MessageBuffer>,
    cache_size: usize,
    buffer_list: VecDeque<String>,
}

impl MessageBufferCache {
    pub fn new(cache_size: usize) -> Self {
        let message_buffers = HashMap::new();
        MessageBufferCache {
            message_buffers,
            cache_size,
            buffer_list: VecDeque::new()
        }
    }

    pub fn add(&mut self, message_chunk: MessageChunk) -> Result<bool> {
        let message_id = message_chunk.id();
        let message_buffer = self.message_buffers.entry(message_id.clone()).
            or_insert_with(MessageBuffer::new);
        let is_complete = message_buffer.insert(message_chunk);
        if self.buffer_list.iter().find(|&id| id == &message_id) == None {
            self.buffer_list.push_front(message_id);
        };

        // TODO: More efficient LRU like pattern. Works for now
        if self.buffer_list.len() > self.cache_size {
            let evicted_key  = self.buffer_list.pop_back().unwrap();
            self.message_buffers.remove(&evicted_key);
        }
        Ok(is_complete)
    }
    
    pub fn get_value(&self, key: &str) -> Option<Vec<u8>> {
        match self.message_buffers.get(key) {
            Some(val) => base32::decode(base32::Alphabet::RFC4648 { padding: false }, val.get_message().as_str()),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_hash() {
        let mut message_buffer_cache = MessageBufferCache::new(3);
        
        message_buffer_cache.add(MessageChunk::from("AAAAAAAAAAAAAAAAPMRGM33PEI5.foo.co", "foo.co").unwrap()).unwrap();
        message_buffer_cache.add(MessageChunk::from("BBAAAAAAAAAAAAAACEYTBOIRH2.foo.co", "foo.co").unwrap()).unwrap();
        message_buffer_cache.add(MessageChunk::from("AA22222222222222FOO.foo.co", "foo.co").unwrap()).unwrap();
        message_buffer_cache.add(MessageChunk::from("AA33333333333333FOO.foo.co", "foo.co").unwrap()).unwrap();
        message_buffer_cache.add(MessageChunk::from("AA44444444444444Foo.foo.co", "foo.co").unwrap()).unwrap();
        assert!(message_buffer_cache.message_buffers.len() == 3);
        assert!(!message_buffer_cache.message_buffers.contains_key("AAAAAAAAAAAAA"));
        assert!(message_buffer_cache.message_buffers.contains_key("2222222222222"));
    }

    #[test]
    fn test_message_building() {
        let mut message_buffer_cache = MessageBufferCache::new(3);
        message_buffer_cache.add(MessageChunk::from("AADDDDDDDDDDDDDDPMRGM33PEI5.foo.co", "foo.co").unwrap()).unwrap();
        let is_complete = message_buffer_cache.add(MessageChunk::from("BBDDDDDDDDDDDDDDCEYTBOIRH2.foo.co", "foo.co").unwrap()).unwrap();
        assert!(is_complete);
        let byte_message = message_buffer_cache.get_value("DDDDDDDDDDDDD").unwrap();
        let message = String::from_utf8(byte_message).unwrap();
        assert_eq!(message, String::from("{\"foo\":\"bar\"}"));
    }

    #[test]
    fn test_message_dupes() {
        // Should happily handle multiple duplicate messages an in any order (UDP constrain)
        let mut message_buffer_cache = MessageBufferCache::new(3);
        let mut is_complete =message_buffer_cache.add(MessageChunk::from("AAZ222222222222ZPMRGM33PEI5.foo.co", "foo.co").unwrap()).unwrap();
        assert!(!is_complete);
        is_complete = message_buffer_cache.add(MessageChunk::from("BBZ222222222222ZCEYTBOIRH2.foo.co", "foo.co").unwrap()).unwrap();
        assert!(is_complete);
        is_complete = message_buffer_cache.add(MessageChunk::from("BBZ222222222222ZCEYTBOIRH2.foo.co", "foo.co").unwrap()).unwrap();
        assert!(is_complete);
        let byte_message = message_buffer_cache.get_value("Z222222222222").unwrap();
        let message = String::from_utf8(byte_message).unwrap();
        assert_eq!(message, String::from("{\"foo\":\"bar\"}"));
    }

}