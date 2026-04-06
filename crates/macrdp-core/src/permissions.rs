//! macOS permission checking and requesting

use crate::callbacks::PermissionStatus;

/// Check current macOS permission status (non-blocking, no prompts)
pub fn check_permissions() -> PermissionStatus {
    PermissionStatus {
        screen_capture: macrdp_capture::check_screen_recording_permission(),
        accessibility: macrdp_input::check_accessibility_permission(),
        microphone: false, // Phase 3
    }
}

/// Request all required macOS permissions (may trigger system dialogs).
/// Returns the updated permission status.
pub fn request_permissions() -> PermissionStatus {
    tracing::info!("Checking macOS permissions...");

    // Screen Recording
    if macrdp_capture::check_screen_recording_permission() {
        tracing::info!("[OK] Screen Recording permission granted");
    } else {
        tracing::warn!("[!!] Screen Recording permission NOT granted");
        macrdp_capture::request_screen_recording_permission();
        if !macrdp_capture::check_screen_recording_permission() {
            tracing::error!(
                "Screen Recording denied. Go to: System Settings > Privacy & Security > Screen Recording"
            );
            macrdp_capture::open_screen_recording_settings();
        }
    }

    // Accessibility
    if macrdp_input::check_accessibility_permission() {
        tracing::info!("[OK] Accessibility permission granted");
    } else {
        tracing::warn!("[!!] Accessibility permission NOT granted");
        macrdp_input::request_accessibility_permission();
        if !macrdp_input::check_accessibility_permission() {
            tracing::error!(
                "Accessibility denied. Go to: System Settings > Privacy & Security > Accessibility"
            );
            macrdp_input::open_accessibility_settings();
        }
    }

    check_permissions()
}

/// Detect the main display's logical pixel size
pub fn detect_display_size() -> anyhow::Result<(u32, u32)> {
    macrdp_capture::detect_display_size()
}
