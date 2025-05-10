use std::{error::Error, io, process::exit};
use colored::Colorize;
use clap::Parser;

use app::{Braille, CommandBar, CurrentEditor};
use crossterm::{
	cursor, event::{
		self, DisableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers
	}, terminal::{
		disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen
	}
};
use ratatui::{
	backend::CrosstermBackend, Terminal
};

// mod app;
mod ui;
mod app;
mod search;

use crate::{
    app::{App, Mode},
	ui::ui,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Name of the file to open
    #[arg(value_parser)]
    file: String,

	// Braille mode
    #[arg(short, long, action = clap::ArgAction::SetTrue, help = "Display Ascii in braille dump")]
	braille: bool,

	// Mixed braille mode
    #[arg(short = 'B', action = clap::ArgAction::SetTrue, help = "Disable the display Ascii in braille dump for 0x80 and above")]
	braille_mixed: bool,

	// Seek to defined byte
	#[arg(short, long, help = "Go to this address. I.e `-s 0xc0ffee`")]
	seek: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
	
	/* handle arguments and flags */

	let args = Args::parse();
	
	// Parse braille flags which define how the ascii pane will be display.
	// Full has priority over mixed, default is None
	let braille_mode = match (args.braille, args.braille_mixed) {
		(braille, _) if braille => Braille::Full, // -b is set
		(_, mixed_braille) if mixed_braille => Braille::None, // -B is set
		(_, _) => Braille::Mixed,
	};

	// Parse --seek parameter
	let seek = match args.seek {
		Some(seek) => parse_seek(&seek),
		None => None,
	};

	let app = App::new(String::from(args.file), braille_mode, seek)?;

	/* Some ratatui code to handle panic!() without messing up the terminal */

	// setup terminal
	let mut terminal = init_terminal()?;
	
	// panic hook
	// restore the terminal before panicking.
	let original_hook = std::panic::take_hook();
	
	std::panic::set_hook(Box::new(move |panic| {
		reset_terminal().expect("Failed to reset the terminal. Use the `reset` command in your terminal.");
		original_hook(panic);
	}));
	
	/* Main loop, handle keyboards event like shortcuts */
	handle_keyboard_inputs(app, &mut terminal)?;

	// restore terminal
	reset_terminal().expect("Failed to reset the terminal. Use the `reset` command in your terminal.");

	Ok(())
}

// Code for handling terminal copied from https://ratatui.rs/examples/apps/panic/

/// Initializes the terminal.
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(io::stdout());

    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

/// Resets the terminal.
fn reset_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture, cursor::Show)?;

    Ok(())
}

/// Check if the input String match an Integer of an Hex value.
/// Return Some(value) if it matches, None otherwise
fn parse_seek(input: &str) -> Option<u64> {
	// Try to parse a int value, i.e "1024"
	if let Ok(val) = u64::from_str_radix(input, 10) {
		return Some(val);
	}

	// Try to parse an hex value "0xc0ffee" or "c0ffee". The inital 0x is optionnal 
	if let Ok(val) = u64::from_str_radix(input.trim_start_matches("0x"), 16) {
		return Some(val);
	}

	None
}

fn handle_keyboard_inputs(mut app: App, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn Error>> {
	loop {
		app.reset();

		// draw the screen
		terminal.draw(|f| ui(f, &mut app))?;

		if let Event::Key(key) = event::read()? {
			
			// Skip events that are not KeyEventKind::Press
			if key.kind == event::KeyEventKind::Release {
				continue;
			}

			// Any action of the user will cleanup any
			// error message.
			app.cleanup_error_message();

			// shortcuts with Ctrl + key
			match key {

				// shortcuts to quit the app
				// Ctrl + Q, Ctrl + C
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
						code: KeyCode::Char('q'), ..
					} => {
						// if we have no changes exit, else show the exit popup
						if app.modified_bytes.is_empty() {
							break Ok(());
						} else {
							app.editor_mode = CurrentEditor::ExitPopup;
						}
					},

				// Ctrl + J: Switch between Insert and Overwrite
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
						code: KeyCode::Char('j') | KeyCode::Char('J'), ..
					} => {
						match app.mode {
							Mode::Insert => {
								app.mode = Mode::Overwrite;
								// Make sure our cursor isn't after the end of the file
								// in Overwrite mode
								if app.cursor >= app.file_size * 2 {
									app.cursor = (app.file_size * 2).saturating_sub(1);
								}
							},
							Mode::Overwrite => app.mode = Mode::Insert,
						};
						continue;
					},				
				
				// Ctrl + C: exit without saving
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
						code: KeyCode::Char('c'), ..
					} => {break Ok(())},

				// Ctrl + Left / Right: jump by 4 bytes
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Right,  ..
				} => {app.change_cursor(0x7)},

				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Left,  ..
				} => {app.change_cursor(-0x7)},

				// Ctrl + Up / Down: jump by 4 lines
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Up,  ..
				} => {app.change_cursor(-0x40)},

				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Down,  ..
				} => {app.change_cursor(0x40)},

				// Alt + Left / Right: move selection by 1 bytes
				KeyEvent {
					modifiers: KeyModifiers::ALT,
					code: KeyCode::Right,  ..
				} => {app.move_selection(0x2); continue;},

				KeyEvent {
					modifiers: KeyModifiers::ALT,
					code: KeyCode::Left,  ..
				} => {app.move_selection(-0x2); continue;},

				// Alt + Up / Down: move selection by 1 line
				KeyEvent {
					modifiers: KeyModifiers::ALT,
					code: KeyCode::Down,  ..
				} => {app.move_selection(0x20); continue;},

				KeyEvent {
					modifiers: KeyModifiers::ALT,
					code: KeyCode::Up,  ..
				} => {app.move_selection(-0x20); continue;},


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

				// Ctrl + S: save the changes
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Char('s'),  ..
				} => {
					// if no changes were made, do nothing.					
					if ! app.modified_bytes.is_empty() {
						// save the changes, and give feedback to the user
						match app.save_to_disk() {
							Ok(()) => {
								app.add_error_message(app::WarningLevel::Info,
									"Changes successfully saved.".to_string());
							},
							Err(e) => {
								app.add_error_message(app::WarningLevel::Error,
									format!("Failed to save the changes: {}.", e));
							}
						}
						continue;
					}
				},

				// Ctrl + U: undo_all()
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Char('u'),  ..
				} => {app.undo_all(); continue;},

				// Ctrl + space: select the current character
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Char(' '),  ..
				} => {
					app.selection_start = Some(app.cursor);
					continue;
				},

				// Go to start of binary, stay on the same column
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::Home,  ..
				} => {
					// use the to stay on the same 'char' of the hex character
					let cursor_on_second_char = app.cursor % 2;

					app.jump_to(app.cursor / 2 % 0x10);

					app.cursor = app.cursor + cursor_on_second_char;
					continue;
				},

				// Go to end of binary, stay on the same column
				KeyEvent {
					modifiers: KeyModifiers::CONTROL,
					code: KeyCode::End,  ..
				} => {
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
					continue;
				},

				_ => {}
			}

			match key.code {

				// Move the cursor
				KeyCode::Down => {
					// if we are on the last line, also move the screen down
					let current_line = (app.cursor.saturating_sub(app.offset * 2)) / 32;

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
					if (app.cursor.saturating_sub(app.offset*2)) / 32 == 0 {
						app.change_offset(-0x10);
					}
					
					app.change_cursor(-0x20);				
				},
				KeyCode::Right => {
					match app.editor_mode {
						CurrentEditor::HexEditor   => {app.change_cursor(1)},
						CurrentEditor::AsciiEditor => {app.change_cursor(2)},
						_ => {}
					};
				},
				KeyCode::Left => {
					match app.editor_mode {
						CurrentEditor::HexEditor   => {app.change_cursor(-1)},
						CurrentEditor::AsciiEditor => {app.change_cursor(-2)},
						_ => {}
					};
				},
				KeyCode::Backspace => {
					match (app.mode, app.editor_mode) {

						// Commandbar: remove the last char. If command is empty, switch to Hex editor
						(_, CurrentEditor::CommandBar) => {
							if let Some(ref mut command_bar) = app.command_bar {
								command_bar.command.pop();

								if command_bar.command.len() == 0 {
									app.command_bar = None;
									app.editor_mode = CurrentEditor::HexEditor;
								}
							};
						},

						// undo the previous change and move the cursor left
						(Mode::Overwrite, pane) => {
							
							// if the previous char is the last modified, undo() instead of 
							// just moving the cursor left
							if let Some((_, addr_last_change, _)) = app.history.last() {
								if *addr_last_change == (app.cursor - 1) / 2 {
									app.undo();
									continue;
								}
							}

							match pane {
								CurrentEditor::AsciiEditor => app.change_cursor(-2),
								CurrentEditor::HexEditor  => app.change_cursor(-1),
								_ => {continue;} // don't handle the other cases
							}
						},

						// Delete the previous byte
						(Mode::Insert, _) => {

							match app.cursor % 2 {
								// Delete the previous byte 
								0 => {
									if app.cursor > 1 {
										app.delete_byte((app.cursor / 2)-1);
										app.cursor = app.cursor - 2;
									}
								},
								// If we are in the middle byte, delete the current byte
								1 => {
									app.delete_byte(app.cursor / 2);
									app.change_cursor(-1);
								},
								2_u64..=u64::MAX => unreachable!("app.cursor % 2 is always 1 or 2")
							}
						}
					}
				},

				KeyCode::Delete => {
					if app.mode == Mode::Insert {
						app.delete_byte(app.cursor / 2);
					}
				},

				// Type ascii : edit the file & shortcuts
				KeyCode::Char(key) => {
					// exit the app on 'q' in Hex mode
					if app.editor_mode == CurrentEditor::HexEditor && key == 'q' {
						// if we don't have any changes, exit. Else show the exit popup
						if app.modified_bytes.is_empty() {
							break Ok(());
						} else {
							app.editor_mode = CurrentEditor::ExitPopup;
						}
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
					
					// 'v' Start the selection
					} else if app.editor_mode == CurrentEditor::HexEditor && key == 'v' {
						app.selection_start = Some(app.cursor);
						continue;
	
					// ':' Open Command bar
					} else if app.editor_mode == CurrentEditor::HexEditor && key == ':' {
						app.command_bar = Some(CommandBar {
							command: String::from(":"),
							_cursor: 1
						});

						app.editor_mode = CurrentEditor::CommandBar;

					// '/' Open command bar with search
					} else if app.editor_mode == CurrentEditor::HexEditor && key == '/' {
						app.command_bar = Some(CommandBar {
							command: String::from("/"),
							_cursor: 1
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
					
					// Exit popup
					} else if app.editor_mode == CurrentEditor::ExitPopup {
						if key == 'y' {
							if app.save_to_disk().is_err() {
								reset_terminal()?;
								let error_msg = format!("Failed to save the changes on {}", app.file_path.bold());
								eprintln!("{}", error_msg.red());
								exit(1);
							}
							break Ok(());
						} else if key == 'n' {
							break Ok(());
						}
					}
				},

				// Esc: quit the command bar or the Ascii mode
				// exit selection if defined
				KeyCode::Esc => {
					// quit command bar
					if app.editor_mode != CurrentEditor::HexEditor {
						app.command_bar = None;
						app.editor_mode = CurrentEditor::HexEditor;
					}

					// Exit selection if defined
					if app.selection_start.is_some() {
						app.selection_start = None;
					}
				}

				// interpret the command, and close the command bar
				KeyCode::Enter => {
					if app.editor_mode == CurrentEditor::CommandBar {
						app.interpret_command();
						app.command_bar = None;
						app.editor_mode = CurrentEditor::HexEditor;
					}
				}

				// Jump by a whole screen
				KeyCode::PageDown => {
					// we jump by a whole screen
					let offset_to_jump = (app.lines_displayed-1) * 0x10;
					// convert to i64
					let offset_to_jump: i64 = offset_to_jump.try_into().unwrap();
					
					app.change_offset(offset_to_jump);
					app.change_cursor(offset_to_jump*2);
				},

				// Jump by a whole screen
				KeyCode::PageUp => {
					let offset_to_jump = (app.lines_displayed-1) * 0x10;
					// convert to i64
					let offset_to_jump: i64 = offset_to_jump.try_into().unwrap();
					
					app.change_offset(-offset_to_jump);
					app.change_cursor(-offset_to_jump*2);
				},

				// Go to start of the line
				KeyCode::Home => {
					app.cursor_jump_to(app.cursor - (app.cursor % 0x20));
				},
				
				// Go to end of the line
				KeyCode::End => {
					app.cursor_jump_to(app.cursor - (app.cursor % 0x20) + 0x1f);
				},

				// switch between Hex and Ascii editor
				KeyCode::Tab => { 
					match app.editor_mode {
						CurrentEditor::HexEditor   => {app.editor_mode = CurrentEditor::AsciiEditor},
						CurrentEditor::AsciiEditor => {app.editor_mode = CurrentEditor::HexEditor},
						_ => {}
					};
				},
				_ => {}
			}
		}
	}
}