//! Steam's binary KeyValues format, as used by `shortcuts.vdf`. Each entry
//! is a type byte, a NUL-terminated key and a value; a map runs until an
//! `END` byte. Types we don't edit are kept byte for byte, so rewriting the
//! file never loses anything Steam (or another tool) put there.

const MAP: u8 = 0x00;
const STRING: u8 = 0x01;
const INT: u8 = 0x02;
const END: u8 = 0x08;

/// Fixed-size types we only pass through: float, pointer, color, uint64,
/// int64.
fn fixed_size(tag: u8) -> Option<usize> {
    match tag {
        0x03 | 0x04 | 0x06 => Some(4),
        0x07 | 0x0a => Some(8),
        _ => None,
    }
}

pub type Map = Vec<(String, Value)>;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Map(Map),
    String(String),
    /// Stored as the raw 32 bits; Steam's app ids use the high bit.
    Int(u32),
    Other { tag: u8, bytes: Vec<u8> },
}

/// Looks up a key the way Steam does: case-insensitively (older files use
/// `appname`/`exe`, newer ones `AppName`/`Exe`).
pub fn get<'a>(map: &'a Map, key: &str) -> Option<&'a Value> {
    map.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

/// Replaces a key's value, keeping its existing spelling and position, or
/// appends it.
pub fn set(map: &mut Map, key: &str, value: Value) {
    match map.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
        Some((_, v)) => *v = value,
        None => map.push((key.to_string(), value)),
    }
}

/// The map stored under `key`, created empty if there is none.
pub fn map_entry<'a>(map: &'a mut Map, key: &str) -> &'a mut Map {
    let index = map
        .iter()
        .position(|(k, v)| k.eq_ignore_ascii_case(key) && matches!(v, Value::Map(_)))
        .unwrap_or_else(|| {
            map.push((key.to_string(), Value::Map(Map::new())));
            map.len() - 1
        });
    match &mut map[index].1 {
        Value::Map(inner) => inner,
        _ => unreachable!("index points at a map"),
    }
}

pub fn parse(bytes: &[u8]) -> Result<Map, String> {
    let mut pos = 0;
    parse_map(bytes, &mut pos, true)
}

fn parse_map(bytes: &[u8], pos: &mut usize, root: bool) -> Result<Map, String> {
    let mut map = Map::new();
    loop {
        let Some(&tag) = bytes.get(*pos) else {
            // The root map may end at EOF instead of with its own END byte.
            return if root {
                Ok(map)
            } else {
                Err("Unexpected end of file".to_string())
            };
        };
        *pos += 1;
        if tag == END {
            return Ok(map);
        }
        let key = read_string(bytes, pos)?;
        let value = match tag {
            MAP => Value::Map(parse_map(bytes, pos, false)?),
            STRING => Value::String(read_string(bytes, pos)?),
            INT => Value::Int(u32::from_le_bytes(read_bytes(bytes, pos, 4)?.try_into().unwrap())),
            _ => match fixed_size(tag) {
                Some(len) => Value::Other {
                    tag,
                    bytes: read_bytes(bytes, pos, len)?.to_vec(),
                },
                None => return Err(format!("Unsupported value type 0x{tag:02x}")),
            },
        };
        map.push((key, value));
    }
}

fn read_string(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    let rest = &bytes[*pos..];
    let len = rest
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| "Unterminated string".to_string())?;
    *pos += len + 1;
    String::from_utf8(rest[..len].to_vec()).map_err(|_| "String is not valid UTF-8".to_string())
}

fn read_bytes<'a>(bytes: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], String> {
    let slice = bytes
        .get(*pos..*pos + len)
        .ok_or_else(|| "Unexpected end of file".to_string())?;
    *pos += len;
    Ok(slice)
}

pub fn write(map: &Map) -> Vec<u8> {
    let mut out = Vec::new();
    write_map(map, &mut out);
    out
}

fn write_map(map: &Map, out: &mut Vec<u8>) {
    for (key, value) in map {
        let tag = match value {
            Value::Map(_) => MAP,
            Value::String(_) => STRING,
            Value::Int(_) => INT,
            Value::Other { tag, .. } => *tag,
        };
        out.push(tag);
        write_string(key, out);
        match value {
            Value::Map(inner) => write_map(inner, out),
            Value::String(s) => write_string(s, out),
            Value::Int(n) => out.extend_from_slice(&n.to_le_bytes()),
            Value::Other { bytes, .. } => out.extend_from_slice(bytes),
        }
    }
    out.push(END);
}

fn write_string(s: &str, out: &mut Vec<u8>) {
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

#[cfg(test)]
mod tests;
