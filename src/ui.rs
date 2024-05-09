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
			Constraint::Length(46),
			Constraint::Length(8)
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

	let hex_block = Block::default()
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default());


	let buf = app.read_16();
	let line = render_hex_line(buf);
	hex_lines.push(line);


	let buf = app.read_16();
	let line = render_hex_line(buf);
	hex_lines.push(line);

	let text = Text::from(hex_lines);
	let paragraph = Paragraph::new(text).block(hex_block);

	f.render_widget(paragraph, chunks[1]);
}

fn render_line_8(buf: [u8; 8]) -> Text<'static> {
	let mut hex_chars: Vec<Span> = vec![];

	for val in buf {
		match val {
			val if val == 0x00 => {
				hex_chars.push(
					Span::styled(
						format!(" {:02x}", val),
						Style::default().fg(Color::DarkGray)
				));
			},
			val => {
				hex_chars.push(
					Span::styled(
						format!(" {:02x}", val),
						Style::default().fg(Color::Yellow)
				));
			},
		}
	}

	let line = Line::from(hex_chars);
	Text::from(line)
}

fn render_hex_line(buf: [u8; 16]) -> Line<'static> {
	let mut hex_chars: Vec<Span> = vec![];

	for i in 0..7 {
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

	for i in 8..15 {
		hex_chars.push(
			Span::styled(
				format!(" {:02x}", buf[i]),
				colorize(buf[i])
			)
		);
	}

	Line::from(hex_chars)
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
		val if val.is_ascii_alphanumeric() => {
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