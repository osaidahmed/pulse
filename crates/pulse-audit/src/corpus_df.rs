use std::collections::HashMap;
use std::sync::OnceLock;

use xxhash_rust::xxh3::xxh3_64;

use pulse_syntax::parse::Language;

const MAGIC: &[u8; 4] = b"PCDF";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 12;

static BAKED: &[u8] = include_bytes!("corpus_df.bin");

pub struct CorpusDf {
    depth: usize,
    width: usize,
    counters: Vec<u16>,
    lang_totals: HashMap<u32, u64>,
}

pub fn empty() -> CorpusDf {
    CorpusDf { depth: 0, width: 0, counters: Vec::new(), lang_totals: HashMap::new() }
}

pub fn with_dims(depth: usize, width: usize) -> CorpusDf {
    assert!(width.is_power_of_two(), "width must be a power of two");
    CorpusDf { depth, width, counters: vec![0u16; depth * width], lang_totals: HashMap::new() }
}

pub fn is_empty(df: &CorpusDf) -> bool {
    df.depth == 0 || df.width == 0
}

pub fn lang_tag(lang: Language) -> u32 {
    xxh3_64(lang.to_config_key().as_bytes()) as u32
}

fn probe(lang_tag: u32, fingerprint: u64) -> (usize, usize) {
    let key = fingerprint.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ u64::from(lang_tag).rotate_left(1);
    let h = xxh3_64(&key.to_le_bytes());
    ((h & 0xFFFF_FFFF) as usize, ((h >> 32) | 1) as usize)
}

fn cells(df: &CorpusDf, h1: usize, h2: usize) -> impl Iterator<Item = usize> + '_ {
    let width = df.width;
    (0..df.depth).map(move |row| row * width + (h1.wrapping_add(row.wrapping_mul(h2)) & (width - 1)))
}

pub fn set_lang_total(df: &mut CorpusDf, lang_tag: u32, total: u64) {
    df.lang_totals.insert(lang_tag, total);
}

pub fn merge(into: &mut CorpusDf, other: &CorpusDf) {
    if into.depth != other.depth || into.width != other.width {
        return;
    }
    for (a, b) in into.counters.iter_mut().zip(&other.counters) {
        *a = a.saturating_add(*b);
    }
    for (&tag, &total) in &other.lang_totals {
        *into.lang_totals.entry(tag).or_insert(0) += total;
    }
}

pub fn add(df: &mut CorpusDf, lang_tag: u32, fingerprint: u64) {
    if is_empty(df) {
        return;
    }
    let (h1, h2) = probe(lang_tag, fingerprint);
    let touched: Vec<usize> = cells(df, h1, h2).collect();
    for i in touched {
        df.counters[i] = df.counters[i].saturating_add(1);
    }
}

pub fn estimate(df: &CorpusDf, lang_tag: u32, fingerprint: u64) -> u32 {
    if is_empty(df) {
        return 0;
    }
    let (h1, h2) = probe(lang_tag, fingerprint);
    cells(df, h1, h2).map(|i| u32::from(df.counters[i])).min().unwrap_or(0)
}

pub fn file_frequency(df: &CorpusDf, lang: Language, fingerprint: u64) -> f64 {
    let tag = lang_tag(lang);
    let total = df.lang_totals.get(&tag).copied().unwrap_or(0);
    if total == 0 {
        return 0.0;
    }
    f64::from(estimate(df, tag, fingerprint)) / total as f64
}

pub fn to_bytes(df: &CorpusDf) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + df.lang_totals.len() * 12 + df.counters.len() * 2);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(df.depth as u8);
    out.extend_from_slice(&(df.width as u32).to_le_bytes());
    out.extend_from_slice(&(df.lang_totals.len() as u16).to_le_bytes());
    let mut entries: Vec<(u32, u64)> = df.lang_totals.iter().map(|(&t, &n)| (t, n)).collect();
    entries.sort_unstable();
    for (tag, total) in entries {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
    }
    for &c in &df.counters {
        out.extend_from_slice(&c.to_le_bytes());
    }
    out
}

pub fn from_bytes(bytes: &[u8]) -> Option<CorpusDf> {
    if bytes.len() < HEADER_LEN || &bytes[0..4] != MAGIC || bytes[4] != VERSION {
        return None;
    }
    let depth = bytes[5] as usize;
    let width = u32::from_le_bytes(bytes[6..10].try_into().ok()?) as usize;
    let n_lang = u16::from_le_bytes(bytes[10..12].try_into().ok()?) as usize;
    let mut pos = HEADER_LEN;
    let mut lang_totals = HashMap::with_capacity(n_lang);
    for _ in 0..n_lang {
        let tag = u32::from_le_bytes(bytes.get(pos..pos + 4)?.try_into().ok()?);
        let total = u64::from_le_bytes(bytes.get(pos + 4..pos + 12)?.try_into().ok()?);
        lang_totals.insert(tag, total);
        pos += 12;
    }
    let cell_count = depth.checked_mul(width)?;
    let raw = bytes.get(pos..pos + cell_count.checked_mul(2)?)?;
    let counters = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    Some(CorpusDf { depth, width, counters, lang_totals })
}

pub fn corpus_df() -> &'static CorpusDf {
    static CACHE: OnceLock<CorpusDf> = OnceLock::new();
    CACHE.get_or_init(|| from_bytes(BAKED).unwrap_or_else(empty))
}
