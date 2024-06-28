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
					// if we are on the last line, also move the screen down
					let current_line = (app.cursor - (app.offset * 2)) / 32;

					if current_line == app.lines_displayed-3 {
						app.change_offset(0x10)
					}

					// move the cursor down
					app.change_cursor(0x20)
				},
				KeyCode::Up => {
					// don't change cursor if we are on the last line of the file
					if app.cursor < 0x1f {
						continue;
					}

					// if we are on the first line, also move the screen up
					if (app.cursor - app.offset*2) / 32 == 0 {
						app.change_offset(-0x10);
					}
					
					app.change_cursor(-0x20);				
				},
				KeyCode::Right => {
					app.change_cursor(1);
				},
				KeyCode::Left => {
					app.change_cursor(-1);
				},
				KeyCode::Backspace => {
					app.change_cursor(-1);
				},
				KeyCode::Char(key) if key.is_ascii_hexdigit() => {
					// convert key pressed to u8 f -> 15
					let value: u8 = key.to_digit(16)
						.unwrap()
						.try_into()
						.unwrap();

					app.write(app.cursor, value);
					app.change_cursor(1);
				},
				KeyCode::PageDown => {
					// we jump a whole screen
					let offset_to_jump = (app.lines_displayed-3) * 0x10;
					// convert to i64
					let offset_to_jump: i64 = offset_to_jump.try_into().unwrap();

					// update the cursor, so that it stay on the same line
					app.change_cursor(offset_to_jump*2);
					app.change_offset(offset_to_jump)
				},
				KeyCode::PageUp => {
					let offset_to_jump = (app.lines_displayed-3) * 0x10;
					// convert to i64
					let offset_to_jump: i64 = offset_to_jump.try_into().unwrap();

					// update the cursor, so that it stay on the same line
					app.change_cursor(-offset_to_jump*2);
					app.change_offset(-offset_to_jump)				},
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