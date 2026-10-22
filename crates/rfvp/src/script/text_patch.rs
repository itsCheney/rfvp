//! Translation patches that replace the script's strings without rebuilding it.
//!
//! Some fan translations (Chinese patches for FAVORITE games, for one) leave the HCB alone and
//! ship their text beside it: `txt/NNNN.idx` lists the offsets in the HCB of the strings a
//! block replaces, as little-endian `u32`s, and `txt/NNNN.txt` holds the new strings, one line
//! for each offset, in UTF-16 (or UTF-8). An offset is that of the string's length byte, after
//! the `push_string` opcode. Offsets of other instructions (the patches list the speaker's
//! `call` too) are skipped.

use std::collections::BTreeMap;
use std::path::Path;

use super::parser::Parser;

const PUSH_STRING: u8 = 0x0e;
const CALL: u8 = 0x02;

/// Loads the patch in `game_dir`, if there is one, into `parser`.
pub fn apply(parser: &mut Parser, game_dir: &Path) {
    let patch = load(parser, game_dir);
    if !patch.is_empty() {
        log::info!("text patch: {} translated strings from txt/", patch.len());
        parser.set_text_patch(patch);
    }
}

fn load(parser: &Parser, game_dir: &Path) -> BTreeMap<u32, String> {
    let mut patch = BTreeMap::new();
    let Some(dir) = find_dir(game_dir) else {
        return patch;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return patch;
    };
    let mut indexes: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("idx"))
        })
        .collect();
    indexes.sort();

    let (mut skipped, mut misplaced) = (0usize, 0usize);
    for index in indexes {
        let Some(lines) = read_lines(&index) else {
            log::warn!("text patch: no text beside {}", index.display());
            continue;
        };
        let Ok(raw) = std::fs::read(&index) else {
            continue;
        };
        let offsets: Vec<u32> = raw
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        if offsets.len() != lines.len() {
            log::warn!(
                "text patch: {} lists {} strings but its text has {} lines; skipped",
                index.display(),
                offsets.len(),
                lines.len()
            );
            continue;
        }
        for (offset, line) in offsets.into_iter().zip(lines) {
            let opcode = offset
                .checked_sub(1)
                .and_then(|at| parser.read_u8(at as usize).ok());
            match opcode {
                Some(PUSH_STRING) => {
                    patch.insert(offset + 1, line);
                }
                Some(CALL) => skipped += 1,
                _ => misplaced += 1,
            }
        }
    }
    if misplaced > patch.len() / 10 {
        // The offsets don't fit this script: the patch is for another version of the game.
        log::warn!(
            "text patch: {misplaced} offsets in txt/ aren't strings of this script; patch not used"
        );
        return BTreeMap::new();
    }
    if skipped > 0 || misplaced > 0 {
        log::info!("text patch: {skipped} speaker entries and {misplaced} stray offsets ignored");
    }
    patch
}

/// The game's `txt` directory, whatever its case.
fn find_dir(game_dir: &Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(game_dir).ok()?.flatten().find_map(|entry| {
        let is_dir = entry.file_type().ok()?.is_dir();
        (is_dir && entry.file_name().to_str()?.eq_ignore_ascii_case("txt")).then(|| entry.path())
    })
}

/// The lines of the text beside `index` (same name, `.txt`), decoded from UTF-16 (with a byte
/// order mark) or UTF-8.
fn read_lines(index: &Path) -> Option<Vec<String>> {
    let parent = index.parent()?;
    let stem = index.file_stem()?.to_str()?;
    let path = std::fs::read_dir(parent).ok()?.flatten().find_map(|entry| {
        let name = entry.file_name();
        let name = name.to_str()?;
        let (base, ext) = name.rsplit_once('.')?;
        (base.eq_ignore_ascii_case(stem) && ext.eq_ignore_ascii_case("txt")).then(|| entry.path())
    })?;
    let bytes = std::fs::read(path).ok()?;
    let text = decode(&bytes)?;
    let mut lines: Vec<String> = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
    // A final line break leaves an empty last line.
    if lines.len() > 1 && lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    Some(lines)
}

fn decode(bytes: &[u8]) -> Option<String> {
    let utf16 = |bytes: &[u8], le: bool| {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| if le { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) })
            .collect();
        String::from_utf16(&units).ok()
    };
    match bytes {
        [0xff, 0xfe, rest @ ..] => utf16(rest, true),
        [0xfe, 0xff, rest @ ..] => utf16(rest, false),
        [0xef, 0xbb, 0xbf, rest @ ..] => String::from_utf8(rest.to_vec()).ok(),
        _ => String::from_utf8(bytes.to_vec()).ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16_and_utf8() {
        assert_eq!(decode(&[0xff, 0xfe, b'a', 0, 0x2d, 0x4e]).as_deref(), Some("a中"));
        assert_eq!(decode("abc".as_bytes()).as_deref(), Some("abc"));
        assert_eq!(decode(&[0xef, 0xbb, 0xbf, b'x']).as_deref(), Some("x"));
    }
}
