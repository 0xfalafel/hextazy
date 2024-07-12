#![allow(unused)]

use crossterm::style;
use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Style, Stylize},
	text::{Line, Span, Text},
	widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
	Frame,
	symbols
};
use crate::{app::CurrentEditor, App};

pub fn ui(f: &mut Frame, app: &mut App) { //, app: &App) {
	
	// top & bottom right corner must render the top & bottom left to join with the left block
	let top_bottom_right_corner = symbols::border::Set {
		top_right: symbols::line::NORMAL.horizontal_down,
		bottom_right: symbols::line::NORMAL.horizontal_up,
		..symbols::border::PLAIN
	};

	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([
			Constraint::Length(10),
			Constraint::Length(52),
			Constraint::Length(18)
		])
		.split(f.size());

	/* Adress Block */
	// Create the address block
	let address_block = Block::default()
		.border_set(top_bottom_right_corner) // make borders continous for the corners
		.borders(Borders::ALL)
		.style(Style::default());

	// Create a list of address
	let mut list_items = Vec::<ListItem>::new();

	let start_address = app.offset;
	let size = chunks[0].height as u64;
	let end_address = start_address + size * 16;

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

	/* Create Hex Block */
	let hex_block = Block::default()
		.border_set(top_bottom_right_corner) // make borders continous for the corners
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default());
	
	let mut hex_lines: Vec<Line> = vec![];

	/* Create ASCII Block */
	let ascii_block = Block::default()
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default());

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

	let remaining_file_size = app.length_to_end();
	let lines_to_end: u64 = chunks[1].height.into();

	let mut lines_to_read = remaining_file_size / 16;

	if (remaining_file_size % 16) != 0 {
		lines_to_read = lines_to_read + 1;
	}

	if lines_to_end < lines_to_read {
		lines_to_read = lines_to_end;
	}


	/*  ******************************************
		 Render every line, and fufill the blocks
		******************************************	*/

	for i in 0..lines_to_read {
		let buf;
		let len: usize;
		
		(buf, len) = app.read_16_length();

		// if this is the line with the cursor
		if (app.cursor - app.offset * 2) / 32 == i {
			let line_cursor = app.cursor % 32;

			// hex line
			let hex_line = render_hex_line_with_cursor(buf, line_cursor.try_into().unwrap(), len, app.editor_mode == CurrentEditor::HexEditor);
			hex_lines.push(hex_line);

			// ascii line
			let ascii_line = render_ascii_line_with_cusor(buf, (line_cursor / 2).try_into().unwrap(), len, app.editor_mode == CurrentEditor::AsciiEditor);
			ascii_lines.push(ascii_line);			
		}

		else {
			// hex line
			let hex_line = render_hex_line(buf, len);
			hex_lines.push(hex_line);
	
			// ascii line
			let ascii_line = render_ascii_line(buf, len);
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
		// used to display the commandline 1 line before the end
		let command_layout = Layout::vertical([
			Constraint::Min(1),
			Constraint::Length(5),
			Constraint::Length(1)
		]).split(f.size());

		let cmdline_popup = Block::default()
			.borders(Borders::NONE)
			.style(Style::default().bg(Color::DarkGray)
		);

		let command_text = Paragraph::new(":")
			.block(cmdline_popup.clone());

		f.render_widget(cmdline_popup, command_layout[1]);
	}
}

/// Take a buffer of u8[16] and render it with a colorize hex line.
/// It will render at most `len` u8, so we can have that nice end line.
fn render_hex_line(buf: [u8; 16], len: usize) -> Line<'static> {
	let mut hex_chars: Vec<Span> = vec![];

	for i in 0..16 {
		if (i >= len) { // display at most `len` chars
			break;
		}

		hex_chars.push(
			Span::styled(
				format!(" {:02x}", buf[i]),
				colorize(buf[i])
			)
		);

		// add the stylish ┊ in the middle
		if (i == 7) {
			hex_chars.push(
				Span::styled(" ┊",
					Style::default().fg(Color::White)
			));
		}
	}

	Line::from(hex_chars)
}

/// Take a buffer of u8[16] and render it with a colorize hex line
/// highlight the character with a cursor.
/// Display at most `len` chars
/// `focused` if the cursor is editing this pane. Otherwise the cursor is on the ascii pane
fn render_hex_line_with_cursor(buf: [u8; 16], cursor: usize, len: usize, focused: bool) -> Line<'static> {
	let mut hex_chars: Vec<Span> = vec![];

	for i in 0..16 {
		if i >= len{ // we have displayed `len` chars
			break;
		}

		//we look at the character that has the cursor
		if cursor / 2 == i {

			hex_chars.push(Span::raw(" "));

			let hex_val = format!("{:02x?}", buf[i]);
			let hex_char1 = hex_val.chars().nth(0).unwrap();
			let hex_char2 = hex_val.chars().nth(1).unwrap();

			// Catchy background if the cusor is focused
			let cursor_backgound = match (focused) {
				true => {Color::White},
				false => {Color::DarkGray}
			};
			
			// highlight the first of second hex character
			if cursor % 2 == 0 {
				let mut style_cursor = colorize(buf[i]);
				// Make cursor value readable on DarkGray Background
				if style_cursor == Style::default().fg(Color::DarkGray) {
					style_cursor = Style::default().fg(Color::Black);
				}
				
				hex_chars.push(Span::styled(format!("{}", hex_char1), style_cursor.bg(cursor_backgound)));
				hex_chars.push(Span::styled(format!("{}", hex_char2), colorize(buf[i])));
			} else {
				let mut style_cursor = colorize(buf[i]);
				// Make cursor value readable on DarkGray Background
				if style_cursor == Style::default().fg(Color::DarkGray) {
					style_cursor = Style::default().fg(Color::Black);
				}

				hex_chars.push(Span::styled(format!("{}", hex_char1), colorize(buf[i])));
				hex_chars.push(Span::styled(format!("{}", hex_char2), style_cursor.bg(cursor_backgound)));
			}
			
		// that's a character without the cusor
		} else {
			let mut colorized_hex_char = Span::styled(
				format!(" {:02x}", buf[i]),
				colorize(buf[i])
			);
			hex_chars.push(colorized_hex_char);
		}


		// add the stylish ┊ in the middle
		if (i == 7) {
			hex_chars.push(
				Span::styled(" ┊",
					Style::default().fg(Color::White)
			));
		}
	}

	Line::from(hex_chars)
}

/// Used for the ascii pane
/// Take a buffer of u8[16] and render it with a colorize ascii line
fn render_ascii_line(buf: [u8; 16], len: usize) -> Line<'static> {
	let mut ascii_colorized: Vec<Span> = vec![];

	for i in 0..16 {
		if i >= len {
			break;
		}

		ascii_colorized.push(
			render_ascii_char(buf[i])
		);

		if i == 7 {
			ascii_colorized.push(
				Span::styled("┊",
				Style::default().fg(Color::White)
			));
		}
	}
	Line::from(ascii_colorized)
}

fn render_ascii_line_with_cusor(buf: [u8; 16], cursor: u8, len: usize, focused: bool) -> Line<'static> {
	let mut ascii_colorized: Vec<Span> = vec![];
	let mut colorized_ascii_char: Span;

	for i in 0..16 {
		if i >= len { // display at most `len` chars
			break;
		}

		colorized_ascii_char = render_ascii_char(buf[i]);

		if i as u8 == cursor { // highlight the cursor
			colorized_ascii_char = match (focused) {
				true => {colorized_ascii_char.bg(Color::White)},
				false => {colorized_ascii_char.bg(Color::DarkGray)}
			};

			if buf[i] == 0x00 { // Make '0' readable on DarkGray background
				colorized_ascii_char = colorized_ascii_char.fg(Color::Black);
			}
		}

		ascii_colorized.push(
			colorized_ascii_char
		);

		if i == 7 { // stylish ┊ in the middle
			ascii_colorized.push(
				Span::styled("┊",
				Style::default().fg(Color::White)
			));
		}
	}
	Line::from(ascii_colorized)
}


/// Used for the ascii pane.
/// Take a u8, and render a colorized ascii, or placeholdler
fn render_ascii_char(val: u8) -> Span<'static> {
	match val {
		val if val == 0x00 => {
			Span::styled(
				"0",
				Style::default().fg(Color::DarkGray)
			)
		},
		val if val == 0x20 => {
			Span::styled(
				" ",
				Style::default().fg(Color::Green)
			)
		},
		val if val.is_ascii_whitespace() => {
			Span::styled(
				"_",
				Style::default().fg(Color::Green)
			)
		},
		val if val > 0x20 && val < 0x7f => {
			Span::styled(
				format!("{}" , val as char),
				Style::default().fg(Color::LightCyan)
			)
		},
		val if val.is_ascii() => {
			Span::styled(
				"•",
				Style::default().fg(Color::Magenta)
			)
		},
		val => {
			Span::styled(
				"x",
				Style::default().fg(Color::Yellow)
			)
		}
	}
}

/// Return a style that match the val
/// i.e Light Cyan for ASCII values
fn colorize(val: u8) -> Style {
	match val {
		val if val == 0x00 => {
			Style::default().fg(Color::DarkGray)
		},
		val if val.is_ascii_whitespace() => {
			Style::default().fg(Color::Green)
		},
		val if val > 0x20 && val < 0x7f => {
			Style::default().fg(Color::LightCyan)
		},
		val if val.is_ascii() => {
			Style::default().fg(Color::Magenta)
		},
		val => {
			Style::default().fg(Color::Yellow)
		}
	}
}