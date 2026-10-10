use std::{
    io::{self, BufWriter, Stdout, stdout},
    panic, thread,
    time::{Duration, Instant},
};

use ratatui::{
    Frame,
    backend::CrosstermBackend,
    crossterm::{
        cursor::Show,
        event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind},
        execute, queue,
        terminal::{
            BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen,
            LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, is_raw_mode_enabled,
        },
    },
    layout::Size,
};
use rustix::process::{Signal, kill_current_process_group};

const OUTPUT_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, PartialEq)]
pub enum TerminalEvent {
    Key(KeyEvent),
    Resize(Size),
}

/// The full-screen terminal. Creating it enters raw mode and the alternate screen. Dropping it
/// restores the terminal.
pub struct Terminal {
    screen: ratatui::Terminal<CrosstermBackend<BufWriter<Stdout>>>,
}

impl Terminal {
    pub fn enter() -> io::Result<Self> {
        let terminal = Self {
            screen: ratatui::Terminal::new(CrosstermBackend::new(BufWriter::with_capacity(
                OUTPUT_BUFFER_SIZE,
                stdout(),
            )))?,
        };
        enter_tui_mode()?;
        Ok(terminal)
    }

    /// Draws a frame as one synchronized update, so the terminal never shows it half-drawn.
    pub fn draw(&mut self, render: impl FnOnce(&mut Frame)) -> io::Result<()> {
        let draw_start = Instant::now();
        queue!(self.screen.backend_mut(), BeginSynchronizedUpdate)?;
        let render_time = self.draw_measuring_render_time(render)?;
        execute!(self.screen.backend_mut(), EndSynchronizedUpdate)?;
        tracing::trace!(
            render_us = render_time.as_micros(),
            total_us = draw_start.elapsed().as_micros(),
            "draw"
        );
        Ok(())
    }

    fn draw_measuring_render_time(
        &mut self,
        render: impl FnOnce(&mut Frame),
    ) -> io::Result<Duration> {
        let mut render_time = Duration::ZERO;
        self.screen.draw(|frame| {
            let render_start = Instant::now();
            render(frame);
            render_time = render_start.elapsed();
        })?;
        Ok(render_time)
    }

    pub fn size(&self) -> io::Result<Size> {
        self.screen.size()
    }

    pub fn next_event(&self, timeout: Option<Duration>) -> io::Result<Option<TerminalEvent>> {
        if let Some(timeout) = timeout
            && !event::poll(timeout)?
        {
            return Ok(None);
        }
        Ok(to_terminal_event(event::read()?))
    }

    pub fn has_waiting_event(&self) -> io::Result<bool> {
        event::poll(Duration::ZERO)
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

/// Restores the terminal when the main thread panics. A panic on another thread is only logged,
/// because the app keeps running and the default hook would write over the screen.
pub fn install_panic_hook() {
    let main_thread = thread::current().id();
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        tracing::error!("{panic_info}");
        if thread::current().id() == main_thread {
            let _ = leave_tui_mode();
            previous_hook(panic_info);
        }
    }));
}

fn to_terminal_event(crossterm_event: CrosstermEvent) -> Option<TerminalEvent> {
    match crossterm_event {
        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
            Some(TerminalEvent::Key(key))
        }
        CrosstermEvent::Resize(width, height) => {
            Some(TerminalEvent::Resize(Size::new(width, height)))
        }
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

        assert_eq!(
            to_terminal_event(CrosstermEvent::Key(press)),
            Some(TerminalEvent::Key(press))
        );
    }

    #[test]
    fn given_key_release_when_converting_then_it_is_ignored() {
        let release = key_with_kind(KeyEventKind::Release);

        assert_eq!(to_terminal_event(CrosstermEvent::Key(release)), None);
    }

    #[test]
    fn given_resize_when_converting_then_it_is_a_resize_event_with_the_new_size() {
        assert_eq!(
            to_terminal_event(CrosstermEvent::Resize(80, 24)),
            Some(TerminalEvent::Resize(Size::new(80, 24)))
        );
    }

    #[test]
    fn given_focus_change_when_converting_then_it_is_ignored() {
        assert_eq!(to_terminal_event(CrosstermEvent::FocusGained), None);
    }
}
