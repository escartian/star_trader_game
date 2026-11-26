//! Save text as kept in a browser's storage: zlib-compressed (with its
//! checksum, so damage is caught) and base64-encoded behind a short tag.
//! Plain JSON is accepted on reading too. A played save of 342 KB of compact
//! JSON is stored in 31 KB.
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use std::io::{Read, Write};

/// Marks the compressed form.
const TAG: &str = "st1:";
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
/// Largest text a stored save may inflate to, in bytes (as the save loader).
const MAX_TEXT_BYTES: u64 = 64 * 1024 * 1024;

pub fn encode(text: &str) -> Result<String, String> {
    let mut packer = ZlibEncoder::new(Vec::new(), Compression::default());
    packer
        .write_all(text.as_bytes())
        .map_err(|error| error.to_string())?;
    let packed = packer.finish().map_err(|error| error.to_string())?;
    Ok(format!("{TAG}{}", base64(&packed)))
}

pub fn decode(stored: &str) -> Result<String, String> {
    let Some(body) = stored.strip_prefix(TAG) else {
        return Ok(stored.to_owned());
    };
    let packed = unbase64(body).ok_or("The stored save is damaged (encoding)")?;
    let mut text = String::new();
    ZlibDecoder::new(packed.as_slice())
        .take(MAX_TEXT_BYTES)
        .read_to_string(&mut text)
        .map_err(|_| "The stored save is damaged (compression)".to_owned())?;
    Ok(text)
}

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = chunk.iter().enumerate().fold(0u32, |sum, (index, byte)| {
            sum | u32::from(*byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(ALPHABET[(value >> (18 - 6 * index) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn unbase64(text: &str) -> Option<Vec<u8>> {
    let text = text.trim_end_matches('=');
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut value = 0u32;
    let mut bits = 0;
    for byte in text.bytes() {
        let digit = ALPHABET.iter().position(|known| *known == byte)? as u32;
        value = value << 6 | digit;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((value >> bits & 0xff) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard_alphabet() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        for sample in [&b""[..], b"f", b"fo", b"foo", b"\x00\xff\x10\x80"] {
            assert_eq!(unbase64(&base64(sample)).unwrap(), sample);
        }
    }

    #[test]
    fn a_save_round_trips_much_smaller_and_damage_is_caught() {
        let save = game_server::Session::default()
            .export_compact_json()
            .unwrap();
        let stored = encode(&save).unwrap();
        // A new game's save shrinks about 3.7 times; a played one (340 KB) 11.
        assert!(
            stored.len() * 3 < save.len(),
            "{} vs {}",
            stored.len(),
            save.len()
        );
        assert_eq!(decode(&stored).unwrap(), save);
        // Plain JSON (no tag) reads as it is.
        assert_eq!(decode(&save).unwrap(), save);
        let mut damaged = stored.clone();
        damaged.replace_range(40..44, "AAAA");
        assert!(decode(&damaged).is_err());
    }
}
