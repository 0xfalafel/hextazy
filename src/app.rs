use ratatui::buffer;
use std::io::prelude::*;
use std::io::{SeekFrom, BufReader, Error, ErrorKind};
use std::fs::{File, OpenOptions};
use std::process::exit;
use regex::Regex;

use crate::reset_terminal;
use crate::usage;

#[derive(PartialEq)]
pub enum CurrentEditor {
	HexEditor,
	AsciiEditor,
	CommandBar
}

#[derive(Clone)]
pub struct CommandBar {
	pub command: String,
	pub cursor: u64
}

#[derive(PartialEq)]
pub struct SearchResults {
	match_addresses: Vec<u64>, // vector of addresses where the text searched has been found
	query_length: usize			// len of the searched text, used to highlight search results
}

pub struct App {
	filename: String,	// 
	reader: BufReader<File>,
	file: File,
	pub offset: u64,		// where are we currently reading the file
	pub file_size: u64,		// size of the file
	pub cursor: u64,		// position of the cursor on the interface
	pub lines_displayed: u16, // the number of lines currently displayed 
							  // by the interface
	pub editor_mode: CurrentEditor,
	pub command_bar: Option<CommandBar>,
	pub search_results: Option<SearchResults>
}

impl App {

	pub fn new(filename: String) -> Result<App, std::io::Error> {

		// Open the file in Read / Write mode
		let mut file_openner = OpenOptions::new()
			.read(true)
			.write(true)
			.open(&filename);

		// If we can't open it Read / Write.
		// Open it as Read Only.
		let mut f = file_openner.unwrap_or_else(|error| {
			if error.kind() == ErrorKind::PermissionDenied {
				OpenOptions::new()
				.read(true)
				.open(&filename).
				expect("Could not open file")
			} else if error.kind() == ErrorKind::NotFound {
				reset_terminal();
				println!("Error: file not found.");
				usage();
				exit(1);
			} else {
				panic!("Problem opening the file: {:?}", error);
			}
		});


		let size = f.metadata()?.len();

		let app = App {
			filename: filename,
			reader: BufReader::new(f.try_clone()?),
			file: f,
			offset: 0,
			file_size: size,
			cursor: 0,
			lines_displayed: 0x100, // updated when the ui is created
			editor_mode: CurrentEditor::HexEditor,
			command_bar: None,
			search_results: None
		};
		Ok(app)
	}

	// reset the "file cusor" to it's intial position (the app.offset value)
	pub fn reset(&mut self) {
		let seek_addr = SeekFrom::Start(self.offset);
		self.reader.seek(seek_addr);
	}

	pub fn length_to_end(&self) -> u64 {
		self.file_size - self.offset
	}

	// read 8 bytes
	pub fn read_8(&mut self) -> [u8; 8] {
		let mut buf = [0;8];
		&self.reader.read(&mut buf);
		buf
	}

	// read 16 bytes
	pub fn read_16(&mut self) -> [u8; 16] {
		let mut buf = [0;16];
		self.reader.read(&mut buf);
		buf
	}

	pub fn write(&mut self, cursor: u64, value: u8) {
		let offset = cursor / 2; // use this to point at the edited byte
		let seek_addr = SeekFrom::Start(offset);
		self.file.seek(seek_addr);

		// get the value pointed by the cusor
		let mut buffer: [u8; 1] = [0; 1]; // we need a buffer, even if we read 1 value.
		self.file.read_exact(& mut buffer);

		let original_value = buffer[0];

		// Determine if we write the first or second letter of the byte
		let mut new_value: u8;

		if cursor % 2 == 0 { // we edit the first char of the hex
			new_value = original_value & 0b1111;
			new_value = new_value ^ (value << 4);
		} 
		else { // we edit the second char of the hex
			new_value = original_value & 0b11110000;
			new_value = new_value ^ value;
		}

		// Write the byte
		self.file.seek(seek_addr);
		self.file.write_all(&[new_value]);

		self.reset();
	}

	/// write a byte at the address given
	pub fn write_ascii(&mut self, cursor: u64, value: u8) {
		let offset = cursor / 2; // use this to point at the edited byte
		let seek_addr = SeekFrom::Start(offset);

		// Write the byte
		self.file.seek(seek_addr);
		self.file.write_all(&[value]);

		self.reset();
	}


	// read 16 bytes, and return the length
	pub fn read_16_length(&mut self) -> ([u8; 16], usize) {
		let mut buf = [0;16];
		let read_length: usize;
		
		read_length = self.reader.read(&mut buf).unwrap();
		(buf, read_length)
	}


	// self.offset = self.offset + direction
	// but we check if the result is bellow 0 or lager than the file
	pub fn change_offset(&mut self, direction:i64) {
		// check if result is bellow 0
		if direction.wrapping_add_unsigned(self.offset.into()) < 0 {
			self.offset = 0;
			return;
		}

		self.offset = self.offset.wrapping_add_signed(direction.into());

		// if offset is beyond the end of file, fix it
		if self.offset > self.file_size - 0x10 {

			// handle the last line proprely
			if self.file_size % 0x10 == 0 { 
				self.offset = self.file_size - 0x10;
			} else {
				self.offset = self.file_size - (self.file_size % 0x10);

				// handle the case where the cursor is just before the last line,
				// but can't go down without exceeding file size.
				if self.offset * 2 > self.cursor {
					self.offset = self.offset - 0x10;
				}
			}
		}
	}

	// self.cursor = self.cursor + direction
	// but we check if the address is bellow 0 or lager than the file
	pub fn change_cursor(&mut self, direction:i64){
		// check the address is bellow 0
		if direction.wrapping_add_unsigned(self.cursor.into()) < 0 {
			self.cursor = 0 + (self.cursor % 0x20);
			return;
		}

		// check if the new cursor address is longer than the file
		// (file_size * 2) - 1 because we have 2 chars for each hex number.
		if self.cursor.wrapping_add_signed(direction.into()) > (self.file_size * 2) - 1 {

			//  + (self.cursor % 0x20) = stay on the same column

			// case where the last line is an exact fit
			if self.file_size % 0x10 == 0 {
				self.cursor = self.file_size * 2 - 0x20 + (self.cursor % 0x20); // stay on the same column
			}

			// we have an incomplete last line
			else {
				let last_line_length = self.file_size % 0x10;
				let column_of_cursor = (self.cursor / 2) % 0x10;
				
				let start_of_last_line = self.file_size - (self.file_size % 0x10);
				let start_of_last_line = start_of_last_line * 2;

				// cursor is on the last line
				if column_of_cursor < last_line_length {
					self.cursor = start_of_last_line + (self.cursor % 0x20);
				}
				
				// cursor is on the line just before the last, but can't go down
				// without exceeding file size
				else {
					self.cursor = start_of_last_line - 0x20 + (self.cursor % 0x20);
				}

			}

			self.change_offset(0x10); // move the view one line down
			return;
		}

		self.cursor = self.cursor.wrapping_add_signed(direction.into());

		// case where by moving the cursor to the left, we go before the offset
		if self.cursor < self.offset * 2 {
			self.change_offset(-0x10);
		}

		// case where by moving the cursor to the right, we go below what the screen displays
		if self.cursor > self.offset * 2 + u64::from(self.lines_displayed)*2*0x10 - 1 {
			self.change_offset(0x10);
		}
	}

	/// use to jump directly at an address, and move the interface accordingly
	pub fn jump_to(&mut self, mut new_address: u64) {
		// check that the address is not bellow the file
		if new_address > self.file_size {
			new_address = self.file_size-1;
		}

		// if address is not on the page currently displayed,
		// jump on the address and display it in the middle of the page
		if (new_address < self.offset) || new_address > self.offset + u64::from(self.lines_displayed-1)*0x10 {
			self.cursor = new_address * 2;

			// cursor should be in the middle of the screen:
			// self.offset = self.cursor - (half the screen)
			let mut lines_before_cursor = (u64::from(self.lines_displayed)/2) * 0x10;
			self.offset = u64::saturating_sub(new_address, lines_before_cursor);

			self.offset = self.offset - (self.offset %0x10); // align self.offset to 0x10
		
		// the new address is displayed on the screen, just move the cursor
		} else {
			self.cursor = new_address * 2;
		}
	}



	/// interpret commands
	pub fn interpret_command(&mut self) {
		let mut command = &mut self.command_bar.clone().unwrap().command;

		// exit - :q
		let regex_q = Regex::new(r"^:\s?+q\s?+$").unwrap();
		if regex_q.is_match(command) {
			reset_terminal();
			exit(0);
		}

		// command is hex address (:0x...), we jump to this address
		let hexnum_regex = Regex::new(r"^:\s?+0[xX][0-9a-fA-F]+$").unwrap();
		if hexnum_regex.is_match(command) {

			// strip spaces and the 0x at the start
			command.remove(0); // remove ':' at the start
			let command = command.trim().strip_prefix("0x").unwrap();

			// convert hex string to u64
			let parse_address = u64::from_str_radix(command, 16);

			match (parse_address) {
				Ok(address) => {&self.jump_to(address);},
				Err(e) => {return} // handle error if we have a parseInt error
			}
		}

		// command is a search (/abc or :/abc)
		let search_regex = Regex::new(r":?/\s?+(\w+)").unwrap();
		if search_regex.is_match(command) {

			// extract search (remove ':/')
			let capture = search_regex.captures(command).unwrap();
			let search = &capture[1];

			// we search an Hex value
			// by definition, an hex representation is also valid ascii
			// if search == "abc" {
			// 	&self.jump_to(0x42);
			// }

			// we search Ascii
			// note: since Hextazy can't display utf-8, it doesn't make sense to search
			// non-ascii chars
			if search.is_ascii() {
				self.search_ascii(search);
				//&self.jump_to(0x42);
			}
		}
	}

	fn search_ascii(&mut self, search: &str) {
		// create a new file reader and buffer, so we don't disrupt our display loop with reads() and seek()
		let mut file = self.file.try_clone().unwrap();
		file.seek(SeekFrom::Start(0)).unwrap();
		let mut reader = BufReader::new(file);

		let first_char = search.chars().nth(0).unwrap();
		let mut buf: [u8; 1] = [0; 1]; // apparently we are supposed to use a buffer, don't juge me

		// read the whole file, and see if a byte match the first char of the search
		// if it's a match, we go in a more in depth search
		loop {
			let read_res = reader.read(&mut buf);

			// If EOF, return
			match read_res {
				Err(e) => {return}
				Ok(len) if len == 0 => { return } // we have reach End Of File
				_ => {}
			}

			// we have a match !
			if buf[0] == first_char as u8 {

				// store where we found the first char
				let match_address = reader.stream_position().unwrap() - 1;

				// check if the rest of the string also matches
				

				self.jump_to(match_address);
				break;
				// if self.search_results == None {

				// }
			}
		}
	}
}