#![allow(unused)]

use std::{collections::btree_map::Values, error::Error, io, process::exit};

use app::CurrentEditor;
use crossterm::{
	event::{
		self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers
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

fn usage() {
	println!("Usage: {} [file]", std::env::args().nth(0)
		.expect("Error: argv[0] don't exist"));
}

fn main() -> Result<(), Box<dyn Error>> {
	let file_argument = std::env::args().nth(1); //.expect("no file given");

	let filename = match file_argument {
		Some(filename) => {filename},
		None => {println!("No file given \n"); usage(); exit(0);}
	};

	// setup terminal
	let mut terminal = init_terminal()?;

	// panic hook
	// restore the terminal before panicking.
	let original_hook = std::panic::take_hook();

	std::panic::set_hook(Box::new(move |panic| {
		reset_terminal().unwrap();
		original_hook(panic);
	}));

	let mut app = App::new(String::from(filename))?;

	loop {
		app.reset();

		// draw the screen
		terminal.draw(|f| ui(f, &mut app))?;

		if let Event::Key(key) = event::read()? {
			
			// Skip events that are not KeyEventKind::Press
			if key.kind == event::KeyEventKind::Release {
				continue;
			}

			match (key) {
				// shortcuts to quit the app
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
						code: KeyCode::Char('q'), ..
					} => {break},
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
						code: KeyCode::Char('c'), ..
					} => {break},

				// Ctrl + direction: jump by 8 chars
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Right,  ..
				} => {app.change_cursor(0xf)},
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Left,  ..
				} => {app.change_cursor(-0xf)},
				_ => {}
			}

			match key.code {
				KeyCode::Char('q') => {
					if (app.editor_mode == CurrentEditor::AsciiEditor) {
						continue;
					} else {
						break;
					}
				},

				// for testing purposes
				// KeyCode::Char('j') => {
				// 	app.cursor = app.cursor + 0x20;
				// },
				// KeyCode::Char('m') => {
				// 	app.offset = app.offset + 0x10;
				// },
				// KeyCode::Char('k') => {
				// 	if app.file_size % 0x10 == 0 {
				// 		app.offset = app.file_size - 0x10;						
				// 	} else {
				// 		app.offset = app.file_size - (app.file_size % 0x10);
				// 	}
				// },
				KeyCode::Down => {
					// if we are on the last line, also move the screen down
					let current_line = (app.cursor - (app.offset * 2)) / 32;

					if current_line == app.lines_displayed-1 {
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
				KeyCode::Char(key) => {
					// Hex editor
					if app.editor_mode == CurrentEditor::HexEditor
						&&  key.is_ascii_hexdigit() {
							// convert key pressed to u8 f -> 15
							let value: u8 = key.to_digit(16)
								.unwrap()
								.try_into()
								.unwrap();
	
							app.write(app.cursor, value);
							app.change_cursor(1);
					
					// Ascii Editor
					} else if app.editor_mode == CurrentEditor::AsciiEditor
						&& key.is_ascii() {
							// convert key pressed to u8 A -> 0x41
							let value: u8 = key as u8;
	
							app.write_ascii(app.cursor, value);
							app.change_cursor(2);
					}
				},
				KeyCode::PageDown => {
					// we jump a whole screen
					let offset_to_jump = (app.lines_displayed-1) * 0x10;
					// convert to i64
					let offset_to_jump: i64 = offset_to_jump.try_into().unwrap();

					// update the cursor, so that it stay on the same line
					app.change_cursor(offset_to_jump*2);
					app.change_offset(offset_to_jump)
				},
				KeyCode::PageUp => {
					let offset_to_jump = (app.lines_displayed-1) * 0x10;
					// convert to i64
					let offset_to_jump: i64 = offset_to_jump.try_into().unwrap();

					// update the cursor, so that it stay on the same line
					app.change_cursor(-offset_to_jump*2);
					app.change_offset(-offset_to_jump)
				},
				KeyCode::Tab => { 
					// switch between Hex and Ascii editor
					if app.editor_mode == CurrentEditor::HexEditor {
						app.editor_mode = CurrentEditor::AsciiEditor
					} else {
						app.editor_mode = CurrentEditor::HexEditor
					}
				}
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

    Ok(terminal)
}

/// Resets the terminal.
fn reset_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}