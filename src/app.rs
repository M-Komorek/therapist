use std::{path::PathBuf, sync::Arc};

use crossterm::event::KeyEvent;
use ratatui::layout::{Rect, Size};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::debug;

use crate::{
    action::Action,
    config::Config,
    domain::file::RawLogs,
    tui::{Event, Tui},
    ui::components::{fps::FpsCounter, logview::LogView},
    ui::compositor::Compositor,
    ui::layout::{LayoutManager, LayoutSlot},
};

pub struct App {
    tui: Tui,
    config: Config,
    component_menager: Compositor,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64, file_path: PathBuf) -> color_eyre::Result<Self> {
        let tui = Tui::new()?.tick_rate(tick_rate).frame_rate(frame_rate);

        let config = Config::new()?;
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let layout = LayoutManager::new(tui.size()?);
        let mut component_menager = Compositor::new(action_tx.clone(), layout);

        let source = Arc::new(RawLogs::open(&file_path)?);
        component_menager.register_component(LogView::new(source), LayoutSlot::Main, &config)?;
        component_menager.register_component(
            FpsCounter::default(),
            LayoutSlot::Unassigned,
            &config,
        )?;

        Ok(Self {
            tui,
            component_menager,
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Home,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
        })
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        self.tui.enter()?;

        loop {
            self.handle_events().await?;
            self.handle_actions()?;
            if self.should_suspend {
                self.tui.suspend()?;
                self.action_tx.send(Action::Resume)?;
                self.action_tx.send(Action::ClearScreen)?;
                self.tui.enter()?;
            } else if self.should_quit {
                self.tui.stop()?;
                break;
            }
        }
        self.tui.exit()?;

        Ok(())
    }

    async fn handle_events(&mut self) -> color_eyre::Result<()> {
        let Some(event) = self.tui.next_event().await else {
            return Ok(());
        };

        match event {
            Event::Quit => self.action_tx.send(Action::Quit)?,
            Event::Tick => self.action_tx.send(Action::Tick)?,
            Event::Render => self.action_tx.send(Action::Render)?,
            Event::Resize(x, y) => self.action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }

        self.component_menager.handle_events(Some(event))?;

        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        let Some(keymap) = self.config.keybindings.0.get(&self.mode) else {
            return Ok(());
        };

        match keymap.get(&vec![key]) {
            Some(action) => {
                debug!("Got action: {action:?}");
                self.action_tx.send(action.clone())?;
                // Single-key match breaks any in-progress multi-key sequence.
                self.last_tick_key_events.clear();
            }
            _ => {
                // Key is not a single-key binding — accumulate for multi-key sequence.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations.
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    debug!("Got action: {action:?}");
                    self.action_tx.send(action.clone())?;
                    // Sequence completed — clear so next sequence starts fresh.
                    self.last_tick_key_events.clear();
                }
            }
        }

        Ok(())
    }

    fn handle_actions(&mut self) -> color_eyre::Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }

            match action {
                Action::Tick if self.last_tick_key_events.len() > 1 => {
                    self.last_tick_key_events.clear();
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => self.tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(w, h)?,
                Action::Render => self.component_menager.draw(&mut self.tui)?,
                _ => {}
            }

            self.component_menager.handle_actions(action)?;
        }

        Ok(())
    }

    fn handle_resize(&mut self, w: u16, h: u16) -> color_eyre::Result<()> {
        self.tui.resize(Rect::new(0, 0, w, h))?;
        self.component_menager.handle_resize(Size::new(w, h))?;
        self.component_menager.draw(&mut self.tui)?;
        Ok(())
    }
}
