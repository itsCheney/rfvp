//! Mikage host additions to the C API.
//!
//! Mikage (an iOS multi-engine player) drives the engine through the
//! `rfvp_ios_*` functions and these. They take the same handle.

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::Path;
use std::sync::Mutex;

use crate::ios_host::{guard, guard_void, IosInstance};
use crate::subsystem::resources::host_fonts::{self, SystemFace};

static LAST_ERROR: Mutex<Option<CString>> = Mutex::new(None);

pub(crate) fn set_last_error(message: &str) {
    let message = CString::new(message.replace('\0', " ")).unwrap_or_default();
    *LAST_ERROR
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(message);
}

unsafe fn cstr_opt(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(str::to_string)
}

unsafe fn instance<'a>(handle: *mut c_void) -> Option<&'a mut IosInstance> {
    (handle as *mut IosInstance).as_mut()
}

/// The last error from `rfvp_ios_create` or `rfvp_mikage_step`, or an empty
/// string. Valid until the next failing call.
#[no_mangle]
pub extern "C" fn rfvp_mikage_last_error() -> *const c_char {
    static EMPTY: &[u8] = b"\0";
    let last = LAST_ERROR
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match last.as_ref() {
        // The CString lives in the static until it is replaced.
        Some(message) => message.as_ptr(),
        None => EMPTY.as_ptr() as *const c_char,
    }
}

/// Registers the stand-in for one of the Windows faces games draw with:
/// `face` 0 MS Gothic (also the default), 1 MS Mincho, 2 MS PGothic, 3 MS
/// PMincho. `face_name_utf8` picks the face in a collection by PostScript,
/// full or family name; null or empty takes the first. Call before
/// `rfvp_ios_create`. Returns 1 when the font loaded.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_register_font_face(
    path_utf8: *const c_char,
    face: i32,
    face_name_utf8: *const c_char,
) -> i32 {
    let (Some(path), Some(face)) = (cstr_opt(path_utf8), SystemFace::from_index(face)) else {
        return 0;
    };
    let name = cstr_opt(face_name_utf8).unwrap_or_default();
    i32::from(host_fonts::register_system_face(face, Path::new(&path), &name))
}

/// Adds a face to the chain used for characters the game's font lacks,
/// tried in the order they were added. Returns 1 when the font loaded.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_register_fallback_font(
    path_utf8: *const c_char,
    face_name_utf8: *const c_char,
) -> i32 {
    let Some(path) = cstr_opt(path_utf8) else {
        return 0;
    };
    let name = cstr_opt(face_name_utf8).unwrap_or_default();
    i32::from(host_fonts::register_fallback(Path::new(&path), &name))
}

/// Forgets every registered font.
#[no_mangle]
pub extern "C" fn rfvp_mikage_clear_fonts() {
    host_fonts::clear();
}

/// Runs the script file `name_utf8` (in the game directory) instead of the
/// first `*.bch`, else `*.hcb`, found. Null or empty restores the default.
/// Call before `rfvp_ios_create`.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_set_script(name_utf8: *const c_char) {
    crate::utils::file::set_preferred_script(cstr_opt(name_utf8));
}

/// Keeps saves in the game directory's folder `name_utf8` (Mikage:
/// `savedata`) instead of `save`. Null or empty restores `save`. Call
/// before `rfvp_ios_create`.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_set_save_dir(name_utf8: *const c_char) {
    crate::utils::file::set_save_dir_name(cstr_opt(name_utf8));
}

/// Like `rfvp_ios_step`: 0 running, 1 the game ended, 2 error (see
/// `rfvp_mikage_last_error`). After an error, only destroy the handle.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_step(handle: *mut c_void, dt_ms: u32) -> i32 {
    let Some(inst) = instance(handle) else {
        set_last_error("null handle");
        return 2;
    };
    match guard("step", || Ok(inst.app.host_step(dt_ms))) {
        Ok(false) => 0,
        Ok(true) => 1,
        Err(e) => {
            log::error!("{e:#}");
            set_last_error(&format!("{e:#}"));
            2
        }
    }
}

/// The game's own resolution.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_logical_size(
    handle: *mut c_void,
    width: *mut u32,
    height: *mut u32,
) {
    let (w, h) = instance(handle)
        .map(|inst| inst.app.host_virtual_size())
        .unwrap_or((0, 0));
    if let Some(width) = width.as_mut() {
        *width = w;
    }
    if let Some(height) = height.as_mut() {
        *height = h;
    }
}

/// 0 when the app goes to the background (sounds pause, the global save is
/// written), 1 when it comes back.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_set_foreground(handle: *mut c_void, foreground: i32) {
    if let Some(inst) = instance(handle) {
        guard_void("set_foreground", || inst.app.host_set_foreground(foreground != 0));
    }
}

/// A key press (`down` 1) or release (0) by Windows virtual-key code:
/// 0x1B Escape, 0x0D Enter, 0x20 Space, 0x25..0x28 arrows, 0x11 Control
/// (held, it skips text in FVP games).
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_key(handle: *mut c_void, vk_code: i32, down: i32) {
    if let Some(inst) = instance(handle) {
        guard_void("key", || inst.app.host_key_vk_ios(vk_code, down != 0));
    }
}

/// Mikage's trackpad cursor as a mouse, at a point in UIKit points in the
/// view (as `rfvp_ios_touch`). phase: 0 move, 1 left down, 2 left up,
/// 3 right down, 4 right up.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_pointer(handle: *mut c_void, phase: i32, x: f64, y: f64) {
    let Some(inst) = instance(handle) else {
        return;
    };
    let (button, button_phase) = match phase {
        0 => (0, 1),
        1 => (0, 0),
        2 => (0, 2),
        3 => (1, 0),
        4 => (1, 2),
        _ => return,
    };
    guard_void("pointer", || {
        inst.app.host_mouse_button_ios(button, button_phase, x, y)
    });
}

/// A right click at a point in UIKit points in the view. FVP games open
/// their menu, close windows and step back with it.
#[no_mangle]
pub unsafe extern "C" fn rfvp_mikage_secondary_click(handle: *mut c_void, x: f64, y: f64) {
    let Some(inst) = instance(handle) else {
        return;
    };
    guard_void("secondary_click", || {
        inst.app.host_mouse_button_ios(1, 0, x, y);
        inst.app.host_mouse_button_ios(1, 2, x, y);
    });
}

/// Receives engine log records: level (1 error, 2 warn, 3 info, 4 debug,
/// 5 trace), the module target and the message, both UTF-8 and valid only
/// during the call. It may be called from any thread.
pub type RfvpMikageLogCallback =
    extern "C" fn(level: i32, target: *const c_char, message: *const c_char, context: *mut c_void);

struct HostLogger {
    sink: Mutex<Option<(RfvpMikageLogCallback, usize)>>,
}

static HOST_LOGGER: HostLogger = HostLogger {
    sink: Mutex::new(None),
};

impl log::Log for HostLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let sink = *self
            .sink
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some((callback, context)) = sink else {
            return;
        };
        let target = CString::new(record.target().replace('\0', " ")).unwrap_or_default();
        let message = CString::new(record.args().to_string().replace('\0', " ")).unwrap_or_default();
        callback(
            record.level() as i32,
            target.as_ptr(),
            message.as_ptr(),
            context as *mut c_void,
        );
    }

    fn flush(&self) {}
}

/// Sends engine log records at `max_level` (as in the callback) and more
/// severe to `callback`; null stops them. Call before creating an engine,
/// which otherwise installs its own stdout logger. Panics are logged as
/// errors.
#[no_mangle]
pub extern "C" fn rfvp_mikage_set_log_callback(
    callback: Option<RfvpMikageLogCallback>,
    context: *mut c_void,
    max_level: i32,
) {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        if log::set_logger(&HOST_LOGGER).is_ok() {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                log::error!("panic: {info}");
                previous(info);
            }));
        }
    });
    *HOST_LOGGER
        .sink
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) =
        callback.map(|callback| (callback, context as usize));
    log::set_max_level(match (callback.is_some(), max_level) {
        (false, _) | (_, ..=0) => log::LevelFilter::Off,
        (_, 1) => log::LevelFilter::Error,
        (_, 2) => log::LevelFilter::Warn,
        (_, 3) => log::LevelFilter::Info,
        (_, 4) => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    });
}
