#![allow(unused)]

use std::{collections::btree_map::Values, error::Error, io, process::exit};

use app::{CommandBar, CurrentEditor};
use crossterm::{
	event::{
		self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers
	},
	execute,
	terminal::{
		disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
		LeaveAlternateScreen,
	},
	cursor
};
use ratatui::{
	backend::{Backend, CrosstermBackend},
	Terminal,
};

// mod app;
mod ui;
mod app;
mod search;

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

			// shortcuts with Ctrl + key
			match (key) {

				// shortcuts to quit the app
				// Ctrl + Q, Ctrl + C
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
				} => {app.change_cursor(0x7)},
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Left,  ..
				} => {app.change_cursor(-0x7)},

				// Shift + N: go to previous search result
				KeyEvent {
					modifiers: KeyModifiers::SHIFT,
					code: KeyCode::Char('n') | KeyCode::Char('N')
					, ..
				} => {app.go_to_previous_search_result()},

				// Ctrl + Y: redo()
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Char('y'), ..
				} => {app.redo(); continue;}

				// Ctrl + Z: undo()
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Char('z'),  ..
				} => {app.undo(); continue;},
				_ => {}
			}

			match key.code {

				// Move the cursor
				KeyCode::Down => {
					// if we are on the last line, also move the screen down
					let current_line = (app.cursor - (app.offset * 2)) / 32;

					if current_line == (app.lines_displayed-1).into() {
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
					match (app.editor_mode) {
						CurrentEditor::HexEditor   => {app.change_cursor(1)},
						CurrentEditor::AsciiEditor => {app.change_cursor(2)},
						_ => {}
					};
				},
				KeyCode::Left => {
					match (app.editor_mode) {
						CurrentEditor::HexEditor   => {app.change_cursor(-1)},
						CurrentEditor::AsciiEditor => {app.change_cursor(-2)},
						_ => {}
					};
				},
				KeyCode::Backspace => {
					match (app.editor_mode) {
						CurrentEditor::HexEditor   => {app.change_cursor(-1)}
						CurrentEditor::AsciiEditor => {app.change_cursor(-2)}
						
						// remove the last char. If command is empty, switch to Hex editor
						CurrentEditor::CommandBar  => {
							if let Some(ref mut command_bar) = app.command_bar {
								command_bar.command.pop();

								if command_bar.command.len() == 0 {
									app.command_bar = None;
									app.editor_mode = CurrentEditor::HexEditor;
								}
							};
						}
					};
				},

				// Type ascii : edit the file & shortcuts
				KeyCode::Char(key) => {
					// exit the app on 'q' in Hex mode
					if (app.editor_mode == CurrentEditor::HexEditor && key == 'q') {
						break;
					}

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
					
					// ':' Open Command bar
					} else if app.editor_mode == CurrentEditor::HexEditor && key == ':' {
						app.command_bar = Some(CommandBar {
							command: String::from(":"),
							cursor: 1
						});

						app.editor_mode = CurrentEditor::CommandBar;

					// '/' Open command bar with search
					} else if app.editor_mode == CurrentEditor::HexEditor && key == '/' {
						app.command_bar = Some(CommandBar {
							command: String::from("/"),
							cursor: 1
						});

						app.editor_mode = CurrentEditor::CommandBar;

					// go to the next search result
					} else if app.editor_mode == CurrentEditor::HexEditor && key == 'n'{
						app.go_to_next_search_result();

					// Ascii Editor
					} else if app.editor_mode == CurrentEditor::AsciiEditor
						&& key.is_ascii() {
							// convert key pressed to u8 A -> 0x41
							let value: u8 = key as u8;
	
							app.write_ascii(app.cursor, value);
							app.change_cursor(2);
					
					// Command Bar
					} else if app.editor_mode == CurrentEditor::CommandBar {

						// add the key pressed to the command typed
						if let Some(cmd_text) = &mut app.command_bar {
							cmd_text.command.push(key);
						}
					}
				},

				// Esc: quit the command bar or the Ascii mode
				KeyCode::Esc => {
					if app.editor_mode != CurrentEditor::HexEditor {
						app.command_bar = None;
						app.editor_mode = CurrentEditor::HexEditor;
					}
				}

				// interpret the command, and close the command bar
				KeyCode::Enter => {
					if app.editor_mode == CurrentEditor::CommandBar {
						app.interpret_command();
					}
					app.command_bar = None;
					app.editor_mode = CurrentEditor::HexEditor;
				}

				// Jump by a whole screen
				KeyCode::PageDown => {
					// we jump a whole screen
					let offset_to_jump = (app.lines_displayed-1) * 0x10;
					// convert to i64
					let offset_to_jump: i64 = offset_to_jump.try_into().unwrap();

					// update the cursor, so that it stay on the same line
					app.change_cursor(offset_to_jump*2 + 0x20); // 0x20 is needed to stay on the same line
					app.change_offset(offset_to_jump)
				},
				KeyCode::PageUp => {
					let offset_to_jump = (app.lines_displayed-1) * 0x10;
					// convert to i64
					let offset_to_jump: i64 = offset_to_jump.try_into().unwrap();

					// update the cursor, so that it stay on the same line
					app.change_cursor(-offset_to_jump*2 - 0x20); // 0x20 is needed to stay on the same line
					app.change_offset(-offset_to_jump)
				},

				// Go to start of binary, stay on the same column
				KeyCode::Home => {
					// use the to stay on the same 'char' of the hex character
					let cursor_on_second_char = app.cursor % 2;

					app.jump_to(app.cursor / 2 % 0x10);

					app.cursor = app.cursor + cursor_on_second_char;
				},
				
				// Go to end of binary, stay on the same column
				KeyCode::End => {
					let size_of_last_line = app.file_size % 0x10;
					let column_of_cursor = app.cursor / 2 % 0x10;

					// use the to stay on the same 'char' of the hex character
					let cursor_on_second_char = app.cursor % 2;

					// we go on the last line
					if column_of_cursor < size_of_last_line {
						app.jump_to(
							app.file_size - size_of_last_line + column_of_cursor
						);
					}

					// we go on the line just before the last one
					else {
						app.jump_to(
							app.file_size - size_of_last_line - 0x10 + column_of_cursor
						);
					}

					app.cursor = app.cursor + cursor_on_second_char;
				},

				// switch between Hex and Ascii editor
				KeyCode::Tab => { 
					match (app.editor_mode) {
						CurrentEditor::HexEditor => {app.editor_mode = CurrentEditor::AsciiEditor},
						CurrentEditor::AsciiEditor =>{app.editor_mode = CurrentEditor::HexEditor},
						_ => {}
					};
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

    Ok(terminal)
}

/// Resets the terminal.
fn reset_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture, cursor::Show)?;

    Ok(())
}