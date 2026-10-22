//! Fonts the host registers for the engine's built-in faces.
//!
//! The original engine draws with the Windows faces MS Gothic, MS Mincho,
//! MS PGothic and MS PMincho. Those fonts may not be redistributed, so the
//! host (Mikage on iOS) registers stand-ins before it creates the engine,
//! plus a chain of fallback faces for characters the stand-ins lack.
//! Without a registration, desktop builds take a CJK font from the system.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};

use crate::font::Font;

/// The built-in faces, in the order of their font ids -2 to -5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemFace {
    Gothic = 0,
    Mincho = 1,
    PGothic = 2,
    PMincho = 3,
}

impl SystemFace {
    pub fn from_index(index: i32) -> Option<Self> {
        match index {
            0 => Some(Self::Gothic),
            1 => Some(Self::Mincho),
            2 => Some(Self::PGothic),
            3 => Some(Self::PMincho),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct HostFont {
    pub name: String,
    pub file_name: String,
    pub font: Font,
}

#[derive(Default)]
struct Registry {
    faces: [Option<Font>; 4],
    fallbacks: Vec<HostFont>,
}

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

fn with_registry<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    let mut guard = REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(guard.get_or_insert_with(Registry::default))
}

/// Loads the face named `face` (PostScript, full or family name; empty for
/// the first face) from the font file or collection at `path`.
fn load_face(path: &Path, face: &str) -> Result<HostFont> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let index = if face.is_empty() {
        0
    } else {
        face_index(&bytes, face).unwrap_or_else(|| {
            log::warn!("font {}: no face named '{face}', using the first", path.display());
            0
        })
    };
    let font = Font::from_vec_index(bytes, index)
        .map_err(|e| anyhow!("{}: {e}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or_default()
        .to_string();
    Ok(HostFont {
        name: if face.is_empty() {
            file_name.clone()
        } else {
            face.to_string()
        },
        file_name,
        font,
    })
}

/// Registers the stand-in for one built-in face. Returns false when the
/// file can't be read or parsed.
pub fn register_system_face(face: SystemFace, path: &Path, face_name: &str) -> bool {
    match load_face(path, face_name) {
        Ok(loaded) => {
            log::info!(
                "host font {:?}: '{}' from {}",
                face,
                loaded.name,
                path.display()
            );
            with_registry(|r| r.faces[face as usize] = Some(loaded.font));
            true
        }
        Err(e) => {
            log::warn!("host font {:?} not loaded: {e:#}", face);
            false
        }
    }
}

/// Adds a face to the chain used for characters the requested face lacks,
/// tried in registration order.
pub fn register_fallback(path: &Path, face_name: &str) -> bool {
    match load_face(path, face_name) {
        Ok(loaded) => {
            log::info!("host fallback font '{}' from {}", loaded.name, path.display());
            with_registry(|r| r.fallbacks.push(loaded));
            true
        }
        Err(e) => {
            log::warn!("host fallback font not loaded: {e:#}");
            false
        }
    }
}

pub fn clear() {
    with_registry(|r| *r = Registry::default());
}

/// Whether a stand-in for MS Gothic is available, which the engine needs.
pub fn has_default_face() -> bool {
    with_registry(|r| r.faces[0].is_some()) || desktop_default().is_some()
}

/// The four built-in faces, by font id -2 to -5. A face without its own
/// stand-in uses its closest registered one (PMincho -> Mincho, any -> Gothic).
pub(crate) fn system_faces() -> [Font; 4] {
    let faces = with_registry(|r| r.faces.clone());
    let gothic = faces[0]
        .clone()
        .or_else(desktop_default)
        .expect("rfvp: no font for MS Gothic; the host must register one before creating the engine");
    let mincho = faces[1].clone().unwrap_or_else(|| gothic.clone());
    let pgothic = faces[2].clone().unwrap_or_else(|| gothic.clone());
    let pmincho = faces[3].clone().unwrap_or_else(|| mincho.clone());
    [gothic, mincho, pgothic, pmincho]
}

pub(crate) fn fallbacks() -> Vec<HostFont> {
    with_registry(|r| r.fallbacks.clone())
}

/// Without a host registration: a Japanese font from the system, for the
/// desktop builds and tests.
fn desktop_default() -> Option<Font> {
    static FOUND: Mutex<Option<Option<Font>>> = Mutex::new(None);
    let mut found = FOUND.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    found
        .get_or_insert_with(|| {
            let candidates = desktop_candidates();
            candidates.iter().find_map(|path| {
                let bytes = std::fs::read(path).ok()?;
                let font = Font::from_vec_index(bytes, 0).ok()?;
                log::info!("default font from the system: {}", path.display());
                Some(font)
            })
        })
        .clone()
}

fn desktop_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(path) = std::env::var_os("RFVP_DEFAULT_FONT") {
        out.push(PathBuf::from(path));
    }
    #[cfg(test)]
    out.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rfvp-rebuilder/assets/fonts/BIZUDGothic-Regular.ttf"),
    );
    #[cfg(target_os = "windows")]
    {
        let windir = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        out.push(windir.join("Fonts").join("msgothic.ttc"));
        out.push(windir.join("Fonts").join("YuGothM.ttc"));
    }
    #[cfg(target_os = "macos")]
    {
        out.push(PathBuf::from("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"));
        out.push(PathBuf::from("/System/Library/Fonts/Hiragino Sans GB.ttc"));
    }
    #[cfg(target_os = "linux")]
    {
        for dir in ["/usr/share/fonts", "/usr/local/share/fonts"] {
            for name in [
                "opentype/noto/NotoSansCJK-Regular.ttc",
                "noto-cjk/NotoSansCJK-Regular.ttc",
                "truetype/fonts-japanese-gothic.ttf",
                "opentype/ipafont-gothic/ipag.ttf",
                "truetype/takao-gothic/TakaoGothic.ttf",
                "truetype/dejavu/DejaVuSans.ttf",
            ] {
                out.push(Path::new(dir).join(name));
            }
        }
    }
    out
}

fn be_u16(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(off..off.checked_add(2)?)?.try_into().ok()?))
}

fn be_u32(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(off..off.checked_add(4)?)?.try_into().ok()?))
}

/// The index of the face with this name in a font file or collection.
fn face_index(bytes: &[u8], wanted: &str) -> Option<u32> {
    let offsets: Vec<usize> = if bytes.get(0..4)? == b"ttcf" {
        let count = be_u32(bytes, 8)? as usize;
        (0..count.min(256))
            .filter_map(|i| be_u32(bytes, 12 + 4 * i).map(|o| o as usize))
            .collect()
    } else {
        vec![0]
    };
    offsets
        .iter()
        .position(|&off| face_names(bytes, off).iter().any(|n| n.eq_ignore_ascii_case(wanted)))
        .map(|i| i as u32)
}

/// Family (1, 16), full (4) and PostScript (6) names of the face whose
/// table directory is at `face_off`.
fn face_names(bytes: &[u8], face_off: usize) -> Vec<String> {
    let mut names = Vec::new();
    let Some(num_tables) = be_u16(bytes, face_off + 4) else {
        return names;
    };
    let mut table = None;
    for i in 0..num_tables as usize {
        let rec = face_off + 12 + 16 * i;
        if bytes.get(rec..rec + 4) == Some(b"name".as_slice()) {
            table = be_u32(bytes, rec + 8).map(|o| o as usize);
            break;
        }
    }
    let Some(name_off) = table else {
        return names;
    };
    let (Some(count), Some(strings)) = (be_u16(bytes, name_off + 2), be_u16(bytes, name_off + 4))
    else {
        return names;
    };
    for i in 0..count as usize {
        let rec = name_off + 6 + 12 * i;
        let (Some(platform), Some(name_id), Some(len), Some(off)) = (
            be_u16(bytes, rec),
            be_u16(bytes, rec + 6),
            be_u16(bytes, rec + 8),
            be_u16(bytes, rec + 10),
        ) else {
            break;
        };
        if !matches!(name_id, 1 | 4 | 6 | 16) {
            continue;
        }
        let start = name_off + strings as usize + off as usize;
        let Some(raw) = bytes.get(start..start + len as usize) else {
            continue;
        };
        let text = match platform {
            0 | 3 => {
                let units: Vec<u16> = raw
                    .chunks_exact(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&units)
            }
            1 => raw.iter().map(|&b| b as char).collect(),
            _ => continue,
        };
        names.push(text.trim_matches('\0').trim().to_string());
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_face_by_name() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rfvp-rebuilder/assets/fonts/BIZUDGothic-Regular.ttf");
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(face_index(&bytes, "BIZUDGothic-Regular"), Some(0));
        assert_eq!(face_index(&bytes, "BIZ UDGothic"), Some(0));
        assert_eq!(face_index(&bytes, "No Such Face"), None);
    }
}
