use std::{i128, u128};

use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Style, Stylize},
	symbols, text::{Line, Span, Text},
	widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
	Frame
};
use crate::{app::{Braille, CurrentEditor, Mode, WarningLevel}, App};
mod braille;
use crate::ui::braille::BRAILLE_CHARSET;

pub fn ui(f: &mut Frame, app: &mut App) { //, app: &App) {
	
	let contraints = match app.selection_start.is_some() {
		true => [
			Constraint::Max(9),
			Constraint::Length(53),
			Constraint::Length(18),
			Constraint::Length(33) // We have a preview pane
		],
		false => [
			Constraint::Max(9),
			Constraint::Length(53),
			Constraint::Length(18),
			Constraint::Length(0) // No preview pane
		]
	};

	let panes = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(contraints)
		.split(f.area());

	// update the number of lines displayed by the app.
	// we use this for shortcuts.
	// -2 because we don't need the 2 lines of border
	app.lines_displayed = (panes[1].height - 2).into();

	/* Adress Block */
	render_address_block(app, panes[0], f);

	/* Hex Block */
	render_hex_block(app, panes[1], f);
	
	/* Create ASCII Block */
	render_ascii_block(app, panes[2], f);

	/* Render the preview Block if some bytes are selected */
	if app.selection_start.is_some() {
		render_preview_block(app, panes[3], f);
	}
	

	// Display command bar (only if it exists)
	if app.editor_mode == CurrentEditor::CommandBar {

		render_command_bar(
			app.command_bar.clone().unwrap().command,
			Style::default().bg(Color::DarkGray),
			f
		);
	}

	// Display error message (if we have one)
	if let Some((warning_level, message)) = &app.error_msg {
		let error_style = match warning_level {
			WarningLevel::Info => {
				Style::default()
					.bg(Color::Blue)
					.fg(Color::Black)
					.bold()
			},
			WarningLevel::Warning => {
				Style::default()
					.bg(Color::Yellow)
					.fg(Color::DarkGray)
					.bold()
			},
			WarningLevel::Error => {
				Style::default()
					.bg(Color::Red)
					.bold()
			}
		};

		render_command_bar(
			message.clone(),
			error_style,
			f
		);
	}

	if app.editor_mode == CurrentEditor::ExitPopup {
		exit_popup(f);
	}

}

/// Render the address pane on the left
fn render_address_block(app: &App, pane: Rect, f: &mut Frame) {
	// top & bottom right corner must render the top & bottom left to join with the left block
	let borders_address_block = symbols::border::Set {
		top_right: symbols::line::NORMAL.horizontal_down,
		bottom_right: symbols::line::NORMAL.horizontal_up,
		..symbols::border::PLAIN
	};

	// Create the address block
	let address_block = Block::default()
		.border_set(borders_address_block) // make borders continous for the corners
		.borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)
		.style(Style::default());	

	// Create a list of address
	let mut list_items = Vec::<ListItem>::new();

	let start_address = app.offset;
	let height: u64 = pane.height as u64;
	let remaining_file_size = app.length_to_end();

	// don't write addresses after the last line
	let mut end_address = match remaining_file_size < height * 16 {
		true  => start_address + remaining_file_size,
		false => start_address + height*16
	};

	if app.mode == Mode::Insert {
		end_address += 1;
	}

	for i in (start_address..end_address).step_by(16) {
		list_items.push(
			ListItem::new(Line::from(
				Span::styled(format!("{:08x}", i),
				Style::default().fg(Color::Indexed(242)))
			)
		));
	}

	// add list to block, and render block
	let list = List::new(list_items).block(address_block);
	f.render_widget(list, pane);
}

/// Render the `main panel` with the hex values
fn render_hex_block(app: &mut App, pane: Rect, f: &mut Frame) {

	/* Create the Box and borders for the pane */

	// Display the position of the cusror on the
	// bottom of the hex block
	let bottom_line = Line::from(
		vec![
			format!(" 0x{:x}", app.cursor / 2).bold(),
			format!(" /{:x}", app.file_size).into(),
			" ─ ".bold(),
			format!("{} ", app.filename()).light_blue(),
		]
	);

	// We need to set the corners, to have continuous borders
	let hexblock_borders = symbols::border::Set {
		top_left: symbols::line::NORMAL.horizontal_down,
		bottom_left: symbols::line::NORMAL.horizontal_up,
		top_right: symbols::line::NORMAL.horizontal_down,
		bottom_right: symbols::line::NORMAL.horizontal_up,
		..symbols::border::PLAIN
	};

	/* Create Hex Block */
	let hex_block = Block::default()
		.border_set(hexblock_borders) // make borders continous for the corners
		.borders(Borders::ALL)
		.style(Style::default());

	// Display the infobar depending of the `app.show_infobar` setting
	let hex_block = match app.show_infobar {
		true => {
			hex_block
				.title_bottom(bottom_line)
				.title_alignment(ratatui::layout::Alignment::Left)
		},
		false => {
			hex_block
				.title_top("┬")
				.title_bottom("┴")
				.title_alignment(ratatui::layout::Alignment::Center)
		}
	};


	/* Render the bytes in hexadecimal */

	let mut hex_lines: Vec<Line> = vec![];
	let focused = app.editor_mode != CurrentEditor::AsciiEditor;

	// Render every line of the Hex pane
	for l in 0..app.lines_displayed {

		// We use this to build a line of hex chars
		let mut line: Vec<Span> = vec![];

		// Render a line of the Hex pane
		for i in 0..0x10 {

			// Colorize space between hex chars if the previous and next chars are selected
			// otherwise, just push a " " without background color
			match app.selection_start {
				Some(_selected_byte) => {
					let current_byte: u64 = app.offset + u64::from(l) * 0x10 + i;
					
					if i == 0 || i == 8 { // don't colorize the initial, and the separator
						line.push(Span::raw(" "));
					}
					// colorize the space between 2 bytes
					else if app.is_selected(current_byte.saturating_sub(1)) && app.is_selected(current_byte) {
						line.push(Span::raw(" ").bg(Color::Indexed(238)));
					}

					else { // We have some bytes selected, but this is not the selected bytes
						line.push(Span::raw(" "))
					}
				} // There are no bytes selected
				_ => line.push(Span::raw(" "))
			};
			
			let byte = app.read_byte();
			let byte_addr = app.last_address_read - 1;

			match byte {
				// We are the cursor, after the end of the file
				None if app.cursor / 2 == byte_addr => {
					let style = Style::default().fg(Color::White);
					let style_focused = style.bg(Color::DarkGray);

					match focused {
						true  => line.push(Span::styled("_", style_focused)),
						false => line.push(Span::styled("_", style))
					};
					line.push(Span::raw(" "));
				},

				// We have reach EOF, pad with some empty spaces
				None => line.push(Span::raw("  ")),

				// We have a byte to display
				Some(val) => {
					// Is this the byte with the cursor ?
					match app.cursor / 2 == byte_addr {

						// It's not the cursor
						false => {
							// Hightlight the byte if it is selected
							let style = match app.is_selected(byte_addr) {
								false => colorize(val),
								true => colorize(val).bg(Color::Indexed(238)), // Selection background color
							};

							line.push(Span::styled(
								format!("{:02x}", val),
								style
							))
						},

						// We have the cursor
						true => {	
							/* Prepare the styling of the cursor */

							// Background of the cursor
							let cursor_background = match focused {
								true if val == 0x00 => Color::White,
								true  => get_color(val),
								false => Color::Black,
							};

							// Color of the char highlighted by the cursor
							let cursor_char_color = match focused {
								true  => {Color::Black}
								false => {Color::White},
							};

							// Mix thoses in a style
							let cursor_style: Style = Style::default()
								.fg(cursor_char_color)
								.bg(cursor_background);

							// Style byte that is not highlighted by the cursor
							let other_style = match app.is_selected_cursor(app.cursor) {
								false => colorize(val),
								true => colorize(val).bg(Color::Indexed(238)), // Selection background color
							};

							/* Apply the style of the cursor to the corresponding char */

							let (style_char1, style_char2) = match app.cursor % 2 == 0 {
	
								// if the ascii pane is focused, we hightlight both chars corresponding to the
								// byte selected with the cursor on the ascii pane
								_ if !focused => (cursor_style, cursor_style),

								// cursor is on the first char
								true  => (cursor_style, other_style),
								// cursor is on the second char
								false => (other_style, cursor_style)
							};


							/* Finally add this to the UI */

							// Get the 2 chars of the cursor
							let hex_val = format!("{:02x?}", val);
							let hex_char1 = hex_val.chars().nth(0).unwrap();
							let hex_char2 = hex_val.chars().nth(1).unwrap();

							line.push(Span::styled(hex_char1.to_string(), style_char1));
							line.push(Span::styled(hex_char2.to_string(), style_char2));
						}
					}
				}
			}

			// add the stylish ┊ in the middle, color changes in hexyl mode
			if i == 7 {
				let separator_style = match app.show_infobar {
					false => Style::default(),
					true => Style::default().fg(Color::DarkGray),
				};
				line.push(Span::styled(" ┊", separator_style));
			}
		}

		hex_lines.push(Line::from(line));
	}
		
	let text = Text::from(hex_lines);
	let paragraph = Paragraph::new(text).block(hex_block);
	f.render_widget(paragraph, pane);

	// restore the position of the `cursor` reading the file
	app.reset();
}

fn render_ascii_block(app: &mut App, pane: Rect, f: &mut Frame) {
	// Style the borders
	let ascii_block = Block::default()
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default())
		.title_alignment(ratatui::layout::Alignment::Center);


	// If we have a pane on the right (preview), we need to change the corners
	// to make the borders look continuous

	let ascii_block = match app.selection_start.is_some() {
		true  => ascii_block.border_set(symbols::border::Set { // top right and bottom right corners
			top_right: symbols::line::NORMAL.horizontal_down,  // ┬ and ┴ symbols
			bottom_right: symbols::line::NORMAL.horizontal_up,
			..symbols::border::PLAIN
		}),
		false => ascii_block
	};

	// show which mode we are using
	let mode = match app.mode {
		Mode::Overwrite => { "overwrite ".yellow().bold() },
		Mode::Insert => { "insert ".green().bold() },
	};

	let ascii_infobar = Line::from(
		vec![" mode: ".into(), mode]);

	// Display the infobar depending of the `app.show_infobar` setting
	let ascii_block = match app.show_infobar {
		true  => { ascii_block.title_bottom(ascii_infobar) },
		false => {
			ascii_block
				.title("┬")
				.title_bottom("┴")
		}
	};

	let mut ascii_lines: Vec<Line> = vec![];


	/*
		Read either the number of lines displayed by the interface
		or to the end of the file.
		Depending of what is the lowest (don't read the whole file if
		it isn't needed).
	*/

	for i in 0..app.lines_displayed {

		// Convert the bytes to an array.
		// We might want to change this in the future.
		// This is because the app use to read 16 bytes into an array. And all the function
		// were build using an array.
		let (content, len) = app.read_16_length();
		let mut buf: [u8; 16] = [0; 16];

		for i in 0..len {
			buf[i] = content[i];
		}

		// if this is the line with the cursor
		if (app.cursor.saturating_sub(app.offset * 2)) / 32 == i.into() {
			let line_cursor = app.cursor % 32;


			// ascii line
			let ascii_line = render_ascii_line_with_cusor(
				buf, (line_cursor / 2).try_into().unwrap(), len,
				app.editor_mode == CurrentEditor::AsciiEditor,
				!app.show_infobar,
				app.braille
			);
			ascii_lines.push(ascii_line);			
		}

		else {	
			// ascii line
			let ascii_line = render_ascii_line(buf, len, !app.show_infobar, app.braille);
			ascii_lines.push(ascii_line);
		}		
	}

	let ascii_text = Text::from(ascii_lines);
	let ascii_paragraph = Paragraph::new(ascii_text).block(ascii_block);
	f.render_widget(ascii_paragraph, pane);


}

/// Render the preview pane on the left
fn render_preview_block(app: &mut App, pane: Rect, f: &mut Frame) {

	// Create the preview block
	let preview_block = Block::default()
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default())
		.title_alignment(ratatui::layout::Alignment::Center);

	let selected_bytes = match app.get_selected_bytes() {
		Some(bytes) => bytes,
		None => {
			app.add_error_message(WarningLevel::Info, String::from("Error: we should have some selected bytes"));
			return;
		}
	};

	let hex_string: String = selected_bytes
		.iter()
		.map(|byte| format!("{:02x}", byte))
		.collect();
	let number_of_bytes = selected_bytes.len();

	// We will use this to render every line of our preview pane
	let mut lines: Vec<Line> = vec![];

	// print the bytes
	let header = format!("Bytes ({} - 0x{:x}):\n", number_of_bytes, number_of_bytes);
	let bytes_header = Line::from(header.blue().bold());
	let bytes = Line::from(format!("{}", hex_string));

	lines.push(bytes_header);
	lines.push(bytes);


	if number_of_bytes <= 16 {

		/*
			Little Endian
		*/
		
		// Unsigned Integer

		let le_header = Line::from(format!("\n\nLittle Endian:\n ({} bits)", number_of_bytes * 8).red().bold());
		lines.push(Line::from(""));
		lines.push(le_header);
		
		let mut conversion_buffer: [u8; 16] = [0; 16];
		for (i, byte) in selected_bytes.iter().enumerate() {
			conversion_buffer[i] = *byte;
		}

		let little_endian_val = u128::from_le_bytes(conversion_buffer);

		let name = match number_of_bytes {
			len if len <=  1 => "u8",
			len if len <=  2 => "u16",
			len if len <=  4 => "u32",
			len if len <=  8 => "u64",
			len if len <= 16 => "u128",
			_ => return
		};

		// Display the lines
		// unsigned int
		let le_line: Line<'_> = Line::from(
			format!("{}: ", name).blue().bold() +
			format!("{}", little_endian_val).into()
		);
		lines.push(Line::from(""));
		lines.push(le_line);


		// Signed Integer

		// Convert 
		let (name, signed_val) = match number_of_bytes {
			len if len <= 1 => {
				let buf: [u8; 1] = conversion_buffer[0..1].try_into().unwrap();
				("i8", i8::from_le_bytes(buf) as i128)
			},
			len if len <= 2 => {
				let buf: [u8; 2] = conversion_buffer[0..2].try_into().unwrap();
				("i16", i16::from_le_bytes(buf) as i128)
			},
			len if len <= 4 => {
				let buf: [u8; 4] = conversion_buffer[0..4].try_into().unwrap();
				("i32", i32::from_le_bytes(buf) as i128)
			},
			len if len <= 8 => {
				let buf: [u8; 8] = conversion_buffer[0..8].try_into().unwrap();
				("i64", i64::from_le_bytes(buf) as i128)
			},
			len if len <= 16 => {
				let buf: [u8; 16] = conversion_buffer[0..16].try_into().unwrap();
				("i128", i128::from_le_bytes(buf) as i128)
			},
			_ => return
		};

		// Display the lines
		let le_signed_line: Line<'_> = Line::from(
			format!("{}: ", name).blue().bold() +
			format!("{}", signed_val).into()
		);
		lines.push(le_signed_line);

		// Hexadecimal
		let hex_le_line = Line::from(
			"hex: ".blue().bold() +
			format!("0x{:x}", little_endian_val).into()
		);
		lines.push(Line::from(""));
		lines.push(hex_le_line);
		
		/*
			Big Endian
		*/
		
		// Unsigned Integer

		let be_header = Line::from(format!("\n\nBig Endian:\n ({} bits)", number_of_bytes * 8).red().bold());
		lines.push(Line::from(""));
		lines.push(be_header);
		
		let mut conversion_buffer: [u8; 16] = [0; 16];
		for (i, byte) in selected_bytes.iter().enumerate() {
			conversion_buffer[i] = *byte;
		}

		let big_endian_val = u128::from_be_bytes(conversion_buffer);

		let name = match number_of_bytes {
			len if len <=  1 => "u8",
			len if len <=  2 => "u16",
			len if len <=  4 => "u32",
			len if len <=  8 => "u64",
			len if len <= 16 => "u128",
			_ => return
		};

		// Display the lines
		// unsigned int
		let be_line: Line<'_> = Line::from(
			format!("{}: ", name).blue().bold() +
			format!("{}", big_endian_val).into()
		);
		lines.push(Line::from(""));
		lines.push(be_line);


		// Signed Integer

		// Convert 
		let (name, signed_val) = match number_of_bytes {
			len if len <= 1 => {
				let buf: [u8; 1] = conversion_buffer[0..1].try_into().unwrap();
				("i8", i8::from_be_bytes(buf) as i128)
			},
			len if len <= 2 => {
				let buf: [u8; 2] = conversion_buffer[0..2].try_into().unwrap();
				("i16", i16::from_be_bytes(buf) as i128)
			},
			len if len <= 4 => {
				let buf: [u8; 4] = conversion_buffer[0..4].try_into().unwrap();
				("i32", i32::from_be_bytes(buf) as i128)
			},
			len if len <= 8 => {
				let buf: [u8; 8] = conversion_buffer[0..8].try_into().unwrap();
				("i64", i64::from_be_bytes(buf) as i128)
			},
			len if len <= 16 => {
				let buf: [u8; 16] = conversion_buffer[0..16].try_into().unwrap();
				("i128", i128::from_be_bytes(buf) as i128)
			},
			_ => return
		};

		// Display the lines
		let be_signed_line: Line<'_> = Line::from(
			format!("{}: ", name).blue().bold() +
			format!("{}", signed_val).into()
		);
		lines.push(be_signed_line);

		// Hexadecimal
		let hex_be_line = Line::from(
			"hex: ".blue().bold() +
			format!("0x{:x}", big_endian_val).into()
		);
		lines.push(Line::from(""));
		lines.push(hex_be_line);
	}
	
	let text = Text::from(lines);
	let paragraph = Paragraph::new(text)
	.block(preview_block)
	.wrap(Wrap {trim: true});

	f.render_widget(paragraph, pane);
}

/// Display the command bar or an error message, as one line at the end of the UI.
/// This function exists to reduce code duplication.
fn render_command_bar(text: String, style: Style, f: &mut Frame) {
	let area = f.area();
		
	let width = if area.width < 80 {
		area.width - 2
	} else {
		78
	};

	// display the commandline 1 line before the end
	let command_layout = Rect {
		width: width,
		height: 1,
		x: 1,
		y: area.height-2
	};

	let cmdline_popup = Block::default()
		.borders(Borders::NONE)
		.style(style);

	let command_text = Paragraph::new(text)
		.block(cmdline_popup);

	f.render_widget(Clear, command_layout);
	f.render_widget(command_text, command_layout);
}


/// Used for the ascii pane
/// Take a buffer of u8[16] and render it with a colorize ascii line
fn render_ascii_line(buf: [u8; 16], len: usize, hexyl_style: bool, braille: Braille) -> Line<'static> {
	let mut ascii_colorized: Vec<Span> = vec![];

	for i in 0..16 {
		if i < len {
			ascii_colorized.push(
				render_ascii_char(buf[i], braille)
			);
		} else {
			ascii_colorized.push(Span::raw(" "));
		}

		if i == 7 {
			let separator_style = match hexyl_style {
				true  => {Style::default()},
				false => {Style::default().fg(Color::DarkGray)},
			};
			ascii_colorized.push(Span::styled("┊", separator_style));
		}
	}
	Line::from(ascii_colorized)
}

fn render_ascii_line_with_cusor(buf: [u8; 16], cursor: usize, len: usize, focused: bool, hexyl_style: bool, braille: Braille) -> Line<'static> {
	let mut ascii_colorized: Vec<Span> = vec![];

	for i in 0..16 {
		if i < len { // display at most `len` chars
						
			if i == cursor { // highlight the cursor

				let mut style = match focused {
					true => Style::default()
						.fg(Color::Black)
						.bg(get_color(buf[i])),
					false => Style::default().fg(Color::White)
				};

				if focused && buf[i] == 0x00 {
					style = style.bg(Color::White);
				}

				let colorized = Span::styled(
					render_ascii_char(buf[i], braille).to_string(),
					style
				);			

				ascii_colorized.push(colorized);

			} else {
				ascii_colorized.push(render_ascii_char(buf[i], braille));
			}
		}
	
		// We are the cursor, after the end of the file
		else if i == cursor {
			let style = Style::default().fg(Color::White);
			let style_focused = style.bg(Color::DarkGray);

			match focused {
				true  => ascii_colorized.push(Span::styled("_", style_focused)),
				false => ascii_colorized.push(Span::styled("_", style))
			};
		}

		// if we don't have any data to write, push blank chars
		else {
			ascii_colorized.push(Span::raw(" "));
		}
		
		if i == 7 { // stylish ┊ in the middle
			let separator_style = match hexyl_style {
				true  => {Style::default()},
				false => {Style::default().fg(Color::DarkGray)},
			};
			ascii_colorized.push(Span::styled("┊", separator_style));
		}
	}
	Line::from(ascii_colorized)
}


/// Used for the ascii pane.
/// Take a u8, and render a colorized ascii, or placeholdler
fn render_ascii_char(val: u8, braille: Braille) -> Span<'static> {
	let ascii_char = match braille {
		Braille::None  => ascii_char(val),
		Braille::Full  => braille_char(val),
		Braille::Mixed => mixed_braille(val)
	};

	Span::styled(
		ascii_char.to_string(),
		get_color(val)
	)
}

// Used for the ascii pane.

/// Take a u8, return an ascii char, or placeholdler
fn ascii_char(val: u8) -> char {
	match val {
		val if val == 0x00 => {'0'},
		val if val == 0x20 => {' '},
		val if val.is_ascii_whitespace() => {'_'},
		val if val > 0x20 && val < 0x7f => {val as char},
		val if val.is_ascii() => {'•'},
		_val => {'x'} // non printable ascii
	}
}

/// Take a u8, return an ascii char from the braille_charset
fn braille_char(val: u8) -> char {
	BRAILLE_CHARSET[val as usize]
}

/// Take a u8, return classic chars for value bellow 0x80, and a Braille ascii for other values
/// It's a pretty Ok compromise in readability
fn mixed_braille(val: u8) -> char {
	match val {
		val if val == 0x00 => {'0'},
		val if val == 0x20 => {' '},
		val if val.is_ascii_whitespace() => {'_'},
		val if val > 0x20 && val < 0x7f => {val as char},
		val if val.is_ascii() => {'•'},
		val => {braille_char(val)} // 0x80 and above
	}
}

/// Return a style that match the val
/// i.e Light Cyan for ASCII values
fn colorize(val: u8) -> Style {
	Style::default().fg(get_color(val))
}

fn get_color(val: u8) -> Color {
	match val {
		val if val == 0x00 => {
			Color::Indexed(242)
		},
		val if val.is_ascii_whitespace() => {
			Color::Green
		},
		val if val > 0x20 && val < 0x7f => {
			Color::Cyan
		},
		val if val.is_ascii() => {
			Color::Magenta
		},
		_val => {
			Color::Yellow
		}
	}
}

fn exit_popup(f: &mut Frame) {
	let area = f.area();

	// take up a third of the screen vertically and half horizontally
	let popup_area = Rect {
		x: area.width / 4,
		y: area.height / 3,
		width: area.width / 2,
		// height = 6, but don't crash if the window is too small
		height: if area.height > 6 {6} else {area.height - 2}, 
	};

	let text = Text::from(vec![
		Line::from("This file has some unsaved modifications."),
		Line::from(""),
		Line::from("Do you want to save your changes ?").bold().centered(),
		Line::from("Yes (y) / No(n)").bold().centered().red(),
	]);

	let popup = Paragraph::new(text)
		.wrap(Wrap { trim: true })
		.style(Style::new())
		.block(
			Block::new()
				.title("Exiting")
				.title_style(Style::new().white().bold())
				.borders(Borders::ALL)
				.border_style(Style::new().red()),
		);
	
	f.render_widget(Clear, popup_area); //this clears the entire screen and anything already drawn
	f.render_widget(popup, popup_area);
}