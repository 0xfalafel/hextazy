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
	enable_raw_mode()?;
	let mut stderr = io::stderr(); // this is a special case. Normally using stdout is fine
	execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

	let backend = CrosstermBackend::new(stderr);
	let mut terminal = Terminal::new(backend)?;
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
				_ => {}
			}
		}
	}

	// restore terminal
	disable_raw_mode()?;
	execute!(
		terminal.backend_mut(),
		LeaveAlternateScreen,
		DisableMouseCapture
	)?;
	terminal.show_cursor()?;

	Ok(())
}