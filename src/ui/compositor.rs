use ratatui::layout::Size;
use tokio::sync::mpsc;

use crate::{
    action::Action,
    config::Config,
    tui::{Event, Tui},
    ui::components::contract::Component,
    ui::layout::{LayoutManager, LayoutSlot},
};

pub struct Compositor {
    components: Vec<(LayoutSlot, Box<dyn Component>)>,
    layout_manager: LayoutManager,
    action_tx: mpsc::UnboundedSender<Action>,
}

impl Compositor {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>, layout_manager: LayoutManager) -> Self {
        Compositor {
            components: Vec::new(),
            layout_manager,
            action_tx,
        }
    }

    pub fn register_component(
        &mut self,
        component: impl Component + 'static,
        layout_slot: LayoutSlot,
        config: &Config,
    ) -> color_eyre::Result<()> {
        let mut component: Box<dyn Component> = Box::new(component);
        component.register_action_handler(self.action_tx.clone())?;
        component.register_config_handler(config.clone())?;
        component.init(self.layout_manager.get_area(layout_slot).as_size())?;
        self.components.push((layout_slot, component));
        Ok(())
    }

    pub fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<()> {
        for (_, component) in self.components.iter_mut() {
            if let Some(action) = component.handle_events(event.clone())? {
                self.action_tx.send(action)?;
            }
        }
        Ok(())
    }

    pub fn handle_actions(&mut self, action: Action) -> color_eyre::Result<()> {
        for (_, component) in self.components.iter_mut() {
            if let Some(action) = component.update(action.clone())? {
                self.action_tx.send(action)?;
            }
        }
        Ok(())
    }

    pub fn handle_resize(&mut self, size: Size) -> color_eyre::Result<()> {
        self.layout_manager.compute(size);
        Ok(())
    }

    pub fn draw(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame: &mut ratatui::Frame<'_>| {
            for (slot, component) in self.components.iter_mut() {
                let area = self.layout_manager.get_area(*slot);
                if let Err(err) = component.draw(frame, area) {
                    let _ = self
                        .action_tx
                        .send(Action::Error(format!("Failed to draw: {:?}", err)));
                }
            }
        })?;
        Ok(())
    }
}
