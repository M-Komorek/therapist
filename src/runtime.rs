use std::{
    io,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crate::{
    app::{self, Effect, Model},
    jobs::JobNotification,
    platform::terminal::{Terminal, TerminalEvent},
    ui,
};

const INPUT_TIMEOUT_WHILE_JOBS_RUN: Duration = Duration::from_millis(50);

pub fn run(terminal: &mut Terminal, mut model: Model) -> io::Result<()> {
    let (job_sender, job_receiver) = mpsc::channel();
    app::resize_log_view(&mut model, terminal.size()?);
    draw(terminal, &model)?;

    loop {
        model.job_board.spawn_scheduled_jobs(&job_sender);
        match handle_next_batch(terminal, &mut model, &job_receiver)? {
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

fn handle_next_batch(
    terminal: &Terminal,
    model: &mut Model,
    job_receiver: &Receiver<JobNotification>,
) -> io::Result<Effect> {
    let input_timeout = has_running_jobs(model).then_some(INPUT_TIMEOUT_WHILE_JOBS_RUN);
    let mut batch_effect = Effect::Nothing;

    loop {
        let effect = terminal
            .next_event(input_timeout)?
            .map_or(Effect::Nothing, |terminal_event| {
                handle_terminal_event(model, terminal_event)
            });
        match effect {
            Effect::Nothing => {}
            Effect::Redraw => batch_effect = Effect::Redraw,
            Effect::Suspend | Effect::Quit => return Ok(effect),
        }
        for job_notification in job_receiver.try_iter() {
            if app::handle_job_notification(model, job_notification) == Effect::Redraw {
                batch_effect = Effect::Redraw;
            }
        }
        if !terminal.has_waiting_event()? {
            return Ok(batch_effect);
        }
    }
}

fn handle_terminal_event(model: &mut Model, terminal_event: TerminalEvent) -> Effect {
    match terminal_event {
        TerminalEvent::Key(key) => app::handle_key(model, key),
        TerminalEvent::Resize(screen_size) => app::resize_log_view(model, screen_size),
    }
}

fn has_running_jobs(model: &Model) -> bool {
    !model.job_board.is_empty()
}

fn draw(terminal: &mut Terminal, model: &Model) -> io::Result<()> {
    terminal.draw(|frame| ui::render(frame, model))
}
