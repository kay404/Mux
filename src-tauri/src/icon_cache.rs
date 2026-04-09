use core_foundation::base::TCFType;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;

static ICON_CACHE: std::sync::LazyLock<Mutex<HashMap<String, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Get the app icon as a base64 PNG data URI, using memory + disk cache.
pub fn get_icon_data_uri(bundle_id: &str) -> Option<String> {
    // Check memory cache
    {
        let cache = ICON_CACHE.lock().ok()?;
        if let Some(uri) = cache.get(bundle_id) {
            return Some(uri.clone());
        }
    }

    // Check disk cache
    let cache_dir = disk_cache_dir();
    let cache_file = cache_dir.join(format!("{}.png.b64", bundle_id));

    if cache_file.exists() {
        if let Ok(data_uri) = std::fs::read_to_string(&cache_file) {
            if !data_uri.is_empty() {
                let mut cache = ICON_CACHE.lock().ok()?;
                cache.insert(bundle_id.to_string(), data_uri.clone());
                return Some(data_uri);
            }
        }
    }

    // Extract icon from app bundle
    let data_uri = extract_icon(bundle_id)?;

    // Write to disk cache
    let _ = std::fs::create_dir_all(&cache_dir);
    let _ = std::fs::write(&cache_file, &data_uri);

    // Write to memory cache
    if let Ok(mut cache) = ICON_CACHE.lock() {
        cache.insert(bundle_id.to_string(), data_uri.clone());
    }

    Some(data_uri)
}

fn disk_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/mux/icons")
}

/// Extract app icon using NSWorkspace.icon(forFile:) via Objective-C runtime.
/// Returns a data URI: "data:image/png;base64,..."
fn extract_icon(bundle_id: &str) -> Option<String> {
    unsafe {
        // Find the app's bundle path via NSRunningApplication or NSWorkspace
        let bundle_path = find_bundle_path(bundle_id)?;

        // Get icon via NSWorkspace
        let workspace_cls =
            objc_getClass(b"NSWorkspace\0".as_ptr() as *const libc::c_char);
        let shared_sel =
            sel_registerName(b"sharedWorkspace\0".as_ptr() as *const libc::c_char);
        let workspace = objc_msgSend(workspace_cls, shared_sel);

        let path_cf = core_foundation::string::CFString::new(&bundle_path);

        // Create NSString from CFString (they're toll-free bridged)
        let icon_sel =
            sel_registerName(b"iconForFile:\0".as_ptr() as *const libc::c_char);
        let ns_image = objc_msgSend(
            workspace,
            icon_sel,
            path_cf.as_concrete_TypeRef() as *const c_void,
        );

        if ns_image.is_null() {
            return None;
        }

        // Set the desired size (20x20 as per design specs)
        let set_size_sel =
            sel_registerName(b"setSize:\0".as_ptr() as *const libc::c_char);
        // NSSize is {width: f64, height: f64} on x86_64
        #[repr(C)]
        struct NSSize {
            width: f64,
            height: f64,
        }
        let size = NSSize {
            width: 40.0,  // 2x for retina
            height: 40.0,
        };
        let _: *mut c_void = {
            let func: extern "C" fn(*mut c_void, *mut c_void, NSSize) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            func(ns_image, set_size_sel, size)
        };

        // Get TIFF representation
        let tiff_sel =
            sel_registerName(b"TIFFRepresentation\0".as_ptr() as *const libc::c_char);
        let tiff_data = objc_msgSend(ns_image, tiff_sel);
        if tiff_data.is_null() {
            return None;
        }

        // Create NSBitmapImageRep from TIFF data
        let bitmap_cls =
            objc_getClass(b"NSBitmapImageRep\0".as_ptr() as *const libc::c_char);
        let init_sel =
            sel_registerName(b"imageRepWithData:\0".as_ptr() as *const libc::c_char);
        let bitmap = objc_msgSend(bitmap_cls, init_sel, tiff_data);
        if bitmap.is_null() {
            return None;
        }

        // Get PNG data
        // NSBitmapImageFileTypePNG = 4
        let png_sel = sel_registerName(
            b"representationUsingType:properties:\0".as_ptr() as *const libc::c_char,
        );
        let empty_dict_cls =
            objc_getClass(b"NSDictionary\0".as_ptr() as *const libc::c_char);
        let dict_sel =
            sel_registerName(b"dictionary\0".as_ptr() as *const libc::c_char);
        let empty_dict = objc_msgSend(empty_dict_cls, dict_sel);

        let png_data = objc_msgSend(bitmap, png_sel, 4u64, empty_dict);
        if png_data.is_null() {
            return None;
        }

        // Get bytes from NSData
        let bytes_sel = sel_registerName(b"bytes\0".as_ptr() as *const libc::c_char);
        let length_sel = sel_registerName(b"length\0".as_ptr() as *const libc::c_char);
        let bytes = objc_msgSend(png_data, bytes_sel) as *const u8;
        let length = objc_msgSend(png_data, length_sel) as usize;

        if bytes.is_null() || length == 0 {
            return None;
        }

        let slice = std::slice::from_raw_parts(bytes, length);
        // Base64 encode
        let b64 = base64_encode(slice);
        Some(format!("data:image/png;base64,{}", b64))
    }
}

fn find_bundle_path(bundle_id: &str) -> Option<String> {
    unsafe {
        let workspace_cls =
            objc_getClass(b"NSWorkspace\0".as_ptr() as *const libc::c_char);
        let shared_sel =
            sel_registerName(b"sharedWorkspace\0".as_ptr() as *const libc::c_char);
        let workspace = objc_msgSend(workspace_cls, shared_sel);

        let url_sel = sel_registerName(
            b"URLForApplicationWithBundleIdentifier:\0".as_ptr() as *const libc::c_char,
        );

        let bundle_cf = core_foundation::string::CFString::new(bundle_id);
        let url = objc_msgSend(
            workspace,
            url_sel,
            bundle_cf.as_concrete_TypeRef() as *const c_void,
        );

        if url.is_null() {
            return None;
        }

        let path_sel = sel_registerName(b"path\0".as_ptr() as *const libc::c_char);
        let path_ns = objc_msgSend(url, path_sel);
        if path_ns.is_null() {
            return None;
        }

        let utf8_sel = sel_registerName(b"UTF8String\0".as_ptr() as *const libc::c_char);
        let cstr = objc_msgSend(path_ns, utf8_sel) as *const libc::c_char;
        if cstr.is_null() {
            return None;
        }

        let s = std::ffi::CStr::from_ptr(cstr).to_str().ok()?;
        Some(s.to_string())
    }
}

// Simple base64 encoder (avoids adding another dependency)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

// Obj-C runtime FFI (duplicated from accessibility.rs to keep modules independent)
extern "C" {
    fn objc_getClass(name: *const libc::c_char) -> *mut c_void;
    fn sel_registerName(name: *const libc::c_char) -> *mut c_void;
    fn objc_msgSend(receiver: *mut c_void, sel: *mut c_void, ...) -> *mut c_void;
}
