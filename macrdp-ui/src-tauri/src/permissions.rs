/// Open the macOS System Preferences to a specific privacy pane.
///
/// Supported pane names:
/// - `"screen_recording"` / `"screen_capture"` — Privacy > Screen Recording
/// - `"accessibility"` — Privacy > Accessibility
/// - `"microphone"` — Privacy > Microphone
pub fn open_system_preferences(pane: &str) -> Result<(), String> {
    let url = match pane {
        "screen_recording" | "screen_capture" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        }
        "accessibility" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        "microphone" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
        _ => return Err(format!("Unknown preferences pane: {pane}")),
    };

    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map_err(|e| format!("Failed to open System Preferences: {e}"))?;

    Ok(())
}
