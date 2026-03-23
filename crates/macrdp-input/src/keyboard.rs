use anyhow::Result;
use core_graphics::event::{CGEvent, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use crate::keymap::scancode_to_keycode;

pub struct KeyboardInjector;

impl KeyboardInjector {
    pub fn new() -> Result<Self> {
        // Verify we can create an event source (permission check)
        let _ = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| anyhow::anyhow!("Failed to create CGEventSource — check Accessibility permission"))?;
        Ok(Self)
    }

    fn source() -> Result<CGEventSource> {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| anyhow::anyhow!("Failed to create CGEventSource"))
    }

    /// Inject a key press or release event
    pub fn inject_key(&self, scancode: u8, extended: bool, pressed: bool) -> Result<()> {
        let keycode = match scancode_to_keycode(scancode, extended) {
            Some(kc) => kc,
            None => {
                tracing::warn!(scancode, extended, "Unknown scancode, ignoring");
                return Ok(());
            }
        };

        let source = Self::source()?;
        let event = CGEvent::new_keyboard_event(source, keycode as CGKeyCode, pressed)
            .map_err(|_| anyhow::anyhow!("Failed to create keyboard event"))?;

        event.post(CGEventTapLocation::HID);
        tracing::trace!(scancode, keycode, pressed, "Keyboard event injected");
        Ok(())
    }

    /// Inject a unicode character press/release
    pub fn inject_unicode(&self, ch: u16, pressed: bool) -> Result<()> {
        let source = Self::source()?;
        let event = CGEvent::new_keyboard_event(source, 0, pressed)
            .map_err(|_| anyhow::anyhow!("Failed to create unicode event"))?;

        if pressed {
            event.set_string_from_utf16_unchecked(&[ch]);
        }

        event.post(CGEventTapLocation::HID);
        Ok(())
    }
}
