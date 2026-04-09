use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;
use std::ffi::c_void;

// --- AX API FFI ---

type AXUIElementRef = *const c_void;
type AXError = i32;
const K_AX_ERROR_SUCCESS: AXError = 0;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: libc::pid_t) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: core_foundation_sys::string::CFStringRef,
        value: *mut core_foundation_sys::base::CFTypeRef,
    ) -> AXError;
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(
        options: core_foundation_sys::dictionary::CFDictionaryRef,
    ) -> bool;
    fn AXUIElementPerformAction(
        element: AXUIElementRef,
        action: core_foundation_sys::string::CFStringRef,
    ) -> AXError;
}

// --- Objective-C Runtime FFI ---

extern "C" {
    fn objc_getClass(name: *const libc::c_char) -> *mut c_void;
    fn sel_registerName(name: *const libc::c_char) -> *mut c_void;
    fn objc_msgSend(receiver: *mut c_void, sel: *mut c_void, ...) -> *mut c_void;
}

// --- Dev app definitions ---

pub struct DevApp {
    pub name: &'static str,
    pub bundle_id: &'static str,
    pub storage_path: &'static str,
    pub title_suffix: &'static str,
}

pub const DEV_APPS: &[DevApp] = &[
    DevApp {
        name: "Visual Studio Code",
        bundle_id: "com.microsoft.VSCode",
        storage_path: "Code/User/globalStorage/state.vscdb",
        title_suffix: "Visual Studio Code",
    },
    DevApp {
        name: "VSCode Insiders",
        bundle_id: "com.microsoft.VSCodeInsiders",
        storage_path: "Code - Insiders/User/globalStorage/state.vscdb",
        title_suffix: "Visual Studio Code - Insiders",
    },
    DevApp {
        name: "Cursor",
        bundle_id: "com.todesktop.230313mzl4w4u92",
        storage_path: "Cursor/User/globalStorage/state.vscdb",
        title_suffix: "Cursor",
    },
];

// --- Permission checks ---

pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn request_permission() -> bool {
    unsafe {
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;

        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();

        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        AXIsProcessTrustedWithOptions(
            options.as_concrete_TypeRef() as core_foundation_sys::dictionary::CFDictionaryRef,
        )
    }
}

// --- Find running apps ---

pub fn find_pids_for_bundle_id(bundle_id: &str) -> Vec<i32> {
    unsafe {
        let cls = objc_getClass(b"NSRunningApplication\0".as_ptr() as *const libc::c_char);
        if cls.is_null() {
            return vec![];
        }

        let sel = sel_registerName(
            b"runningApplicationsWithBundleIdentifier:\0".as_ptr() as *const libc::c_char,
        );

        let bundle_cf = CFString::new(bundle_id);
        let apps: *mut c_void =
            objc_msgSend(cls, sel, bundle_cf.as_concrete_TypeRef() as *const c_void);

        if apps.is_null() {
            return vec![];
        }

        let count_sel = sel_registerName(b"count\0".as_ptr() as *const libc::c_char);
        let count = objc_msgSend(apps, count_sel) as usize;

        let mut pids = Vec::new();
        let obj_at_sel =
            sel_registerName(b"objectAtIndex:\0".as_ptr() as *const libc::c_char);
        let pid_sel =
            sel_registerName(b"processIdentifier\0".as_ptr() as *const libc::c_char);

        for i in 0..count {
            let app = objc_msgSend(apps, obj_at_sel, i as u64);
            if !app.is_null() {
                let pid = objc_msgSend(app, pid_sel) as i32;
                if pid > 0 {
                    pids.push(pid);
                }
            }
        }

        pids
    }
}

// --- Window enumeration ---

pub fn get_window_titles(pid: i32) -> Vec<String> {
    unsafe {
        let app_element = AXUIElementCreateApplication(pid);
        if app_element.is_null() {
            return vec![];
        }

        let windows_attr = CFString::new("AXWindows");
        let mut windows_value: core_foundation_sys::base::CFTypeRef = std::ptr::null();

        let err = AXUIElementCopyAttributeValue(
            app_element,
            windows_attr.as_concrete_TypeRef(),
            &mut windows_value,
        );

        if err != K_AX_ERROR_SUCCESS || windows_value.is_null() {
            core_foundation::base::CFRelease(app_element as core_foundation_sys::base::CFTypeRef);
            return vec![];
        }

        let windows_array = CFArray::<CFType>::wrap_under_create_rule(
            windows_value as core_foundation_sys::array::CFArrayRef,
        );

        let mut titles = Vec::new();
        let title_attr = CFString::new("AXTitle");

        for i in 0..windows_array.len() {
            if let Some(window) = windows_array.get(i) {
                let window_ref = window.as_CFTypeRef() as AXUIElementRef;
                let mut title_value: core_foundation_sys::base::CFTypeRef = std::ptr::null();

                let err = AXUIElementCopyAttributeValue(
                    window_ref,
                    title_attr.as_concrete_TypeRef(),
                    &mut title_value,
                );

                if err == K_AX_ERROR_SUCCESS && !title_value.is_null() {
                    let title_cf = CFString::wrap_under_create_rule(
                        title_value as core_foundation_sys::string::CFStringRef,
                    );
                    let s = title_cf.to_string();
                    if !s.is_empty() {
                        titles.push(s);
                    }
                }
            }
        }

        core_foundation::base::CFRelease(app_element as core_foundation_sys::base::CFTypeRef);
        titles
    }
}

// --- Window focusing (two-step) ---

pub fn focus_window(pid: i32, window_index: usize) -> Result<(), String> {
    unsafe {
        // Step 1: Activate the app via NSRunningApplication
        activate_app(pid)?;

        // Step 2: Raise the specific window via AX API
        let app_element = AXUIElementCreateApplication(pid);
        if app_element.is_null() {
            return Err("Failed to create AX element for app".to_string());
        }

        let windows_attr = CFString::new("AXWindows");
        let mut windows_value: core_foundation_sys::base::CFTypeRef = std::ptr::null();

        let err = AXUIElementCopyAttributeValue(
            app_element,
            windows_attr.as_concrete_TypeRef(),
            &mut windows_value,
        );

        if err != K_AX_ERROR_SUCCESS || windows_value.is_null() {
            core_foundation::base::CFRelease(app_element as core_foundation_sys::base::CFTypeRef);
            return Err("Failed to get windows".to_string());
        }

        let windows_array = CFArray::<CFType>::wrap_under_create_rule(
            windows_value as core_foundation_sys::array::CFArrayRef,
        );

        if window_index >= windows_array.len() as usize {
            core_foundation::base::CFRelease(app_element as core_foundation_sys::base::CFTypeRef);
            return Err("Window index out of bounds".to_string());
        }

        if let Some(window) = windows_array.get(window_index as isize) {
            let window_ref = window.as_CFTypeRef() as AXUIElementRef;
            let raise_action = CFString::new("AXRaise");

            let err =
                AXUIElementPerformAction(window_ref, raise_action.as_concrete_TypeRef());

            if err != K_AX_ERROR_SUCCESS {
                core_foundation::base::CFRelease(
                    app_element as core_foundation_sys::base::CFTypeRef,
                );
                return Err(format!("AXRaise failed with error {}", err));
            }
        }

        core_foundation::base::CFRelease(app_element as core_foundation_sys::base::CFTypeRef);
        Ok(())
    }
}

unsafe fn activate_app(pid: i32) -> Result<(), String> {
    // Find the app by iterating all running apps and matching PID
    let workspace_cls = objc_getClass(b"NSWorkspace\0".as_ptr() as *const libc::c_char);
    let shared_sel = sel_registerName(b"sharedWorkspace\0".as_ptr() as *const libc::c_char);
    let workspace = objc_msgSend(workspace_cls, shared_sel);

    let apps_sel = sel_registerName(b"runningApplications\0".as_ptr() as *const libc::c_char);
    let apps = objc_msgSend(workspace, apps_sel);

    if apps.is_null() {
        return Err("Could not get running applications".to_string());
    }

    let count_sel = sel_registerName(b"count\0".as_ptr() as *const libc::c_char);
    let count = objc_msgSend(apps, count_sel) as usize;

    let obj_at_sel = sel_registerName(b"objectAtIndex:\0".as_ptr() as *const libc::c_char);
    let pid_sel = sel_registerName(b"processIdentifier\0".as_ptr() as *const libc::c_char);
    let activate_sel = sel_registerName(
        b"activateWithOptions:\0".as_ptr() as *const libc::c_char,
    );

    for i in 0..count {
        let app = objc_msgSend(apps, obj_at_sel, i as u64);
        if !app.is_null() {
            let app_pid = objc_msgSend(app, pid_sel) as i32;
            if app_pid == pid {
                // NSApplicationActivateIgnoringOtherApps = 1 << 1 = 2
                let _result = objc_msgSend(app, activate_sel, 2u64);
                return Ok(());
            }
        }
    }

    Err(format!("App with PID {} not found", pid))
}
