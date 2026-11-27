//! (Plan GPU, stage 1) The index of an RDS5 texture pool (`.ttpf`): its chunks and its ETPF texture
//! records, read from the head of the file alone. Layout as measured on all 57 pools of the retail
//! title (community format notes):
//!
//! - `RDS5` at 0, entry count `n` at 4 (u32), virtual size in the low 56 bits at 8 (top byte 22);
//! - the chunk table at 0x18, `n - 1` u64 (the last is a sentinel), `n - 2` chunks;
//!   a chunk is: physical offset bits 0-25 × 256, codec bits 26-29 (0 raw, 1-7 BCPack BC1-BC7,
//!   8 zlib), virtual offset bits 30-51 × 4096, padding at the end of its physical range bits 52-63;
//! - the ETPF index at the table's end rounded up to 256: `ETPF`, version 634, the texture count,
//!   the record area; 16-byte records, whose third word & 0x1fffff is the 4 KiB page of the
//!   texture's first chunk (strictly increasing, the first is chunk 0).
//!
//! A DirectStorage request reads one chunk's physical range: [`Pool::at`] says which texture and
//! which of its chunks (mips) that is.

/// One chunk of a pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunk {
    /// Where its bytes are in the file, and how many (padding left out).
    pub offset: u64,
    pub packed: u32,
    /// Where it lands in the pool's virtual image, and its decoded size.
    pub virt: u64,
    pub size: u32,
    /// 0 raw, 1-7 BCPack BC1-BC7, 8 zlib.
    pub codec: u8,
}

/// One texture record: its first chunk and the record's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Texture {
    pub first_chunk: u32,
    pub chunks: u32,
    pub width: u32,
    pub height: u32,
    /// The record's second word (format and category).
    pub kind: u32,
}

#[derive(Debug)]
pub struct Pool {
    pub chunks: Vec<Chunk>,
    pub textures: Vec<Texture>,
}

/// Bytes of a pool's head that hold the whole index for any pool of the title (the largest
/// index measured is under 9 MiB).
pub const HEAD: usize = 16 << 20;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

impl Pool {
    /// Reads the index from the first bytes of a pool (at least the index; [`HEAD`] is enough).
    pub fn parse(d: &[u8]) -> Result<Pool, &'static str> {
        if d.get(..4) != Some(b"RDS5") {
            return Err("not an RDS5 pool");
        }
        let n = u32_at(d, 4).ok_or("truncated")? as usize;
        if !(3..=1_000_002).contains(&n) || u64_at(d, 8).ok_or("truncated")? >> 56 != 22 {
            return Err("unsupported RDS5 layout");
        }
        let entry = |i: usize| u64_at(d, 0x18 + i * 8).ok_or("chunk table past the bytes read");
        let mut chunks = Vec::with_capacity(n - 2);
        let mut next = entry(0)?;
        for i in 0..n - 2 {
            let v = next;
            next = entry(i + 1)?;
            let (start, end) = ((v & 0x3ff_ffff) * 256, (next & 0x3ff_ffff) * 256);
            let (virt, vend) = (((v >> 30) & 0x3f_ffff) * 4096, ((next >> 30) & 0x3f_ffff) * 4096);
            let packed = end.checked_sub(start + (v >> 52)).ok_or("chunk range")?;
            let size = vend.checked_sub(virt).ok_or("chunk range")?;
            chunks.push(Chunk { offset: start, packed: packed as u32, virt, size: size as u32, codec: ((v >> 26) & 15) as u8 });
        }
        let etpf = (0x18 + (n - 1) * 8).next_multiple_of(256);
        if d.get(etpf..etpf + 4) != Some(b"ETPF") || u32_at(d, etpf + 4) != Some(634) {
            return Err("unsupported ETPF index");
        }
        let count = u32_at(d, etpf + 8).ok_or("truncated")? as usize;
        let mut textures: Vec<Texture> = Vec::with_capacity(count);
        let mut c = 0usize;
        for t in 0..count {
            let r = etpf + 16 + t * 16;
            let (w0, w1, w2) = (u32_at(d, r).ok_or("records past the bytes read")?, u32_at(d, r + 4).unwrap_or(0), u32_at(d, r + 8).unwrap_or(0));
            let virt = u64::from(w2 & 0x1f_ffff) * 4096;
            while c < chunks.len() && chunks[c].virt < virt {
                c += 1;
            }
            if chunks.get(c).map(|k| k.virt) != Some(virt) {
                return Err("a record's first page is no chunk's start");
            }
            if let Some(last) = textures.last_mut() {
                last.chunks = c as u32 - last.first_chunk;
            }
            textures.push(Texture {
                first_chunk: c as u32,
                chunks: 0,
                width: (w0 & 0x3fff) + 1,
                height: ((w0 >> 14) & 0x3fff) + 1,
                kind: w1,
            });
        }
        if let Some(last) = textures.last_mut() {
            last.chunks = chunks.len() as u32 - last.first_chunk;
        }
        Ok(Pool { chunks, textures })
    }

    /// The texture and chunk whose bytes start at file offset `offset`.
    pub fn at(&self, offset: u64) -> Option<(usize, usize)> {
        let c = self.chunks.binary_search_by_key(&offset, |k| k.offset).ok()?;
        let t = self.textures.partition_point(|t| t.first_chunk as usize <= c).checked_sub(1)?;
        Some((t, c - self.textures[t].first_chunk as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic pool: two textures, of 1 and 2 chunks (no game data).
    fn pool() -> Vec<u8> {
        let n = 3 + 2; // 3 chunks + sentinel + 1
        let etpf = (0x18 + (n - 1) * 8usize).next_multiple_of(256);
        let data = etpf + 4096;
        let mut d = vec![0u8; data + 3 * 256];
        d[..4].copy_from_slice(b"RDS5");
        d[4..8].copy_from_slice(&(n as u32).to_le_bytes());
        d[8..16].copy_from_slice(&((22u64 << 56) | 4 * 4096).to_le_bytes());
        let e = |phys: usize, virt: u64, codec: u64| ((phys as u64 / 256) | codec << 26 | (virt / 4096) << 30).to_le_bytes();
        for (i, (p, v, c)) in [(data, 4096, 8), (data + 256, 2 * 4096, 8), (data + 512, 3 * 4096, 1), (data + 768, 4 * 4096, 0)].into_iter().enumerate() {
            d[0x18 + i * 8..0x20 + i * 8].copy_from_slice(&e(p, v, c));
        }
        d[etpf..etpf + 4].copy_from_slice(b"ETPF");
        d[etpf + 4..etpf + 8].copy_from_slice(&634u32.to_le_bytes());
        d[etpf + 8..etpf + 12].copy_from_slice(&2u32.to_le_bytes());
        d[etpf + 12..etpf + 16].copy_from_slice(&32u32.to_le_bytes());
        let rec = |at: usize, w0: u32, page: u32, d: &mut Vec<u8>| {
            d[at..at + 4].copy_from_slice(&w0.to_le_bytes());
            d[at + 8..at + 12].copy_from_slice(&page.to_le_bytes());
        };
        rec(etpf + 16, 31 | 31 << 14, 1, &mut d); // 32 × 32, chunk 0
        rec(etpf + 32, 1023 | 511 << 14, 2, &mut d); // 1024 × 512, chunks 1-2
        d
    }

    #[test]
    fn reads_the_index_and_finds_requests() {
        let d = pool();
        let p = Pool::parse(&d).unwrap();
        assert_eq!(p.chunks.len(), 3);
        assert_eq!(p.chunks[2].codec, 1);
        assert_eq!(p.textures.len(), 2);
        assert_eq!((p.textures[0].first_chunk, p.textures[0].chunks), (0, 1));
        assert_eq!((p.textures[1].first_chunk, p.textures[1].chunks), (1, 2));
        assert_eq!((p.textures[1].width, p.textures[1].height), (1024, 512));
        let data = p.chunks[0].offset;
        assert_eq!(p.at(data), Some((0, 0)));
        assert_eq!(p.at(data + 512), Some((1, 1)));
        assert_eq!(p.at(data + 1), None);
        assert!(Pool::parse(b"RDS4").is_err());
    }
}

/// (Run by hand, the game's pools needed) `KLBF_TTPF_DIR=C:\Games\Title cargo test -p victory-formats -- --ignored`:
/// every pool under the folder reads, and every chunk is found again by its offset.
#[cfg(test)]
#[test]
#[ignore]
fn reads_every_pool_of_the_title() {
    use std::io::Read;
    let dir = std::env::var("KLBF_TTPF_DIR").expect("KLBF_TTPF_DIR");
    let mut todo = vec![std::path::PathBuf::from(dir)];
    let (mut pools, mut textures, mut chunks) = (0, 0, 0);
    while let Some(p) = todo.pop() {
        for e in std::fs::read_dir(&p).unwrap().flatten() {
            let path = e.path();
            if path.is_dir() {
                todo.push(path);
            } else if path.extension().is_some_and(|x| x == "ttpf") {
                let mut head = Vec::new();
                std::fs::File::open(&path).unwrap().take(HEAD as u64).read_to_end(&mut head).unwrap();
                let pool = Pool::parse(&head).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                for (i, c) in pool.chunks.iter().enumerate() {
                    let (t, k) = pool.at(c.offset).unwrap();
                    assert_eq!(pool.textures[t].first_chunk as usize + k, i);
                }
                pools += 1;
                textures += pool.textures.len();
                chunks += pool.chunks.len();
            }
        }
    }
    println!("{pools} pools, {textures} textures, {chunks} chunks");
    assert_eq!((pools, textures, chunks), (57, 357_115, 1_026_833));
}
