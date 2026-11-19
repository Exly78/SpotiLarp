//! Whether the page can play video (the Now Playing panel's Canvas).
//!
//! On Linux the webview (WebKitGTK) plays video through the system's
//! GStreamer, and when GStreamer has no `autoaudiosink` (it comes with
//! gst-plugins-good) the page's process aborts as soon as a video starts,
//! muted or not, and takes the whole interface with it. The page can't find
//! out safely: `canPlayType` says yes all the same. So this asks GStreamer
//! itself, loading the same system library WebKitGTK plays through.

#[cfg(target_os = "linux")]
pub fn supported() -> bool {
    use std::sync::OnceLock;
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| has_gstreamer_element(c"autoaudiosink"))
}

#[cfg(not(target_os = "linux"))]
pub fn supported() -> bool {
    true
}

#[cfg(target_os = "linux")]
fn has_gstreamer_element(name: &std::ffi::CStr) -> bool {
    use std::ffi::{c_char, c_int, c_void};
    use std::ptr::null_mut;

    type InitCheck = unsafe extern "C" fn(*mut c_int, *mut *mut *mut c_char, *mut *mut c_void) -> c_int;
    type FactoryFind = unsafe extern "C" fn(*const c_char) -> *mut c_void;
    type ObjectUnref = unsafe extern "C" fn(*mut c_void);

    // SAFETY: GStreamer's own C signatures. The library is never unloaded:
    // once initialized, its types stay registered with GLib.
    unsafe {
        let Ok(library) = libloading::Library::new("libgstreamer-1.0.so.0") else {
            return false;
        };
        let library = std::mem::ManuallyDrop::new(library);
        let (Ok(init_check), Ok(factory_find), Ok(object_unref)) = (
            library.get::<InitCheck>(b"gst_init_check\0"),
            library.get::<FactoryFind>(b"gst_element_factory_find\0"),
            library.get::<ObjectUnref>(b"gst_object_unref\0"),
        ) else {
            return false;
        };
        if init_check(null_mut(), null_mut(), null_mut()) == 0 {
            return false;
        }
        let factory = factory_find(name.as_ptr());
        if factory.is_null() {
            return false;
        }
        object_unref(factory);
        true
    }
}
