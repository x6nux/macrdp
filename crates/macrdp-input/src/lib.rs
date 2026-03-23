//! Keyboard and mouse input injection via CGEvent

pub mod keyboard;
pub mod keymap;
pub mod mouse;

pub use keyboard::KeyboardInjector;
pub use keymap::scancode_to_keycode;
pub use mouse::{MouseButton, MouseInjector};

// FFI for Accessibility APIs
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
}

/// Check if Accessibility permission is granted (no prompt)
pub fn check_accessibility_permission() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Check Accessibility permission and prompt user if not granted.
/// Returns true if already granted.
pub fn request_accessibility_permission() -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::CFString;

    unsafe {
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();

        // Build CFDictionary manually via CFDictionaryCreate
        let keys = [key.as_CFTypeRef()];
        let values = [value.as_CFTypeRef()];
        let dict = core_foundation::base::CFType::wrap_under_create_rule(
            core_foundation_sys::dictionary::CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr() as *const *const _,
                values.as_ptr() as *const *const _,
                1,
                &core_foundation_sys::dictionary::kCFTypeDictionaryKeyCallBacks,
                &core_foundation_sys::dictionary::kCFTypeDictionaryValueCallBacks,
            ) as *const _,
        );
        AXIsProcessTrustedWithOptions(dict.as_CFTypeRef() as *const _)
    }
}

/// Open System Settings to the Accessibility permission page
pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}
