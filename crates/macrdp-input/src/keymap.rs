/// Convert an RDP scancode (Set 1) to a macOS virtual keycode.
/// `extended` indicates an extended key (e.g., right Ctrl, arrow keys).
/// Returns None if the scancode has no known mapping.
pub fn scancode_to_keycode(scancode: u8, extended: bool) -> Option<u16> {
    if extended {
        EXTENDED_SCANCODE_MAP.get(&scancode).copied()
    } else {
        SCANCODE_MAP.get(&scancode).copied()
    }
}

use std::collections::HashMap;
use std::sync::LazyLock;

/// Standard (non-extended) RDP scancode → macOS keycode
static SCANCODE_MAP: LazyLock<HashMap<u8, u16>> = LazyLock::new(|| {
    HashMap::from([
        // Row 1: Escape + F-keys
        (0x01, 0x35u16), // Escape
        (0x3B, 0x7A),    // F1
        (0x3C, 0x78),    // F2
        (0x3D, 0x63),    // F3
        (0x3E, 0x76),    // F4
        (0x3F, 0x60),    // F5
        (0x40, 0x61),    // F6
        (0x41, 0x62),    // F7
        (0x42, 0x64),    // F8
        (0x43, 0x65),    // F9
        (0x44, 0x6D),    // F10
        (0x57, 0x67),    // F11
        (0x58, 0x6F),    // F12
        // Row 2: Number row
        (0x29, 0x32), // ` ~
        (0x02, 0x12), // 1
        (0x03, 0x13), // 2
        (0x04, 0x14), // 3
        (0x05, 0x15), // 4
        (0x06, 0x17), // 5
        (0x07, 0x16), // 6
        (0x08, 0x1A), // 7
        (0x09, 0x1C), // 8
        (0x0A, 0x19), // 9
        (0x0B, 0x1D), // 0
        (0x0C, 0x1B), // - _
        (0x0D, 0x18), // = +
        (0x0E, 0x33), // Backspace
        // Row 3: QWERTY
        (0x0F, 0x30), // Tab
        (0x10, 0x0C), // Q
        (0x11, 0x0D), // W
        (0x12, 0x0E), // E
        (0x13, 0x0F), // R
        (0x14, 0x11), // T
        (0x15, 0x10), // Y
        (0x16, 0x20), // U
        (0x17, 0x22), // I
        (0x18, 0x1F), // O
        (0x19, 0x23), // P
        (0x1A, 0x21), // [ {
        (0x1B, 0x1E), // ] }
        (0x2B, 0x2A), // \ |
        // Row 4: ASDF
        (0x3A, 0x39), // Caps Lock
        (0x1E, 0x00), // A
        (0x1F, 0x01), // S
        (0x20, 0x02), // D
        (0x21, 0x03), // F
        (0x22, 0x05), // G
        (0x23, 0x04), // H
        (0x24, 0x26), // J
        (0x25, 0x28), // K
        (0x26, 0x25), // L
        (0x27, 0x29), // ; :
        (0x28, 0x27), // ' "
        (0x1C, 0x24), // Enter
        // Row 5: ZXCV
        (0x2A, 0x38), // Left Shift
        (0x2C, 0x06), // Z
        (0x2D, 0x07), // X
        (0x2E, 0x08), // C
        (0x2F, 0x09), // V
        (0x30, 0x0B), // B
        (0x31, 0x2D), // N
        (0x32, 0x2E), // M
        (0x33, 0x2B), // , <
        (0x34, 0x2F), // . >
        (0x35, 0x2C), // / ?
        (0x36, 0x3C), // Right Shift
        // Row 6: Bottom — modifier mapping for Windows → macOS
        // Physical layout: [Ctrl] [Win] [Alt] [Space] [Alt] [Win] [Ctrl]
        //         macOS:   [Ctrl] [Opt] [Cmd] [Space] [Cmd] [Opt] [Ctrl]
        // So: Ctrl→Control, Alt→Command (same position, primary modifier), Win→Option
        (0x1D, 0x3B), // Left Ctrl → Left Control
        (0x38, 0x37), // Left Alt → Left Command (⌘) — most intuitive: Alt+C → Cmd+C = copy
        (0x39, 0x31), // Space
        // Numpad — always treat as numeric input (ignore NumLock state).
        // Non-extended scancodes map to numpad digits/operators;
        // extended versions of same scancodes map to navigation (in EXTENDED_SCANCODE_MAP).
        (0x52, 0x52), // Numpad 0
        (0x4F, 0x53), // Numpad 1
        (0x50, 0x54), // Numpad 2
        (0x51, 0x55), // Numpad 3
        (0x4B, 0x56), // Numpad 4
        (0x4C, 0x57), // Numpad 5
        (0x4D, 0x58), // Numpad 6
        (0x47, 0x59), // Numpad 7
        (0x48, 0x5B), // Numpad 8
        (0x49, 0x5C), // Numpad 9
        (0x53, 0x41), // Numpad . (decimal)
        (0x37, 0x43), // Numpad * (multiply)
        (0x4A, 0x4E), // Numpad - (subtract)
        (0x4E, 0x45), // Numpad + (add)
        // Misc
        (0x45, 0x47), // Num Lock → Clear (macOS equivalent)
        (0x46, 0x6B), // Scroll Lock → F14
    ])
});

/// Extended RDP scancode → macOS keycode
static EXTENDED_SCANCODE_MAP: LazyLock<HashMap<u8, u16>> = LazyLock::new(|| {
    HashMap::from([
        // Modifier keys — Windows → macOS position-based mapping:
        // Ctrl→Control, Alt→Command, Win→Option
        (0x1D, 0x3Eu16), // Right Ctrl → Right Control
        (0x38, 0x36),    // Right Alt → Right Command (⌘) — consistent with Left Alt→Left Cmd
        (0x5B, 0x3A),    // Left Win → Left Option (⌥) — by position (between Ctrl and Alt/Cmd)
        (0x5C, 0x3D),    // Right Win → Right Option (⌥)
        // Arrow keys
        (0x48, 0x7E), // Up
        (0x50, 0x7D), // Down
        (0x4B, 0x7B), // Left
        (0x4D, 0x7C), // Right
        // Navigation
        (0x47, 0x73), // Home
        (0x4F, 0x77), // End
        (0x49, 0x74), // Page Up
        (0x51, 0x79), // Page Down
        (0x52, 0x72), // Insert (→ Help on Mac)
        (0x53, 0x75), // Delete
        // Numpad (extended)
        (0x35, 0x4B), // Numpad / (divide)
        (0x1C, 0x4C), // Numpad Enter
    ])
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_keys() {
        assert_eq!(scancode_to_keycode(0x1E, false), Some(0x00)); // A
        assert_eq!(scancode_to_keycode(0x1C, false), Some(0x24)); // Enter
        assert_eq!(scancode_to_keycode(0x39, false), Some(0x31)); // Space
        assert_eq!(scancode_to_keycode(0x01, false), Some(0x35)); // Escape
    }

    #[test]
    fn test_modifier_keys() {
        assert_eq!(scancode_to_keycode(0x2A, false), Some(0x38)); // Left Shift
        assert_eq!(scancode_to_keycode(0x1D, false), Some(0x3B)); // Left Ctrl
        assert_eq!(scancode_to_keycode(0x38, false), Some(0x37)); // Left Alt → Cmd
    }

    #[test]
    fn test_extended_keys() {
        assert_eq!(scancode_to_keycode(0x1D, true), Some(0x3E)); // Right Ctrl → Right Control
        assert_eq!(scancode_to_keycode(0x38, true), Some(0x36)); // Right Alt → Right Command
        assert_eq!(scancode_to_keycode(0x5B, true), Some(0x3A)); // Left Win → Left Option
        assert_eq!(scancode_to_keycode(0x5C, true), Some(0x3D)); // Right Win → Right Option
        assert_eq!(scancode_to_keycode(0x48, true), Some(0x7E)); // Arrow Up
        assert_eq!(scancode_to_keycode(0x50, true), Some(0x7D)); // Arrow Down
        assert_eq!(scancode_to_keycode(0x4B, true), Some(0x7B)); // Arrow Left
        assert_eq!(scancode_to_keycode(0x4D, true), Some(0x7C)); // Arrow Right
    }

    #[test]
    fn test_unknown_scancode() {
        assert_eq!(scancode_to_keycode(0xFF, false), None);
    }

    #[test]
    fn test_number_keys() {
        assert_eq!(scancode_to_keycode(0x02, false), Some(0x12)); // 1
        assert_eq!(scancode_to_keycode(0x0B, false), Some(0x1D)); // 0
    }
}
