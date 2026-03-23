use ironrdp_server::{KeyboardEvent, MouseEvent, RdpServerInputHandler};
use macrdp_input::{KeyboardInjector, MouseButton, MouseInjector};

/// Bridges RDP input events to macOS CGEvent injection
pub struct MacInputHandler {
    keyboard: Option<KeyboardInjector>,
    mouse: Option<MouseInjector>,
    last_mouse_x: u16,
    last_mouse_y: u16,
    /// RDP-to-macOS coordinate scale (RDP coords ÷ scale = macOS logical points)
    mouse_scale_x: f64,
    mouse_scale_y: f64,
}

impl MacInputHandler {
    pub fn new(mouse_scale_x: f64, mouse_scale_y: f64) -> Self {
        let keyboard = KeyboardInjector::new()
            .map_err(|e| tracing::error!("Failed to create keyboard injector: {e}"))
            .ok();
        let mouse = MouseInjector::new()
            .map_err(|e| tracing::error!("Failed to create mouse injector: {e}"))
            .ok();

        if keyboard.is_none() || mouse.is_none() {
            tracing::warn!(
                "Input injection may fail — ensure Accessibility permission is granted"
            );
        }

        Self {
            keyboard,
            mouse,
            last_mouse_x: 0,
            last_mouse_y: 0,
            mouse_scale_x: mouse_scale_x.max(1.0),
            mouse_scale_y: mouse_scale_y.max(1.0),
        }
    }
}

impl RdpServerInputHandler for MacInputHandler {
    fn keyboard(&mut self, event: KeyboardEvent) {
        let Some(kb) = &self.keyboard else { return };

        let result = match event {
            KeyboardEvent::Pressed { code, extended } => kb.inject_key(code, extended, true),
            KeyboardEvent::Released { code, extended } => kb.inject_key(code, extended, false),
            KeyboardEvent::UnicodePressed(ch) => kb.inject_unicode(ch, true),
            KeyboardEvent::UnicodeReleased(ch) => kb.inject_unicode(ch, false),
            KeyboardEvent::Synchronize(_flags) => {
                tracing::debug!("Keyboard synchronize event (ignored)");
                Ok(())
            }
        };

        if let Err(e) = result {
            tracing::warn!("Keyboard injection failed: {e}");
        }
    }

    fn mouse(&mut self, event: MouseEvent) {
        let Some(m) = &self.mouse else { return };

        let result = match event {
            MouseEvent::Move { x, y } => {
                // Scale RDP desktop coordinates to macOS logical points
                let mx = (x as f64 / self.mouse_scale_x) as u16;
                let my = (y as f64 / self.mouse_scale_y) as u16;
                self.last_mouse_x = mx;
                self.last_mouse_y = my;
                m.move_to(mx, my)
            }
            MouseEvent::LeftPressed => {
                m.button_event(MouseButton::Left, true, self.last_mouse_x, self.last_mouse_y)
            }
            MouseEvent::LeftReleased => {
                m.button_event(MouseButton::Left, false, self.last_mouse_x, self.last_mouse_y)
            }
            MouseEvent::RightPressed => {
                m.button_event(MouseButton::Right, true, self.last_mouse_x, self.last_mouse_y)
            }
            MouseEvent::RightReleased => {
                m.button_event(MouseButton::Right, false, self.last_mouse_x, self.last_mouse_y)
            }
            MouseEvent::MiddlePressed => {
                m.button_event(MouseButton::Middle, true, self.last_mouse_x, self.last_mouse_y)
            }
            MouseEvent::MiddleReleased => {
                m.button_event(MouseButton::Middle, false, self.last_mouse_x, self.last_mouse_y)
            }
            MouseEvent::VerticalScroll { value } => m.scroll(value),
            _ => {
                tracing::trace!(?event, "Unhandled mouse event");
                Ok(())
            }
        };

        if let Err(e) = result {
            tracing::warn!("Mouse injection failed: {e}");
        }
    }
}
