use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Style, Stylize, Modifier},
	symbols, text::{Line, Span, Text},
	widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
	Frame
};
use crate::{app::{CurrentEditor, WarningLevel, Mode}, App};

pub fn ui(f: &mut Frame, app: &mut App) { //, app: &App) {
	
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([
			Constraint::Max(9),
			Constraint::Length(53),
			Constraint::Length(18)
		])
		.split(f.area());

	/* Adress Block */
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
	let height: u64 = chunks[0].height as u64;
	let remaining_file_size = app.length_to_end();

	// don't write addresses after the last line

	let end_address = if remaining_file_size < height * 16 {
		start_address + remaining_file_size
	} else {
		start_address + height*16
	};

	for i in (start_address..end_address).step_by(16) {
		list_items.push(
			ListItem::new(Line::from(
				Span::styled(format!("{:08x}", i),
				Style::default().fg(Color::DarkGray))
			)
		));
	}

	// add list to block, and render block
	let list = List::new(list_items).block(address_block);
	f.render_widget(list, chunks[0]);

	let bottom_line = Line::from(
		vec![
			format!(" 0x{:x}", app.cursor / 2).bold(),
			format!(" /{:x}", app.file_size).into(),
			" ─ ".bold(),
			format!("{} ", app.filename()).light_blue(),
		]
	);

	/* Create Hex Block */
	
	// We need to set the corners, to have continuous borders
	let hexblock_borders = symbols::border::Set {
		top_left: symbols::line::NORMAL.horizontal_down,
		bottom_left: symbols::line::NORMAL.horizontal_up,
		top_right: symbols::line::NORMAL.horizontal_down,
		bottom_right: symbols::line::NORMAL.horizontal_up,
		..symbols::border::PLAIN
	};

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

	let mut hex_lines: Vec<Line> = vec![];

	/* Create ASCII Block */
	let ascii_block = Block::default()
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default())
		.title_alignment(ratatui::layout::Alignment::Center);

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

	// update the number of lines displayed by the app.
	// we use this for shortcuts.
	// -2 because we don't need the 2 lines of border
	app.lines_displayed = (chunks[1].height - 2).into();

	/*
		Read either the number of lines displayed by the interface
		or to the end of the file.
		Depending of what is the lowest (don't read the whole file if
		it isn't needed).
	*/

	let lines_to_end: u64 = chunks[1].height.into();

	/*  ******************************************
		 Render every line, and fufill the blocks
		******************************************	*/

	for i in 0..lines_to_end {

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
		if (app.cursor - app.offset * 2) / 32 == i {
			let line_cursor = app.cursor % 32;

			// hex line
			let hex_line = render_hex_line_with_cursor(buf, line_cursor.try_into().unwrap(), len,
			app.editor_mode != CurrentEditor::AsciiEditor,
			app.show_infobar == false); // don't change the cusor style when the command bas is open
			hex_lines.push(hex_line);

			// ascii line
			let ascii_line = render_ascii_line_with_cusor(
				buf, (line_cursor / 2).try_into().unwrap(), len,
				app.editor_mode == CurrentEditor::AsciiEditor,
				!app.show_infobar		
			);
			ascii_lines.push(ascii_line);			
		}

		else {
			// hex line
			let hex_line = render_hex_line(buf, len, app.show_infobar==false);
			hex_lines.push(hex_line);
	
			// ascii line
			let ascii_line = render_ascii_line(buf, len, !app.show_infobar);
			ascii_lines.push(ascii_line);
		}		
	}

	let text = Text::from(hex_lines);
	let paragraph = Paragraph::new(text).block(hex_block);
	f.render_widget(paragraph, chunks[1]);

	let ascii_text = Text::from(ascii_lines);
	let ascii_paragraph = Paragraph::new(ascii_text).block(ascii_block);
	f.render_widget(ascii_paragraph, chunks[2]);

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


/// Take a buffer of u8[16] and render it with a colorize hex line.
/// It will render at most `len` u8, so we can have that nice end line.
fn render_hex_line(buf: [u8; 16], len: usize, hexyl_style: bool) -> Line<'static> {
	let mut hex_chars: Vec<Span> = vec![];

	for i in 0..16 {
		if i < len { // display at most `len` chars
			hex_chars.push(Span::styled(
					format!(" {:02x}", buf[i]),
					colorize(buf[i])
				));
		} else { // add whitespace when we don't have any more values
			hex_chars.push(Span::raw("   "));
		}
			
		// add the stylish ┊ in the middle
		if i == 7 {
			let separator_style = match hexyl_style {
				true  => {Style::default()},
				false => {Style::default().fg(Color::DarkGray)},
			};
			hex_chars.push(Span::styled(" ┊", separator_style));
		}
	}

	Line::from(hex_chars)
}

/// Take a buffer of u8[16] and render it with a colorize hex line
/// highlight the character with a cursor.
/// Display at most `len` chars
/// `focused` if the cursor is editing this pane. Otherwise the cursor is on the ascii pane
fn render_hex_line_with_cursor(buf: [u8; 16], cursor: usize, len: usize, focused: bool, hexyl_style: bool) -> Line<'static> {
	let mut hex_chars: Vec<Span> = vec![];

	for i in 0..16 {
		// 
		let val: u8 = buf[i];

		if i < len { // we have data to write
		
			//we look at the character that has the cursor
			if cursor / 2 == i {
				
				hex_chars.push(Span::raw(" "));
				
				let hex_val = format!("{:02x?}", val);
				let hex_char1 = hex_val.chars().nth(0).unwrap();
				let hex_char2 = hex_val.chars().nth(1).unwrap();

				// Catchy background if the cusor is focused
				let cursor_backgound = match focused {
					false => {Color::DarkGray}
					true => {
						if val == 0x00 { // So we don't have the same background for the focused cursor
							Color::Gray
						} else {
							get_color(val)
						}
					},
				};

				// Color of the char highlighted by the cursor
				let mut cursor_char_color = match focused {
					false => {get_color(val)},
					true  => {Color::Black}
				};
				
				// otherwise for 0x00, the background and char have the same color
				if cursor_char_color == Color::DarkGray {
					cursor_char_color = Color::Black;
				}

				match focused {

					// If the hex view is focused. We highlight only the
					// hex char that has the cursor
					true => {
						// highlight the first of the two hex character
						if cursor % 2 == 0 {
							let style: Style = Style::default()
								.fg(cursor_char_color)
								.bg(cursor_backgound);

							hex_chars.push(
								Span::styled(
									format!("{}", hex_char1),
									style
								));
							
							hex_chars.push(
								Span::styled(
									format!("{}", hex_char2),
									colorize(val)
								));

								
						// highlight the second of the two hex character
						} else {
							let style: Style = Style::default()
								.fg(cursor_char_color)
								.bg(cursor_backgound);
							
							hex_chars.push(
								Span::styled(
									format!("{}", hex_char1),
									colorize(val)
								));
							hex_chars.push(
								Span::styled(
									format!("{}", hex_char2),
									style
								));
						}
	
					},

					// If the Ascii pane is focused. We highlight the
					// whole byte corresponding to the selected ascii char.
					false => {
						let style: Style = Style::default()
							.fg(Color::White)
							.bg(Color::Black);

						hex_chars.push(
							Span::styled(
								format!("{}", hex_val),
								style
							));
					},
				}
				
			// that's a character without the cusor
			} else {
				let colorized_hex_char = Span::styled(
					format!(" {:02x}", val),
					colorize(val)
				);
				hex_chars.push(colorized_hex_char);
			}
		}
		
		// We are the cursor, after the end of the file
		else if cursor / 2 == i {
			hex_chars.push(Span::raw(" "));
			let style = Style::default().bg(Color::DarkGray);

			match focused {
				true  => hex_chars.push(Span::styled("_", style)),
				false => hex_chars.push(Span::styled(" ", style))
			};
			hex_chars.push(Span::raw(" "));
		}

		// if we don't have data, put blank chars to write the '┊' correctly
		else {
			hex_chars.push(Span::raw("   "));
		}
			
		// add the stylish ┊ in the middle
		if i == 7 {
			let separator_style = match hexyl_style {
				true  => {Style::default()},
				false => {Style::default().fg(Color::DarkGray)},
			};
			hex_chars.push(Span::styled(" ┊", separator_style));
		}
	}

	Line::from(hex_chars)
}

/// Used for the ascii pane
/// Take a buffer of u8[16] and render it with a colorize ascii line
fn render_ascii_line(buf: [u8; 16], len: usize, hexyl_style: bool) -> Line<'static> {
	let mut ascii_colorized: Vec<Span> = vec![];

	for i in 0..16 {
		if i < len {
			ascii_colorized.push(
				render_ascii_char(buf[i])
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

fn render_ascii_line_with_cusor(buf: [u8; 16], cursor: usize, len: usize, focused: bool, hexyl_style: bool) -> Line<'static> {
	let mut ascii_colorized: Vec<Span> = vec![];

	for i in 0..16 {
		if i < len { // display at most `len` chars
						
			if i == cursor { // highlight the cursor

				let style = match focused {
					true => Style::default()
						.fg(Color::Black)
						.bg(get_color(buf[i])),
					false => Style::default().fg(Color::White)
				};

				let colorized = Span::styled(
					ascii_char(buf[i]).to_string(),
					style
				);			

				ascii_colorized.push(colorized);

			} else {
				ascii_colorized.push(render_ascii_char(buf[i]));
			}
		}
	
		// We are the cursor, after the end of the file
		else if i == cursor {
			let style = Style::default().bg(Color::DarkGray);

			match focused {
				true  => ascii_colorized.push(Span::styled("_", style)),
				false => ascii_colorized.push(Span::styled(" ", style))
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
fn render_ascii_char(val: u8) -> Span<'static> {
	Span::styled(
		ascii_char(val).to_string(),
		get_color(val)
	)
}

/// Used for the ascii pane.
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

/// Return a style that match the val
/// i.e Light Cyan for ASCII values
fn colorize(val: u8) -> Style {
	Style::default().fg(get_color(val))
}

fn get_color(val: u8) -> Color {
	match val {
		val if val == 0x00 => {
			Color::DarkGray
		},
		val if val.is_ascii_whitespace() => {
			Color::Green
		},
		val if val > 0x20 && val < 0x7f => {
			Color::LightCyan
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