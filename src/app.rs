use std::{path::PathBuf, sync::Arc};

use crossterm::event::KeyEvent;
use ratatui::layout::{Rect, Size};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::debug;

use crate::{
    action::Action,
    config::Config,
    data::{RawLogs, contract::LogSource},
    domain::{SearchResult, search::search},
    tui::{Event, Tui},
    ui::components::{
        fps::FpsCounter, logview::LogView, searchbar::SearchBar, statusbar::StatusBar,
    },
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
    source: Arc<dyn LogSource>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Normal,
    Search,
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64, file_path: PathBuf) -> color_eyre::Result<Self> {
        let tui = Tui::new()?.tick_rate(tick_rate).frame_rate(frame_rate);

        let config = Config::new()?;
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let layout = LayoutManager::new(tui.size()?);
        let mut component_menager = Compositor::new(action_tx.clone(), layout);

        let source: Arc<dyn LogSource> = Arc::new(RawLogs::open(&file_path)?);

        component_menager.register_component(
            LogView::new(Arc::clone(&source)),
            LayoutSlot::Main,
            &config,
        )?;
        component_menager.register_component(
            FpsCounter::default(),
            LayoutSlot::Unassigned,
            &config,
        )?;
        component_menager.register_component(
            StatusBar::new(file_path.to_string_lossy().into_owned()),
            LayoutSlot::Footer,
            &config,
        )?;
        component_menager.register_component(SearchBar::new(), LayoutSlot::Footer, &config)?;

        Ok(Self {
            tui,
            component_menager,
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Normal,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            source,
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

        if (self.mode == Mode::Search) {
            self.component_menager.handle_events(Some(event))?;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        match self.mode {
            Mode::Search => {
                // In Search mode all keys are handled by SearchBar via handle_key_event.
                // SearchBar is already called by Compositor::handle_events above.
                // Nothing extra to do here — SearchBar emits SearchCommit / ExitSearch itself.
            }
            Mode::Normal => {
                let Some(keymap) = self.config.keybindings.0.get(&self.mode) else {
                    return Ok(());
                };

                match keymap.get(&vec![key]) {
                    Some(action) => {
                        debug!("Got action: {action:?}");
                        self.action_tx.send(action.clone())?;
                        self.last_tick_key_events.clear();
                    }
                    _ => {
                        self.last_tick_key_events.push(key);

                        if let Some(action) = keymap.get(&self.last_tick_key_events) {
                            debug!("Got action: {action:?}");
                            self.action_tx.send(action.clone())?;
                            self.last_tick_key_events.clear();
                        }
                    }
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

            match action.clone() {
                Action::Tick if self.last_tick_key_events.len() > 1 => {
                    self.last_tick_key_events.clear();
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => self.tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(w, h)?,
                Action::Render => self.component_menager.draw(&mut self.tui)?,
                Action::EnterSearch => {
                    self.mode = Mode::Search;
                }
                Action::SearchResponse(_) => {
                    self.mode = Mode::Normal;
                }
                Action::SearchRequest(phrase) => {
                    self.spawn_search(phrase);
                }
                _ => {}
            }

            self.component_menager.handle_actions(action)?;
        }

        Ok(())
    }

    fn spawn_search(&self, phrase: String) {
        let source = Arc::clone(&self.source);
        let tx = self.action_tx.clone();
        tokio::task::spawn_blocking(move || {
            let result =
                search(source.data(), &phrase).unwrap_or_else(|| SearchResult::empty(phrase));
            let _ = tx.send(Action::SearchResponse(result));
        });
    }

    fn handle_resize(&mut self, w: u16, h: u16) -> color_eyre::Result<()> {
        self.tui.resize(Rect::new(0, 0, w, h))?;
        self.component_menager.handle_resize(Size::new(w, h))?;
        self.component_menager.draw(&mut self.tui)?;
        Ok(())
    }
}
