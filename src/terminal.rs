use std::{
    io::{self, Stdout, stdout},
    panic,
};

use ratatui::{
    Frame,
    backend::CrosstermBackend,
    crossterm::{
        cursor::Show,
        event::{self, Event as TerminalEvent, KeyEvent, KeyEventKind},
        execute,
        terminal::{
            EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
            is_raw_mode_enabled,
        },
    },
};
use rustix::process::{Signal, kill_current_process_group};

#[derive(Debug, PartialEq)]
pub enum Event {
    Key(KeyEvent),
    Resize,
}

/// The full-screen terminal. Creating it enters raw mode and the alternate screen. Dropping it
/// restores the terminal.
pub struct Terminal {
    screen: ratatui::Terminal<CrosstermBackend<Stdout>>,
}

impl Terminal {
    pub fn enter() -> io::Result<Self> {
        let terminal = Self {
            screen: ratatui::Terminal::new(CrosstermBackend::new(stdout()))?,
        };
        enter_tui_mode()?;
        Ok(terminal)
    }

    pub fn draw(&mut self, render: impl FnOnce(&mut Frame)) -> io::Result<()> {
        tracing::debug!("draw");
        self.screen.draw(render)?;
        Ok(())
    }

    pub fn next_event(&self) -> io::Result<Option<Event>> {
        Ok(to_event(event::read()?))
    }

    /// Suspends the app like Ctrl-z in a shell, and returns after `fg`.
    pub fn suspend(&mut self) -> io::Result<()> {
        leave_tui_mode()?;
        kill_current_process_group(Signal::TSTP)?;
        enter_tui_mode()?;
        self.screen.clear()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = leave_tui_mode();
    }
}

pub fn install_panic_hook() {
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = leave_tui_mode();
        tracing::error!("{panic_info}");
        previous_hook(panic_info);
    }));
}

fn to_event(event: TerminalEvent) -> Option<Event> {
    match event {
        TerminalEvent::Key(key) if key.kind == KeyEventKind::Press => Some(Event::Key(key)),
        TerminalEvent::Resize(..) => Some(Event::Resize),
        _ => None,
    }
}

fn enter_tui_mode() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)
}

fn leave_tui_mode() -> io::Result<()> {
    if !is_raw_mode_enabled()? {
        return Ok(());
    }
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, Show)
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    fn key_with_kind(kind: KeyEventKind) -> KeyEvent {
        KeyEvent::new_with_kind(KeyCode::Char('q'), KeyModifiers::NONE, kind)
    }

    #[test]
    fn given_key_press_when_converting_then_it_is_a_key_event() {
        let press = key_with_kind(KeyEventKind::Press);

        assert_eq!(to_event(TerminalEvent::Key(press)), Some(Event::Key(press)));
    }

    #[test]
    fn given_key_release_when_converting_then_it_is_ignored() {
        let release = key_with_kind(KeyEventKind::Release);

        assert_eq!(to_event(TerminalEvent::Key(release)), None);
    }

    #[test]
    fn given_resize_when_converting_then_it_is_a_resize_event() {
        assert_eq!(to_event(TerminalEvent::Resize(80, 24)), Some(Event::Resize));
    }

    #[test]
    fn given_focus_change_when_converting_then_it_is_ignored() {
        assert_eq!(to_event(TerminalEvent::FocusGained), None);
    }
}
