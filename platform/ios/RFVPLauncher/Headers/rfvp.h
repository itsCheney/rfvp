#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// Legacy entry point (winit runloop). Do NOT use on iOS embedded/SwiftUI.
void rfvp_run_entry(const char* game_root_utf8, const char* nls_utf8);

// Sets the default for instances created after this call.
void rfvp_set_text_hidpi_enabled(int enabled);

// iOS host-mode entry points (SwiftUI/UIKit drives the runloop).
// `ui_view` must be a UIView* whose backing layer is CAMetalLayer.
void* rfvp_ios_create(
    void* ui_view,
    unsigned int width_px,
    unsigned int height_px,
    double native_scale_factor,
    const char* game_root_utf8,
    const char* nls_utf8
);

// Return 1 => exit requested, 0 => continue.
int rfvp_ios_step(void* handle, unsigned int dt_ms);

void rfvp_ios_resize(void* handle, unsigned int width_px, unsigned int height_px);

// phase: 0 began, 1 moved, 2 ended, 3 cancelled
void rfvp_ios_touch(void* handle, int phase, double x_points, double y_points);

// button: 0 left, 1 right. phase: 0 down, 1 move, 2 up, 3 cancelled/up.
void rfvp_ios_mouse_button(void* handle, int button, int phase, double x_points, double y_points);

// Wheel delta is forwarded to the engine wheel accumulator.
void rfvp_ios_mouse_wheel(void* handle, int delta, double x_points, double y_points);

// key: 0 Escape. phase: 0 down, 2 up, 3 cancelled/up.
void rfvp_ios_key(void* handle, int key, int phase);

// enabled: 0 disables HiDPI text backing, non-zero enables it.
void rfvp_ios_set_text_hidpi(void* handle, int enabled);

void rfvp_ios_destroy(void* handle);

// Mikage host additions (crates/rfvp/src/mikage_host.rs).

// The last error from rfvp_ios_create or rfvp_mikage_step, or "".
const char* rfvp_mikage_last_error(void);

// Stand-ins for the Windows faces: face 0 MS Gothic (required, also the
// default), 1 MS Mincho, 2 MS PGothic, 3 MS PMincho. face_name picks a face
// in a collection (PostScript, full or family name); NULL takes the first.
// Call before rfvp_ios_create. Return 1 when the font loaded.
int rfvp_mikage_register_font_face(const char* path_utf8, int face, const char* face_name_utf8);
int rfvp_mikage_register_fallback_font(const char* path_utf8, const char* face_name_utf8);
void rfvp_mikage_clear_fonts(void);

// The script file in the game directory to run (a .bch patch or one of
// several .hcb files); NULL restores the default. Call before create.
void rfvp_mikage_set_script(const char* name_utf8);

// The game folder's folder for saves instead of "save" (Mikage: "savedata");
// NULL restores "save". Call before create.
void rfvp_mikage_set_save_dir(const char* name_utf8);

// 0 running, 1 the game ended, 2 error (rfvp_mikage_last_error).
int rfvp_mikage_step(void* handle, unsigned int dt_ms);

void rfvp_mikage_logical_size(void* handle, unsigned int* width, unsigned int* height);

// 0 background (sounds pause, global save written), 1 foreground.
void rfvp_mikage_set_foreground(void* handle, int foreground);

// Windows virtual-key codes: 0x1B Escape, 0x0D Enter, 0x20 Space,
// 0x25..0x28 arrows, 0x11 Control (held, skips text). down: 1 press, 0 release.
void rfvp_mikage_key(void* handle, int vk_code, int down);

// Trackpad cursor in UIKit points: phase 0 move, 1 left down, 2 left up,
// 3 right down, 4 right up.
void rfvp_mikage_pointer(void* handle, int phase, double x_points, double y_points);

// A right click (down and up) at a point in UIKit points.
void rfvp_mikage_secondary_click(void* handle, double x_points, double y_points);

// level: 1 error, 2 warn, 3 info, 4 debug, 5 trace.
typedef void (*rfvp_mikage_log_callback)(int level, const char* target, const char* message, void* context);
void rfvp_mikage_set_log_callback(rfvp_mikage_log_callback callback, void* context, int max_level);

#ifdef __cplusplus
} // extern "C"
#endif
