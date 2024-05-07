#![allow(unused)]

use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Style},
	text::{Line, Span, Text},
	widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
	Frame
};
use crate::App;


pub fn ui(f: &mut Frame, app: &App) { //, app: &App) {
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([
			Constraint::Length(10),
			Constraint::Min(16),
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


	/* Hex Block */
	let hex_block = Block::default()
		.borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
		.style(Style::default());

	let line = Paragraph::new(Text::styled(
		" de ad be ef ",
		Style::default().fg(Color::Green)
	)).block(hex_block);

	f.render_widget(line, chunks[1]);
}