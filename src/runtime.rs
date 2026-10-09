use std::{
    io,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crate::{
    app::{self, Effect, JobMessage, Model},
    domain::log_file::Indexing,
    platform::terminal::{Event, Terminal},
    ui,
};

const JOB_POLL_TIMEOUT: Duration = Duration::from_millis(50);

pub fn run(terminal: &mut Terminal, mut model: Model) -> io::Result<()> {
    let (job_sender, job_receiver) = mpsc::channel();
    app::spawn_indexer(job_sender, model.log_file.indexer());
    app::resize_log_view(&mut model, terminal.size()?);
    draw(terminal, &model)?;

    loop {
        match handle_waiting_events(terminal, &mut model, &job_receiver)? {
            Effect::Nothing => {}
            Effect::Redraw => draw(terminal, &model)?,
            Effect::Suspend => {
                terminal.suspend()?;
                draw(terminal, &model)?;
            }
            Effect::Quit => return Ok(()),
        }
    }
}

/// Handles the next event and every event already waiting behind it, so keys that arrive faster
/// than the screen is drawn (a held-down key) cost one draw instead of a backlog of draws. While a
/// job runs, waiting for a key times out, so that job messages are handled in between.
fn handle_waiting_events(
    terminal: &Terminal,
    model: &mut Model,
    job_receiver: &Receiver<JobMessage>,
) -> io::Result<Effect> {
    let input_timeout = has_running_jobs(model).then_some(JOB_POLL_TIMEOUT);
    let mut batch_effect = Effect::Nothing;

    loop {
        let effect = terminal
            .next_event(input_timeout)?
            .map_or(Effect::Nothing, |event| handle_event(model, event));
        match effect {
            Effect::Nothing => {}
            Effect::Redraw => batch_effect = Effect::Redraw,
            Effect::Suspend | Effect::Quit => return Ok(effect),
        }
        for job_message in job_receiver.try_iter() {
            if app::handle_job_message(model, job_message) == Effect::Redraw {
                batch_effect = Effect::Redraw;
            }
        }
        if !terminal.has_waiting_event()? {
            return Ok(batch_effect);
        }
    }
}

fn handle_event(model: &mut Model, event: Event) -> Effect {
    match event {
        Event::Key(key) => app::handle_key(model, key),
        Event::Resize(screen_size) => app::resize_log_view(model, screen_size),
    }
}

fn has_running_jobs(model: &Model) -> bool {
    model.log_file.indexing() == Indexing::Running
}

fn draw(terminal: &mut Terminal, model: &Model) -> io::Result<()> {
    terminal.draw(|frame| ui::render(frame, model))
}
