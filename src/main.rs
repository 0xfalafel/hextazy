#![allow(unused)]

use std::{error::Error, io};

use crossterm::{
	event::{
		self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode,
		KeyEventKind,
	},
	execute,
	terminal::{
		disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
		LeaveAlternateScreen,
	},
};
use ratatui::{
	backend::{Backend, CrosstermBackend},
	Terminal,
};

// mod app;
mod ui;
mod app;

use crate::{
    app::App,
	ui::ui,
};


fn main() -> Result<(), Box<dyn Error>> {
	let file = std::env::args().nth(1).expect("no file given");

	// setup terminal
	let mut terminal = init_terminal()?;

	// panic hook
	// restore the terminal before panicking.
	let original_hook = std::panic::take_hook();

	std::panic::set_hook(Box::new(move |panic| {
		reset_terminal().unwrap();
		original_hook(panic);
	}));

	let mut app = App::new(String::from(file))?;

	loop {
		app.reset();

		// draw the screen
		terminal.draw(|f| ui(f, &mut app))?;

		if let Event::Key(key) = event::read()? {
			if key.kind == event::KeyEventKind::Release {
				// Skip events that are not KeyEventKind::Press
				continue;
			}

			match key.code {
				KeyCode::Char('q') => {
					break;
				},
				KeyCode::Down => {
					app.change_cursor(0x20)
				},
				KeyCode::Up => {
					// don't change cursor if we are on the last line
					if app.cursor > 0x1f {
						app.change_cursor(-0x20);				
					}
				},
				KeyCode::Right => {
					app.change_cursor(1);
				},
				KeyCode::Left => {
					app.change_cursor(-1);
				},
				KeyCode::Char('0') => {
					app.write(app.cursor, 0);
					app.change_cursor(1);
				}
				KeyCode::PageDown => {
					app.change_offset(0x100)
				},
				KeyCode::PageUp => {
					app.change_offset(-0x100)
				},
				_ => {}
			}
		}
	}

	// restore terminal
	reset_terminal()?;

	Ok(())
}

// Code for handling terminal copied from https://ratatui.rs/examples/apps/panic/

/// Initializes the terminal.
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(io::stdout());

    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    Ok(terminal)
}

/// Resets the terminal.
fn reset_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}