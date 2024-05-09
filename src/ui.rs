#![allow(unused)]

use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Style},
	text::{Line, Span, Text},
	widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
	Frame,
	symbols
};
use crate::App;


pub fn ui(f: &mut Frame, app: &mut App) { //, app: &App) {
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
		.borders(Borders::ALL)
		.style(Style::default());


	// Create a list of address
	let mut list_items = Vec::<ListItem>::new();

	let start_address = 0;
	let size = chunks[0].height;
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
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default());

	let mut hex_lines: Vec<Line> = vec![];

	/* Create ASCII Block */
	let ascii_block = Block::default()
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default());

	let mut ascii_lines: Vec<Line> = vec![];

	/* calculate how much we can read */
	let remaining_file_size = app.length_to_end();
	let lines_to_end = chunks[1].height;

	let mut lines_to_read = remaining_file_size / 16;

	if (remaining_file_size % 16) != 0 {
		lines_to_read = lines_to_read + 1;
	}

	for i in 0..lines_to_read {
		// 1st Read
		let buf = app.read_16();
		// hex line
		let hex_line = render_hex_line(buf);
		hex_lines.push(hex_line);

		// ascii line
		let ascii_line = render_ascii_line(buf);
		ascii_lines.push(ascii_line);
	}


	let text = Text::from(hex_lines);
	let paragraph = Paragraph::new(text).block(hex_block);
	f.render_widget(paragraph, chunks[1]);

	let ascii_text = Text::from(ascii_lines);
	let ascii_paragraph = Paragraph::new(ascii_text).block(ascii_block);
	f.render_widget(ascii_paragraph, chunks[2]);
}

/// Take a buffer of u8[16] and render it with a colorize hex line
fn render_hex_line(buf: [u8; 16]) -> Line<'static> {
	let mut hex_chars: Vec<Span> = vec![];

	for i in 0..8 {
		hex_chars.push(
			Span::styled(
				format!(" {:02x}", buf[i]),
				colorize(buf[i])
			)
		);
	}

	hex_chars.push(
		Span::styled(" ┊",
			Style::default().fg(Color::White)
	));

	for i in 8..16 {
		hex_chars.push(
			Span::styled(
				format!(" {:02x}", buf[i]),
				colorize(buf[i])
			)
		);
	}

	Line::from(hex_chars)
}

/// Take a buffer of u8[16] and render it with a colorize ascii line
fn render_ascii_line(buf: [u8; 16]) -> Line<'static> {
	let mut ascii_colorized: Vec<Span> = vec![];

	for i in 0..8 {
		ascii_colorized.push(
			render_ascii_char(buf[i])
		);
	}

	ascii_colorized.push(
		Span::styled("┊",
			Style::default().fg(Color::White)
	));

	for i in 8..16 {
		ascii_colorized.push(
			render_ascii_char(buf[i])
		);
	}
	Line::from(ascii_colorized)
}

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