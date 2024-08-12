use std::io::{prelude::*, Error};
use std::io::{SeekFrom, BufReader, ErrorKind};
use std::fs::{File, OpenOptions};
use std::process::exit;
use regex::Regex;
use std::collections::HashMap;

use crate::reset_terminal;
use crate::usage;

pub use crate::search::{search_ascii, search_hex, search_hex_ascii, search_hex_reverse,
	convert_hexstring_to_vec, SearchResults};

#[derive(PartialEq)]
pub enum CurrentEditor {
	HexEditor,
	AsciiEditor,
	CommandBar,
	ExitPopup
}

#[derive(Clone)]
pub struct CommandBar {
	pub command: String,
	pub cursor: u64
}

#[allow(unused)]
pub enum WarningLevel {
	Info,
	Warning,
	Error
}

pub struct App {
	reader: BufReader<File>,
	file: File,
	pub offset: u64,		// where are we currently reading the file
	pub file_size: u64,		// size of the file
	pub cursor: u64,		// position of the cursor on the interface
	pub lines_displayed: u16, // the number of lines currently displayed 
							  // by the interface
	pub editor_mode: CurrentEditor,
	pub command_bar: Option<CommandBar>,
	pub search_results: Option<SearchResults>,
	pub error_msg: Option<(WarningLevel, String)>,
	pub modified_bytes:  HashMap<u64, u8>, // store every modified bytes (address, new_value) in this vector
									   // we write the bytes to the disk only when exiting the app.

	history: Vec<(u64, u8)>,		// store the (address, old_value) of bytes edited for undo() 
	history_redo: Vec<(u64, u8)>	// used when we restore history. We can go back with redo()
}

impl App {

	pub fn new(filename: String) -> Result<App, std::io::Error> {

		// Open the file in Read / Write mode
		let file_openner = OpenOptions::new()
			.read(true)
			.write(true)
			.open(&filename);

		// If we can't open it Read / Write.
		// Open it as Read Only.
		let f = file_openner.unwrap_or_else(|error| {
			if error.kind() == ErrorKind::PermissionDenied {
				OpenOptions::new()
				.read(true)
				.open(&filename).
				expect("Could not open file")
			} else if error.kind() == ErrorKind::NotFound {
				reset_terminal().expect("Failed to reset the terminal. Use the `reset` command in your terminal.");
				println!("Error: file not found.");
				usage();
				exit(1);
			} else {
				panic!("Problem opening the file: {:?}", error);
			}
		});


		let size = f.metadata()?.len();

		let app = App {
			reader: BufReader::new(f.try_clone()?),
			file: f,
			offset: 0,
			file_size: size,
			cursor: 0,
			lines_displayed: 0x100, // updated when the ui is created
			editor_mode: CurrentEditor::HexEditor,
			command_bar: None,
			search_results: None,
			error_msg: None,
			modified_bytes: HashMap::new(),
			history: vec![],
			history_redo: vec![]
		};
		Ok(app)
	}

	// reset the "file cusor" to it's intial position (the app.offset value)
	pub fn reset(&mut self) {
		let seek_addr: SeekFrom = SeekFrom::Start(self.offset);
		self.reader.seek(seek_addr).expect("Failed to reset the cursor");
	}

	pub fn length_to_end(&self) -> u64 {
		self.file_size - self.offset
	}

	fn add_error_message(&mut self, level: WarningLevel, message: String) {
		self.error_msg = Some((level, message));
	}

	pub fn cleanup_error_message(&mut self) {
		self.error_msg = None;
	}

	/// read a single byte (u8) at the address `address`, from `self.reader`
	/// if the byte has been modified, give the value from `self.modified`
	pub fn read_byte_addr(&mut self, address: u64) -> Result<u8, std::io::Error> {
		if let Some(&value) = self.modified_bytes.get(&address) {
			return Ok(value);
		}
		let seek_addr = SeekFrom::Start(address);
		self.reader.seek(seek_addr)?;

		let mut buf: [u8; 1] = [0;1];
		self.reader.read_exact(&mut buf)?;

		let value: u8 = buf[0];
		Ok(value)
	}

	/// read a single byte (u8) at the address `address`, from `self.reader`
	/// Even if the byte has been modified, give the value from `self.reader`
	pub fn read_byte_addr_file(&mut self, address: u64) -> Result<u8, std::io::Error> {
		let seek_addr = SeekFrom::Start(address);
		self.reader.seek(seek_addr)?;

		let mut buf: [u8; 1] = [0;1];
		self.reader.read_exact(&mut buf)?;

		let value: u8 = buf[0];
		Ok(value)
	}

	/// write a single byte (u8), at the address `address`
	pub fn write_byte(&mut self, address: u64, value: u8) -> Result<(), std::io::Error> {

		// bytes written are stored inside the hashmap `modified_bytes` and only
		// written when the user save the file.
		self.modified_bytes.insert(address, value);

		// if the `value` is the same as the current byte. Remove it from
		// the `self.modified_bytes` hashmap.
		// Also, let's not write an error message for such a little optimisation.
		if let Ok(current_byte) = self.read_byte_addr_file(address) {
			if current_byte == value {
				self.modified_bytes.remove(&address);
			}
		}
		Ok(())
	}

	pub fn write(&mut self, cursor: u64, value: u8) {
		let offset = cursor / 2; // use this to point at the edited byte
		self.backup_byte(offset);

		let original_value = self.read_byte_addr(offset).expect("Failed to write byte");

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
		self.write_byte(offset, new_value).expect("Failed to write byte");

		// empty self.history_redo
		if self.history_redo.len() > 0 {
			self.history_redo = vec![];
		}

		self.reset();
	}

	/// write a byte at the address given
	pub fn write_ascii(&mut self, cursor: u64, value: u8) {
		let offset = cursor / 2; // use this to point at the edited byte
		self.backup_byte(offset);

		// Write the byte
		self.write_byte(offset, value)
			.unwrap_or_else(|_err| {
				self.add_error_message(
					WarningLevel::Warning,
					format!("Failed to write the byte at offset 0x{:x}", offset)
				)});

		// empty self.history_redo
		if self.history_redo.len() > 0 {
			self.history_redo = vec![];
		}

		self.reset();
	}

	/// store every byte edited in self.history
	fn backup_byte(&mut self,address: u64) {
		
		if let Ok(value) = self.read_byte_addr(address) {
			// add it to the history
			self.history.push((address, value));
		} else {
			self.add_error_message(WarningLevel::Warning, format!("Could not backup byte at address 0x{:x}", address));
		}
	}

	/// restore the last edited byte from self.history
	pub fn undo(&mut self) {
		// get value from self.history
		let (address, old_value) = match self.history.pop() {
			None => { return }
			Some ((address, old_value)) => {(address, old_value)}
		};

		// copy the the current value of the byte we restore into history.redo
		if let Ok(current_value) = self.read_byte_addr(address) {
			self.history_redo.push((address, current_value));

			// write the value from self.history
			self.write_byte(address, old_value).unwrap_or_else(|_err| {
				self.add_error_message(
					WarningLevel::Error,
					"Undo: Failed to restore byte".to_string()
				)
			});
		} else {
			self.add_error_message(
				WarningLevel::Warning,
				"Undo error: Failed to store previous byte in redo history".to_string());
		}

		self.jump_to(address);
	}

	/// invert the previous undo() using self.history_redo
	pub fn redo(&mut self) {
		// get value from self.history_redo
		let (address, redo_value) = match self.history_redo.pop() {
			None => { return }
			Some ((address, redo_value)) => {(address, redo_value)}
		};

		// add the current value to self.history
		self.backup_byte(address);

		// write the value from self.history_redo
		self.write_byte(address, redo_value).unwrap_or_else(|_err| {
			self.add_error_message(
				WarningLevel::Warning,
				format!("Could not restore byte at address 0x{:x}",address)
			)
		});

		self.jump_to(address);
	}

	/// undo all changes using self.history
	pub fn undo_all(&mut self) {
		while self.history.len() > 0 {
			self.undo();
		}
	}

	/// written all the modified bytes into the file.
	pub fn save_to_disk(&mut self) -> Result<(), Error>{
		for (address, value) in self.modified_bytes.clone().into_iter() {
			let seek_addr = SeekFrom::Start(address);
			self.file.seek(seek_addr)?;	
			self.file.write_all(&[value])?;
		}

		Ok(())
	}

	// read 16 bytes, and return the length
	pub fn read_16_length(&mut self) -> ([u8; 16], usize) {
		// get the position of our cursor in the BufReader
		let current_address = self.reader.stream_position()
			.expect("Could not get cursor position in read_16_length()"); 
		
		// read 16 bytes
		let mut buf = [0;16];
		let read_length: usize;

		read_length = self.reader.read(&mut buf).unwrap();

		// if we have modified values at one of the addresses
		// replace the value with the one from self.modified_bytes
		for i in 0..(read_length as u64) {
			let checked_address = current_address + i;

			if let Some(&modified_byte) = self.modified_bytes.get(&checked_address) {
				buf[i as usize] = modified_byte;
			};
		}

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
	pub fn jump_to(&mut self, new_address: u64) {
		let mut new_address = new_address;

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
			let lines_before_cursor = (u64::from(self.lines_displayed)/2) * 0x10;
			self.offset = u64::saturating_sub(new_address, lines_before_cursor);

			self.offset = self.offset - (self.offset %0x10); // align self.offset to 0x10
		
		// the new address is displayed on the screen, just move the cursor
		} else {
			self.cursor = new_address * 2;
		}
	}

	#[allow(unused)]
	pub fn add_to_search_results(&mut self, result_address: u64, query_len: usize) {
		if let Some(ref mut search_results) = &mut self.search_results {
			search_results.match_addresses.push(result_address);
		} else {
			self.jump_to(result_address);
			self.search_results = Some(SearchResults{
				match_addresses: vec![result_address],
				query_length: query_len
			})
		}
	}

	/// jump to the search first result after our cursor
	pub fn go_to_next_search_result(&mut self) {

		// if we don't have any search results, return
		let search_results = match &self.search_results {
			None => {return},
			Some(search_results) => {search_results}
		};
		
		// find the first search result with an address
		// that is after our current cursor

		let current_address = self.cursor / 2;
		let mut new_address: Option<u64> = None;
		
		for addr in &search_results.match_addresses {
			if *addr > current_address {
				new_address = Some(*addr);
				break;
			}
		}

		// and jump to it. If we found one
		if let Some(new_addr) = new_address {
			self.jump_to(new_addr);
		}
	}

	/// jump to the search first result before our cursor
	pub fn go_to_previous_search_result(&mut self) {

		// if we don't have any search results, return
		let search_results = match &self.search_results {
			None => {return},
			Some(search_results) => {search_results}
		};
		
		// find the first search result with an address
		// that is after our current cursor

		let current_address = self.cursor / 2;
		let mut new_address: Option<u64> = None;
		
		for addr in (&(&search_results).match_addresses).into_iter().rev() {
			if *addr < current_address {
				new_address = Some(*addr);
				break;
			}
		}

		// and jump to it. If we found one
		if let Some(new_addr) = new_address {
			self.jump_to(new_addr);
		}
	}

	/// interpret commands
	pub fn interpret_command(&mut self) {
		let command = &mut self.command_bar.clone().unwrap().command;

		// exit - :q
		let regex_q = Regex::new(r"^:\s?+q\s?+$").unwrap();
		if regex_q.is_match(command) {
			reset_terminal().expect("Failed to reset the terminal. Use the `reset` command in your terminal.");
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

			match parse_address {
				Ok(address) => {self.jump_to(address);},
				Err(_e) => {self.add_error_message(
					WarningLevel::Warning,
					"Failed to parse given address".to_string())} // handle error if we have a parseInt error
			}
			return;
		}

		// command is a search with hex addresses (/42ff or :/42ff)
		let search_hex_ascii_regex = Regex::new(r"^:?\s?+/([a-fA-F0-9]{2}+)").unwrap();
		if search_hex_ascii_regex.is_match(command) {
			// remove previous search results
			self.search_results = None;

			// extract search (remove ':/')
			let capture = search_hex_ascii_regex.captures(command).unwrap();
			let searched_text = &capture[1];

			// convert the searched hex string to a vector of u8
			let searched_len = searched_text.len();
			let mut searched_bytes: Vec<u8> = vec!();

			for i in (0..searched_len).step_by(2) {
				let hex_byte = &searched_text[i..i+2];
				let byte = u8::from_str_radix(hex_byte, 16).unwrap();

				searched_bytes.push(byte);
			}

			// do the search. We search both hex values, and the ascii string.
			let file_copy = self.file.try_clone().unwrap();
			let res = search_hex_ascii(file_copy, searched_text, searched_bytes);

			// update self with the search results
			match res {
				Err(_e) => {
					self.add_error_message(
						WarningLevel::Error,
						"Error: search failed".to_string());
				},
				Ok(Some(search_results)) => {
					self.search_results = Some(search_results);
					self.go_to_next_search_result();
				},
				Ok(None) => {self.search_results = None}
			};

			return;
		}

		// command is a search (/abc or :/abc)
		let search_regex = Regex::new(r"^:?/\s?+(\w+)").unwrap();
		if search_regex.is_match(command) {
			// remove previous search results
			self.search_results = None;

			// extract search (remove ':/')
			let capture = search_regex.captures(command).unwrap();
			let search = &capture[1];

			// we search Ascii
			// note: since Hextazy can't display utf-8, it doesn't make sense to search
			// non-ascii chars
			if search.is_ascii() {

    			// create a new file, so we don't disrupt our display loop with reads() and seek()
				let file_copy = self.file.try_clone().unwrap();
				let res = search_ascii(file_copy, search);

				match res {
					Err(_e) => {
						self.add_error_message(
							WarningLevel::Error,
							"Error: ascii search failed".to_string());
					},
					Ok(Some(search_results)) => {
						self.search_results = Some(search_results);
						self.go_to_next_search_result();
					},
					Ok(None) => {self.search_results = None}
				};
			}
			return;
		}

		// command is an hex search (ie, ':x/42')
		// todo: handle search that begin with '0x'
		let hexsearch_regex: Regex = Regex::new(r"^:\s?+x\s?+/([0-9a-fA-F]{2}+)$").unwrap();
		if hexsearch_regex.is_match(command) {
			// remove previous search results
			self.search_results = None;

			let capture = hexsearch_regex.captures(command).unwrap();
			let searched_text = &capture[1];

			// convert the searched hex string to a vector of u8
			let search = convert_hexstring_to_vec(searched_text);

			// do the actual search with search_hex(), and store the result
			let file_copy = self.file.try_clone().unwrap();
			let res = search_hex(file_copy, search);

			match res {
				Err(_e) => {
					self.add_error_message(
						WarningLevel::Error,
						"Error: hexadecimal search failed".to_string());
				},
				Ok(Some(search_results)) => {
					self.search_results = Some(search_results);
					self.go_to_next_search_result();
				},
				Ok(None) => {
					self.search_results = None;
				}
			};

			return;
		}

		// command is an inverted hex search (ie, ':ix/4342')
		let hexsearch_regex: Regex = Regex::new(r"^:\s?+xi\s?+/([0-9a-fA-F]{2}+)$").unwrap();
		if hexsearch_regex.is_match(command) {
			// remove previous search results
			self.search_results = None;

			let capture = hexsearch_regex.captures(command).unwrap();
			let searched_text = &capture[1];

			// convert the searched hex string to a vector of u8
			let search = convert_hexstring_to_vec(searched_text);

			// do the actual search with search_hex_reverse(), and store the result
			let file_copy = self.file.try_clone().unwrap();
			let res = search_hex_reverse(file_copy, search);

			match res {
				Err(_e) => {
					self.add_error_message(
						WarningLevel::Error,
						"Error: inverted hexadecimal search failed".to_string());
				},
				Ok(Some(search_results)) => {
					self.search_results = Some(search_results);
					self.go_to_next_search_result();
				},
				Ok(None) => {
					self.search_results = None;
				}
			};

			return;
		}

		// command is an ascii search (:s/abc)
		let ascii_search_regex = Regex::new(r"^:\s?+s\s?+/\s?+(\w+)").unwrap();
		if ascii_search_regex.is_match(command) {
			// remove previous search results
			self.search_results = None;

			// extract search (remove ':/')
			let capture = ascii_search_regex.captures(command).unwrap();
			let search = &capture[1];

			// we search Ascii
			// note: since Hextazy can't display utf-8, it doesn't make sense to search
			// non-ascii chars
			if search.is_ascii() {
				let res = search_ascii(self.file.try_clone().unwrap(), search);
				
				match res {
					Err(_e) => {
						self.add_error_message(
							WarningLevel::Error,
							"Error: inverted hexadecimal search failed".to_string());
					},
					Ok(Some(search_results)) => {
						self.search_results = Some(search_results);
						self.go_to_next_search_result();
					},
					Ok(None) => {
						self.search_results = None;
					}
				};	
			} else {
				self.add_error_message(
					WarningLevel::Info,
					"Search only support ascii characters".to_string()
				);
			}
			return;
		}

		// command is an empty search (:s/abc), cleanup search results
		let empty_search_regex = Regex::new(r"^:?\s?+/$").unwrap();
		if empty_search_regex.is_match(command) {
			self.search_results = None;
			return;
		}
	}
}