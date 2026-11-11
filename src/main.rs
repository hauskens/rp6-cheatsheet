//! Front ends: crossterm in a terminal, Ratzilla (WebAssembly) in the browser.

use anyhow::Result;
use rp6_tui::App;

fn main() -> Result<()> {
    let app = App::load()?;
    platform::run(app)
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::time::Duration;

    use anyhow::Result;
    use ratatui::crossterm::event::{self, Event, KeyCode as K, KeyEvent, KeyEventKind, KeyModifiers};
    use rp6_tui::{App, Key, KeyCode};

    fn key(ev: KeyEvent) -> Key {
        let code = match ev.code {
            K::Char(c) => KeyCode::Char(c),
            K::Enter => KeyCode::Enter,
            K::Esc => KeyCode::Esc,
            K::Backspace => KeyCode::Backspace,
            K::Tab => KeyCode::Tab,
            K::BackTab => KeyCode::BackTab,
            K::Left => KeyCode::Left,
            K::Right => KeyCode::Right,
            K::Up => KeyCode::Up,
            K::Down => KeyCode::Down,
            K::Home => KeyCode::Home,
            K::PageUp => KeyCode::PageUp,
            K::PageDown => KeyCode::PageDown,
            _ => KeyCode::Other,
        };
        Key { code, ctrl: ev.modifiers.contains(KeyModifiers::CONTROL) }
    }

    pub fn run(mut app: App) -> Result<()> {
        let mut terminal = ratatui::init();
        let res = (|| -> Result<()> {
            while !app.quit {
                if app.needs_clear {
                    terminal.clear()?;
                    app.needs_clear = false;
                }
                terminal.draw(|f| rp6_tui::draw(f, &mut app))?;
                // Redraw periodically so the key-press animation keeps moving.
                if !event::poll(Duration::from_millis(50))? {
                    continue;
                }
                if let Event::Key(ev) = event::read()?
                    && ev.kind == KeyEventKind::Press
                {
                    app.handle(key(ev));
                }
            }
            Ok(())
        })();
        ratatui::restore();
        res
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use std::cell::RefCell;
    use std::rc::Rc;

    use anyhow::{Result, anyhow};
    use ratzilla::event::{KeyCode as K, KeyEvent};
    use ratzilla::ratatui::Terminal;
    use ratzilla::{DomBackend, WebRenderer};
    use rp6_tui::{App, Key, KeyCode};

    fn key(ev: KeyEvent) -> Key {
        let code = match ev.code {
            K::Char(c) => KeyCode::Char(c),
            K::Enter => KeyCode::Enter,
            K::Esc => KeyCode::Esc,
            K::Backspace => KeyCode::Backspace,
            K::Tab if ev.shift => KeyCode::BackTab,
            K::Tab => KeyCode::Tab,
            K::Left => KeyCode::Left,
            K::Right => KeyCode::Right,
            K::Up => KeyCode::Up,
            K::Down => KeyCode::Down,
            K::Home => KeyCode::Home,
            K::PageUp => KeyCode::PageUp,
            K::PageDown => KeyCode::PageDown,
            _ => KeyCode::Other,
        };
        Key { code, ctrl: ev.ctrl }
    }

    pub fn run(app: App) -> Result<()> {
        let app = Rc::new(RefCell::new(app));
        let backend = DomBackend::new().map_err(|e| anyhow!("{e}"))?;
        let mut terminal = Terminal::new(backend)?;
        terminal
            .on_key_event({
                let app = app.clone();
                move |ev| app.borrow_mut().handle(key(ev))
            })
            .map_err(|e| anyhow!("{e}"))?;
        // Ratzilla redraws every animation frame, which keeps the key-press blinking going.
        terminal.draw_web(move |f| rp6_tui::draw(f, &mut app.borrow_mut()));
        Ok(())
    }
}
