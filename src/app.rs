use std::io::{prelude::*, Error};
use std::io::{SeekFrom, BufReader, BufWriter, ErrorKind};
use std::fs::{File, OpenOptions};
use std::process::exit;
use std::cmp::{min, max};
use regex::Regex;
use std::collections::BTreeMap;

use crate::reset_terminal;

pub use crate::search::{search_ascii, search_hex, search_hex_ascii, search_hex_reverse,
	convert_hexstring_to_vec, SearchResults};

#[derive(PartialEq, Clone, Copy)]
pub enum CurrentEditor {
	HexEditor,
	AsciiEditor,
	CommandBar,
	ExitPopup
}

#[derive(Clone)]
pub struct CommandBar {
	pub command: String,
	pub _cursor: u64
}

#[allow(unused)]
pub enum WarningLevel {
	Info,
	Warning,
	Error
}

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
	Overwrite,
	Insert
}

#[derive(Debug, PartialEq)]
struct Inserted {
	vector_address: u64,
	offset_in_vector: u64
}

#[derive(PartialEq, Debug)]
enum Addr {
	FileAddress(u64),
	InsertedAddress(Inserted)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Changes {
	Insertion(Vec<u8>),
	Deleted
}



/// used for self.history
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Modification {
	Insertion,
	Modification,
	Deletetion
}

/// Different braille mode available for the Ascii pane display.
/// Default is None, where we don't use braille dump.
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Braille {
	None,	// default mode, don't use Braille Dump
	Mixed,	// Braille dump for 0x80 and above
	Full	// Braille dump for all 255 values
}

pub struct App {
	reader: BufReader<File>,
	pub file_path: String,
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
	pub modified_bytes:  BTreeMap<u64, Changes>, // store every inserted bytes (address, new_value) in this vector
												 // we write the bytes to the disk only when exiting the app.
												 // BTreeMap instead of Hashmap because it is always sorted.

	pub history: Vec<(Modification, u64, Option<u8>)>,	// store the (Modification, address, old_value) of bytes edited for undo() 
	history_redo: Vec<(Modification, u64, Option<u8>)>,	// used when we restore history. We can go back with redo()

	// mode: overwrite, insert
	pub mode: Mode,
	pub braille: Braille, // Display the ascii non printable chars using Braille dump (https://justine.lol/braille/)
	
	pub selection_start: Option<u64>, // Indicate the start of the selection,
									  // chars between this address and the cusor
									  // are selected.

	// interface customization options
	pub show_infobar: bool,

	pub last_address_read: u64,		// used by the app to keep track of where our reader is
}

impl App {

	pub fn new(file_path: String, braille_mode: Braille, seek: Option<u64>) -> Result<App, std::io::Error> {

		// Open the file in Read / Write mode
		let file_openner = OpenOptions::new()
			.read(true)
			.write(true)
			.open(&file_path);

		let mut mode = Mode::Overwrite;
		let mut error_msg: Option<(WarningLevel, String)> = None;

		// If we can't open it Read / Write.
		// Open it as Read Only.
		let f = file_openner.unwrap_or_else(|error| {
			if error.kind() == ErrorKind::PermissionDenied {
				error_msg = Some((WarningLevel::Info, "File opened as Read-Only.".to_string()));

				OpenOptions::new()
				.read(true)
				.open(&file_path).
				expect("Could not open file")
			} else if error.kind() == ErrorKind::NotFound {
				// Create the file if it doesn't exists
				match File::create_new(&file_path) {
					Ok(file) => {mode = Mode::Insert; file},
					Err(_e) => {
						eprintln!("Could not create the file {}", &file_path);
						reset_terminal().expect("Failed to reset the terminal. Use the `reset` command in your terminal.");
						exit(1);
					}
				}				
			} else {
				panic!("Problem opening the file: {:?}", error);
			}
		});


		let size = f.metadata()?.len();

		let mut app = App {
			reader: BufReader::new(f.try_clone()?),
			file_path: file_path,
			file: f,
			offset: 0,
			file_size: size,
			cursor: 0,
			lines_displayed: 20, // updated when the ui is created
			editor_mode: CurrentEditor::HexEditor,
			command_bar: None,
			search_results: None,
			error_msg: error_msg,
			modified_bytes: BTreeMap::new(),
			history: vec![],
			history_redo: vec![],
			mode: mode,
			selection_start: None,
			braille: braille_mode,
			show_infobar: true,
			last_address_read: 0,
		};

		app.jump_to(seek.unwrap_or(0));

		Ok(app)
	}

	// reset the "file cusor" to it's intial position (the app.offset value)
	pub fn reset(&mut self) {
		self.last_address_read = self.offset;
	}

	pub fn length_to_end(&self) -> u64 {
		self.file_size - self.offset
	}

	pub fn add_error_message(&mut self, level: WarningLevel, message: String) {
		self.error_msg = Some((level, message));
	}

	pub fn cleanup_error_message(&mut self) {
		self.error_msg = None;
	}


	//   Let's make a schema, were we have inserted `0xaa, 0xbb, 0xcc` before our `0x01` value.
	//                                     ┌──┐                         
	//                                     │01│ 4                       
	//                                     ├──┤                         
	//                                     │cc│ 3    
	//    self.file / self.reader          ├──┤                         
	//                                     │bb│ 2                       
	//   ┌──┬──┬──┬──┬──┬──┬──┬──┐         ├──┤       ┌───────┐                  
	//   │00│01│02│03│04│05│06│07│         │aa│ 1     │Deleted│             
	//   └──┴──┴──┴──┴──┴──┴──┴──┘         └──┘       └───────┘
	//     0  X  5  6  X  7  8  9          <01>			<04>     self.modified_bytes                   
	//  
	//	The `get_real_address(self, address)` make a convertion between our continuous access
	//	and self.modified bytes or self.file.
	//
	//	In the schema above:
	//
	//	- get_real_address(5) -> returns Addr::FileAddress(2)
	//	  This is the address of the file just after our inserted_bytes
	//
	//	- get_real_address(2) -> returns Addr::InsertedAddress( Inserted { 
	//										vector_address: 01,
	//										offset_in_vector: 2
	//									})
	// 	  which points to 0xbb


	/// This function gives use the address we would be accessing if there was
	/// no `inserted_bytes`. If we end up in the middle of an `self.inserted_bytes` vector
	/// we return the key of self.modified_bytes hashmap, and the offset to the byte we are accessing
	fn get_real_address(&self, address: u64) -> Addr {
		let mut address = address;

		for (modified_addr, changes) in &self.modified_bytes {

			// only subtract addresses of bytes inserted
			// before the address we are watching
			if address < *modified_addr {
				break;
			}

			let inserted_vec = match changes {
				Changes::Deleted => {
					address = address + 1;
					continue;
				},
				Changes::Insertion(inserted_vec) => {
					inserted_vec
				}
			};

			
			// .len()-1 because all vector are at least 1
			// and a modification should not shift our access to self.file
			let vec_len: u64 = inserted_vec.len() as u64 - 1;

			// our address is inside the vector
			if *modified_addr <= address && address <= *modified_addr + vec_len {
				return Addr::InsertedAddress( Inserted {
					vector_address: *modified_addr,
					offset_in_vector: address - modified_addr
				});
			}

			address = address - vec_len;
		}

		Addr::FileAddress(address)
	}


	/// read a single byte (u8) at the address `address`, from `self.reader`
	/// if the byte has been modified, give the value from `self.modified`
	pub fn read_byte_addr(&mut self, address: u64) -> Result<u8, std::io::Error> {

		match self.get_real_address(address) {

			// value is in modified_bytes
			Addr::InsertedAddress(Inserted {vector_address, offset_in_vector}) => {

				let changes = self.modified_bytes.get(&vector_address).unwrap();

				match changes {
					Changes::Insertion(values) => {

						// if we are on the last element of the vector
						// we need to move the cursor of self.reader to
						// go over the modified byte
						if offset_in_vector as usize == values.len() - 1 {
							let _ = self.reader.seek_relative(1);
						}

						return Ok(values[offset_in_vector as usize])
					},
					Changes::Deleted => { panic!("Error: trying to read a deleted byte")}
				}
			},

			// value is in the file
			Addr::FileAddress(addr) => {
				let val = self.read_byte_addr_file(addr)?;
				Ok(val)
			}
		}
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
	pub fn write_byte(&mut self, address: u64, value: u8, mode: Mode) -> Result<(), std::io::Error> {

		// We overwrite the current byte, modification is stored inside `app.modified_bytes`
		if mode == Mode::Overwrite {

			let (insertion_address, offset_in_vector) = match self.get_real_address(address) {
				Addr::FileAddress(addr) => (addr, 0),
				Addr::InsertedAddress(Inserted{vector_address, offset_in_vector}) => (vector_address, offset_in_vector)
			};

			match self.modified_bytes.get_mut(&insertion_address) {
				
				// There is no bytes for the moment, we create a Changes::Insertion byte with
				// the new value
				None => {
					let changes = Changes::Insertion(vec![value]);
					self.modified_bytes.insert(insertion_address, changes);
					return Ok(());
				},

				// They are modified bytes, we overwrite the modified byte with a new value
				Some(changes) => {
					match changes {
						Changes::Insertion(inserted_values) => {
							inserted_values[offset_in_vector as usize] = value;
							return Ok(());
						},
						Changes::Deleted => { 
							panic!("Should we be able change a delete byte in overwrite ?");
							// *changes = Changes::Insertion(vec![value]);
							// Ok(())
						}
					}
				}
			}

		// Insertion mode
		} else if mode == Mode::Insert {
			// use the get_read_address function to see where the bytes should be inserted
			let (insertion_address, offset_in_vector) = match self.get_real_address(address) {
				Addr::FileAddress(addr) => (addr, 0),
				Addr::InsertedAddress(Inserted{vector_address, offset_in_vector}) => (vector_address, offset_in_vector)
			};

			match self.modified_bytes.get_mut(&insertion_address) {
				// We append to the end of a file, on in an empty file.
				None if insertion_address >= self.file_size => {
					let inserted_bytes = vec![value];
					self.modified_bytes.insert(insertion_address, Changes::Insertion(inserted_bytes));
				},

				// If there are no inserted bytes, we create a vector with the current value, our new value
				// and we add it to the modified_bytes structure.
				None => {
					let current_val = self.read_byte_addr_file(insertion_address)
						.unwrap_or(0x00); // add a default value so that we can insert bytes in an empty file
					let inserted_bytes = vec![value, current_val];
					self.modified_bytes.insert(insertion_address, Changes::Insertion(inserted_bytes));
				},
				Some(changes) => {
					match changes {
						Changes::Insertion(inserted_bytes) => {
							inserted_bytes.insert(offset_in_vector as usize, value);
						},
						Changes::Deleted => { 
							*changes = Changes::Insertion(vec![value]);
						}
					}
				}
			}

			// We have inserted a new byte, let's update file_size
			self.file_size = self.file_size + 1;
			Ok(())
		} else {
			panic!("Only Insert and Overwrite were implemented for write_byte");
		}
/*
			// bytes written are stored inside the hashmap `modified_bytes` and only
			// written when the user save the file.
			self.modified_bytes.insert(address, value);
	
			// if the `value` is the same as the current byte. Remove it from
			// the `self.modified_bytes` hashmap.
			// Also, let's not write an error message for such a little optimisation.
			if let Ok(current_byte) = self.read_byte_addr_file(address) {
				if current_byte == value {
					}
					self.modified_bytes.remove(&address);
			}
		
		// We insert a new byte. The byte is stored inside `app.inserted_bytes`
		} else {
			if let Some(inserted) = self.inserted_bytes.get_mut(&address) {
				inserted.push(value);
			} else {
				self.inserted_bytes.insert(address, vec![value]);
			}
		}
		Ok(())
		 */
	}

	pub fn write(&mut self, cursor: u64, value: u8) {
		let address = cursor / 2; // use this to point at the edited byte

		if self.mode == Mode::Overwrite {
			// return if we don't have any bytes we can overwrite
			if self.file_size == 0 {
				self.add_error_message(WarningLevel::Info, String::from("No byte to overwrite"));
				return;
			}

			self.add_to_history(Modification::Modification, address);

			let original_value = self.read_byte_addr(address).expect("Failed to write byte");
	
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
			self.write_byte(address, new_value, Mode::Overwrite)
				.expect("Failed to write byte");
		
		} else if self.mode == Mode::Insert {
			
			if cursor % 2 == 0 { // we edit the first char of the hex
				self.add_to_history(Modification::Insertion, address);
				
				let value = value << 4;
				self.write_byte(address, value, Mode::Insert)
				.expect("Failed to insert byte");
			

			} else { // we edit the second char of the hex -> Overwrite instead of Insterting
				self.add_to_history(Modification::Modification, address);
				
				let original_value = self.read_byte_addr(address).expect("Failed to write byte");

				let new_value = (original_value & 0b11110000) ^ value;

				self.write_byte(address, new_value, Mode::Overwrite)
					.expect("Failed to overwrite the 2nd char of byte");
			}
		}

		else { panic!("Only Mode::Overwrite and Mode::Insert were considered")}

		// empty self.history_redo
		if self.history_redo.len() > 0 {
			self.history_redo = vec![];
		}

		self.reset();
	}

	/// write a byte at the address given
	pub fn write_ascii(&mut self, cursor: u64, value: u8) {
		let address = cursor / 2; // use this to point at the edited byte

		// Add the current value to history
		match self.mode {
			Mode::Overwrite => self.add_to_history(Modification::Modification, address),
			Mode::Insert => self.add_to_history(Modification::Insertion, address)
		}

		// Write the byte
		self.write_byte(address, value, self.mode)
			.unwrap_or_else(|_err| {
				self.add_error_message(
					WarningLevel::Warning,
					format!("Failed to write the byte at address 0x{:x}", address)
				)});

		// empty self.history_redo
		if self.history_redo.len() > 0 {
			self.history_redo = vec![];
		}

		self.reset();
	}

	/// private function to delete a byte. Don't add the value to `self.history` use `delete_byte()` instead
	fn remove_byte(&mut self, address: u64) {
		if self.file_size == 0 {
			self.add_error_message(
				WarningLevel::Info, 
				"Can not remove bytes in an empty file".to_string()
			);
			return;
		}

		let real_address = self.get_real_address(address);

		match real_address {
			// If there is no entry, add one to self.modified_bytes
			Addr::FileAddress(addr) => {
				self.modified_bytes.insert(addr, Changes::Deleted);
			},

			// If the byte is part of a vector of inserted bytes, delete
			// the entry in the vector of inserted bytes
			Addr::InsertedAddress(Inserted {
					vector_address, offset_in_vector 
				}) => {
					let changes = self.modified_bytes.get_mut(&vector_address).unwrap();

					match changes {
						Changes::Deleted => { panic!("We can't delete a deleted bytes")},
						Changes::Insertion(inserted_bytes) => {
							inserted_bytes.remove(offset_in_vector as usize);

							// handle the case where have remove every bytes
							// of the vector of inserted bytes
							if inserted_bytes.len() == 0 {
								*changes = Changes::Deleted;
							}
						}
					}
			}
		}

		self.file_size -= 1;
	}

	/// public function to delete a byte. Add the current value to self.history
	pub fn delete_byte(&mut self, address: u64) {
		
		// add the current value self.history
		match self.read_byte_addr(address) {
			Ok(value) => self.history.push((Modification::Deletetion, address, Some(value))),
			Err(_e) => {
				self.add_error_message(
					WarningLevel::Error,
					format!("No byte to delete at 0x{:x}", address)
				);
				// Avoid calling remove_byte() and entering an incoherent state by returning now
				return;
			}
		}

		self.remove_byte(address);
	}

	/// add the Modification of `address` to `self.history`
	fn add_to_history(&mut self, modif: Modification, address: u64) {

		match modif {
			Modification::Modification => {
				match self.read_byte_addr(address) {

					Ok(value) => self.history.push((modif, address, Some(value))),
					Err(e) => self.add_error_message(
							WarningLevel::Warning,
							format!("Could not backup byte at address 0x{:x}: {}", address, e))
				}		
			},
			Modification::Insertion => {
				self.history.push((modif, address, None));
			},
			Modification::Deletetion => {

				match self.read_byte_addr(address) {
					Ok(value) => self.history.push((modif, address, Some(value))),
					Err(e) => self.add_error_message(
						WarningLevel::Warning,
						format!("Could not backup byte at address 0x{:x}: {}", address, e)
					)
				}
			}
		}
	}

	/// restore the last edited byte from self.history
	pub fn undo(&mut self) {
		
		// get value from self.history
		let (modification, addr, previous_value) = match self.history.pop() {
			None => { return }, // we don't have any value in the history
			Some(history_entry) => { history_entry }
		};


		match modification {
			Modification::Modification => {
				
				// Add the current value to self.history_redo
				let current_val = self.read_byte_addr(addr).unwrap();
				self.history_redo.push((Modification::Modification, addr, Some(current_val)));

				// For modification, we should always have a value, so we can unwrap()
				let previous_value = previous_value.unwrap();

				// if the `char` restored is the second `char` of the byte, set the cursor to the
				// second `char` else set the cursor to the first char
				if current_val & 0b11110000 == previous_value & 0b11110000 {
					self.cursor_jump_to(addr * 2 + 1);
				} else {
					self.cursor_jump_to(addr * 2);
				}

				// restore the previous value
				self.write_byte(addr, previous_value, Mode::Overwrite)
					.unwrap_or_else(|_err| {
				 		self.add_error_message(
				 			WarningLevel::Error,
				 			"Undo: Failed to restore byte".to_string()
				 		)
					});
			},
			Modification::Deletetion => {
				// Add to redo history
				self.history_redo.push((Modification::Insertion, addr, None));

				// For modification, we should always have a previous_value
				let previous_value = previous_value.unwrap();

				// restore the previous value
				self.write_byte(addr, previous_value, Mode::Insert)
					.unwrap_or_else(|_err| {self.add_error_message(
								WarningLevel::Error,
								"Undo: Failed to restore byte".to_string()
							)});
				
				// move our cursor to the changed location
				self.jump_to(addr);
			},
			Modification::Insertion => {
				// Add the current value to self.history_redo
				let current_val = self.read_byte_addr(addr).unwrap();
				self.history_redo.push((Modification::Deletetion, addr, Some(current_val)));

				// Delete the previously inserted byte
				self.remove_byte(addr);

				// Move our cursor at the address of the delete byte
				self.jump_to(addr);
			},
		}
	}

	/// restore the last edited byte from self.history
	/// There is probably a smart way to do this, but at this point this is just a copy
	/// of the undo() function where `self.history` and `self.history_redo` are inverted
	pub fn redo(&mut self) {
		
		// get value from self.history
		let (modification, addr, previous_value) = match self.history_redo.pop() {
			None => { return }, // we don't have any value in the history
			Some(history_entry) => { history_entry }
		};


		match modification {
			Modification::Modification => {
				
				// Add the current value to self.history
				let current_val = self.read_byte_addr(addr).unwrap();
				self.history.push((Modification::Modification, addr, Some(current_val)));

				// For modification, we should always have a value, so we can unwrap()
				let previous_value = previous_value.unwrap();

				// if the `char` restored is the second `char` of the byte, set the cursor to the
				// second `char` else set the cursor to the first char
				if current_val & 0b11110000 == previous_value & 0b11110000 {
					self.cursor_jump_to(addr * 2 + 1);
				} else {
					self.cursor_jump_to(addr * 2);
				}

				// restore the previous value
				self.write_byte(addr, previous_value, Mode::Overwrite)
					.unwrap_or_else(|_err| {
				 		self.add_error_message(
				 			WarningLevel::Error,
				 			"Undo: Failed to restore byte".to_string()
				 		)
					});
			},
			Modification::Deletetion => {
				// Add to redo history
				self.history.push((Modification::Insertion, addr, None));

				// For modification, we should always have a previous_value
				let previous_value = previous_value.unwrap();

				// restore the previous value
				self.write_byte(addr, previous_value, Mode::Insert)
					.unwrap_or_else(|_err| {self.add_error_message(
								WarningLevel::Error,
								"Undo: Failed to restore byte".to_string()
							)});
				
				// move our cursor to the changed location
				self.jump_to(addr);
			},
			Modification::Insertion => {
				// Add the current value to self.history
				let current_val = self.read_byte_addr(addr).unwrap();
				self.history.push((Modification::Deletetion, addr, Some(current_val)));

				// Delete the previously inserted byte
				self.remove_byte(addr);

				// Move our cursor at the address of the delete byte
				self.jump_to(addr);
			},
		}

		// move the cursor after our restored byte
		self.change_cursor(1);
	}

	/// undo all changes using self.history
	pub fn undo_all(&mut self) {
		while self.history.len() > 0 {
			self.undo();
		}
	}

	/// Return the name of the file we are editing
	pub fn filename(&self) -> String {
		match &self.file_path.split('/').last() {
			Some(filename) => {filename.to_string()},
			None => {self.file_path.clone()}
		}
	}

	/// Tells us if we  have some unsaved insertion and deletions
	fn no_insertion_or_deletion(&self) -> bool {
		
		// We go over the list of modified bytes to see if there are insertions
		// or deletion.

		// The code doesn't handle the case where the final file has the same
		// number of bytes because we have the same number of insertion and
		// deletions
		for (_, change) in self.modified_bytes.iter() {
			match change {
				Changes::Deleted => return false,
				Changes::Insertion(vector) => {
					if vector.len() != 1 {
						return false
					}
				}
			}
		}
		true
	}

	/// Save by overwritting the file.
	/// We do this only if there are no insertions / deletions
	fn save_by_overwritting(&mut self) -> Result<(), Error> {
		
		let mut writer = BufWriter::new(&self.file);
		
		// Because no_insertion_or_deletion() has return true.
		// We know that each vector has only one element.
		
		// We iter or self.modified_bytes and apply each modification.
		while let Some((addr, change)) = self.modified_bytes.pop_first() {

			let new_byte_value = match change {
				Changes::Deleted => unreachable!("We should only have insertion if we save by overwriting"),
				Changes::Insertion(vector) => {
					vector[0]
				}
			};

			writer.seek(SeekFrom::Start(addr))?;
			writer.write(&[new_byte_value])?;
		}

		Ok(())
	}

	fn save_with_temporary_file(&mut self) -> Result<(), Error> {
		/* Create a temporary file to do our writes */
		let temp_filename = format!("{}.hextazy", self.file_path);
		
		let temp_file = File::create(&temp_filename)?;
		let mut writer = BufWriter::new(temp_file);

		for i in 0..self.file_size {
			let byte = self.read_byte_addr(i)?;
			writer.write(&[byte])?;
		}

		writer.flush()?;

		// Replace our file with the temporary file
		std::fs::rename(&temp_filename, &self.file_path)?;
		self.modified_bytes.clear(); // Remove all our modifications
		std::fs::remove_file(temp_filename).ok(); // Just ignore if we have an error here

		// Reload our file, we ought to do a function for this instead of
		// copy pasting.

		// Open the file in Read / Write mode
		let file_openner = OpenOptions::new()
			.read(true)
			.write(true)
			.open(&self.file_path);

		// If we can't open it Read / Write.
		// Open it as Read Only.
		let f = file_openner.unwrap_or_else(|error| {
			if error.kind() == ErrorKind::PermissionDenied {
				OpenOptions::new()
				.read(true)
				.open(&self.file_path).
				expect("Could not open file")
			} else if error.kind() == ErrorKind::NotFound {
				// TODO: create the file if it doesn't exists
				reset_terminal().expect("Failed to reset the terminal. Use the `reset` command in your terminal.");
				println!("Error: file not found.");
				exit(1);
			} else {
				panic!("Problem opening the file: {:?}", error);
			}
		});
		self.file = f;

		self.reader = BufReader::new(self.file.try_clone()?);

		Ok(())
	}

	/// written all the modified bytes into the file.
	pub fn save_to_disk(&mut self) -> Result<(), Error> {

		// If there are only modification (no insertion / deletion)
		// we can replace the bytes directly in the file
		match self.no_insertion_or_deletion() {
			true  => self.save_by_overwritting(), 
			false => self.save_with_temporary_file(),
		}
	}

	/// Read one byte
	pub fn read_byte(&mut self) -> Option<u8> {
		let addr = self.last_address_read;
		self.last_address_read += 1;

		match self.read_byte_addr(addr) {
			Ok(val) => {
				Some(val)
			},
			Err(_e) => None
		}
	}

	// read 16 bytes, and return the length
	pub fn read_16_length(&mut self) -> (Vec<u8>, usize) {
		let mut bytes: Vec<u8> = vec![];

		let mut current_address = self.last_address_read;

		// Return immediatly if we have reached end of file
		if current_address == self.file_size {
			return (vec![], 0);
		}
		
		for _ in 0..16 {
			// return byte from the file, or modified byte from `self.modified_bytes`
			match self.read_byte_addr(current_address) {
				Ok(val) => bytes.push(val),
				Err(e) if e.kind() == ErrorKind::UnexpectedEof => { // we have reached end of file
					break;
				},
				_ => self.add_error_message(
					WarningLevel::Warning,
					format!("Could not backup byte at address 0x{:x}", current_address)
				),
			}
			current_address += 1;
		}

		self.last_address_read = current_address;
		let len = bytes.len();
		(bytes, len)
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
		if self.offset > self.file_size.saturating_sub(0x10) {

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

		// We use this so that in insertion mode we have the possibility
		// to write at the end of the file.
		let insertion_mode = match self.mode {
			Mode::Insert => 1,
			Mode::Overwrite => 0
		};

		let end_of_file = self.file_size * 2 + insertion_mode;

		// check if the new cursor address is longer than the file
		// (file_size * 2) - 1 because we have 2 chars for each hex number.
		if self.cursor.wrapping_add_signed(direction.into()) > end_of_file.saturating_sub(1) {

			//  + (self.cursor % 0x20) = stay on the same column

			// case where the last line is an exact fit
			if end_of_file % 0x20 == 0 {
				self.cursor = end_of_file.saturating_sub(0x20) + (self.cursor % 0x20); // stay on the same column
			}

			// we have an incomplete last line
			else {
				let last_line_length = end_of_file % 0x20;
				let column_of_cursor = self.cursor % 0x20;
							
				let start_of_last_line = end_of_file - (end_of_file % 0x20);

				// cursor is on the last line
				if column_of_cursor < last_line_length {
					self.cursor = start_of_last_line + (self.cursor % 0x20);
				}
				
				// cursor is on the line just before the last, but can't go down
				// without exceeding file size
				else {
					self.cursor = start_of_last_line.saturating_sub(0x20) + (self.cursor % 0x20);
				}

			}

			if direction == 0x10 { // If we are moving the cursor down
				self.change_offset(0x10); // move the view one line down
			}
			return;
		}

		self.cursor = self.cursor.saturating_add_signed(direction.into());

		// case where the cursor is before what the screen displays
		if self.cursor / 2 < self.offset {
			self.offset = (self.cursor / 2) - (self.cursor / 2 % 0x10);
		}

		// case where the cursor is below what the screen displays
		if self.cursor / 2 > self.offset + u64::from(self.lines_displayed) * 0x10 {
			let cursor_line_start = (self.cursor / 2)  - (self.cursor / 2 % 0x10) ;
			self.offset = cursor_line_start.saturating_sub(u64::from(self.lines_displayed - 1) * 0x10);
		}
	}

	/// move the selected bytes to the direction
	pub fn move_selection(&mut self, direction: i64) {
		// We don't have any selected bytes, just move the cursor
		if self.selection_start.is_none() {
			self.change_cursor(direction / 2);
			return;
		}

		// Start and end of the our selected bytes
		let selection = self.selection_start.unwrap();
		let start = min(self.cursor, selection);
		let end = max(self.cursor, selection);

		// We can't move the selection more to the `left`
		if direction.saturating_add_unsigned(start) < 0 {
			return;
		}

		let end_of_file = self.file_size * 2;

		if end.saturating_add_signed(direction) > end_of_file {
			return;
		}

		self.cursor = self.cursor.saturating_add_signed(direction);
		self.selection_start = Some(self.selection_start.unwrap().saturating_add_signed(direction));
	}

	/// use to jump directly at an address, and move the interface accordingly
	pub fn jump_to(&mut self, new_address: u64) {
		self.cursor_jump_to(new_address * 2);
	}

	/// use to jump directly at an address (using a cursor address), and move the interface accordingly
	pub fn cursor_jump_to(&mut self, new_cursor_address: u64) {
		let mut new_address = new_cursor_address / 2;

		// check that the address is not bellow the file
		if new_address > self.file_size {
			new_address = self.file_size-1;
		}

		// if address is not on the page currently displayed,
		// jump on the address and display it in the middle of the page
		if (new_address < self.offset) || new_address > self.offset + u64::from(self.lines_displayed)*0x10 - 1{
			self.cursor = new_cursor_address;

			// cursor should be in the middle of the screen:
			// self.offset = self.cursor - (half the screen)
			let lines_before_cursor = (u64::from(self.lines_displayed)/2) * 0x10;
			self.offset = u64::saturating_sub(new_address, lines_before_cursor);

			self.offset = self.offset - (self.offset %0x10); // align self.offset to 0x10
		
		// the new address is displayed on the screen, just move the cursor
		} else {
			self.cursor = new_cursor_address;
		}

		// if cursor is after the end of the file. Go to the end
		let mut file_end = (self.file_size * 2).saturating_sub(1); // size * 2 - 1 but without going under 0
		if self.mode == Mode::Insert {
			file_end += 1;
		}

		if self.cursor > file_end {
			self.cursor = file_end;
		}
	}

	/// Determine if the given address is part of a search result
	pub fn is_searched(&self, address: u64) -> bool {
		if let Some(search_results) = &self.search_results {
			for result in search_results {
				
			}		
		} else {
			return false
		}

		let cursor = self.cursor;

		if let Some(selection) = self.selection_start {
			let start = if selection < cursor { selection} else { cursor };
			let end = if selection < cursor { cursor} else { selection };

			if start <= address*2 && address*2 <= end {
				return true
			}
		}
		false
	}

	/// Determine if the given address is selected
	pub fn is_selected(&self, address: u64) -> bool {
		let cursor = self.cursor;

		if let Some(selection) = self.selection_start {
			let start = if selection < cursor { selection} else { cursor };
			let end = if selection < cursor { cursor} else { selection };

			if start <= address*2 && address*2 <= end {
				return true
			}
		}
		false
	}

	/// Determine if the given address is selected
	pub fn is_selected_cursor(&self, address: u64) -> bool {

		if let Some(selection) = self.selection_start {
			if selection == address {
				return false;
			}
		}

		let address = match self.selection_start {
			Some(selection) if (address > selection)  && (address % 2 == 0)=> address.saturating_add(1),
			Some(selection) if (address < selection) && (address % 2 == 1)=> address.saturating_sub(1),
			_ => address,
		};

		let cursor = self.cursor;

		if let Some(selection) = self.selection_start {
			let start = if selection < cursor { selection} else { cursor };
			let end = if selection < cursor { cursor} else { selection };

			if start <= address && address <= end {
				return true
			}
		}
		false
	}

	/// Return the bytes currently selected
	pub fn get_selected_bytes(&mut self) -> Option<Vec<u8>> {
		if self.selection_start.is_none() {
			return None;
		}

		let (start_cursor, end_cursor) = match self.selection_start.unwrap() < self.cursor {
			true  => (self.selection_start.unwrap(), self.cursor + 1),
			false => (self.cursor, self.selection_start.unwrap()),
		};

		let start = start_cursor / 2;
		let end = end_cursor / 2;

		let mut selected_bytes: Vec<u8> = vec![];
		
		for addr in start..end {
			if let Ok(byte) = self.read_byte_addr(addr) {
				selected_bytes.push(byte);
			} else {
				self.add_error_message(WarningLevel::Error, format!("Could not read selected byte at address {:x}", addr));
				break;
			}
		}

		Some(selected_bytes)
	}

	// #[allow(unused)]
	// pub fn add_to_search_results(&mut self, result_address: u64, query_len: usize) {
	// 	if let Some(ref mut search_results) = &mut self.search_results {
	// 		search_results.match_addresses.push(result_address);
	// 	} else {
	// 		self.jump_to(result_address);
	// 		self.search_results = Some(SearchResults{
	// 			match_addresses: vec![result_address],
	// 			query_length: query_len
	// 		})
	// 	}
	// }

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
		
		for (addr, _) in &search_results.match_addresses {
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
		
		for (addr, _) in (&(&search_results).match_addresses).into_iter().rev() {
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
			// if we have no changes exit, else show the exit popup
			if self.modified_bytes.is_empty() {
				reset_terminal().expect("Failed to reset the terminal. Use the `reset` command in your terminal.");
				exit(0);
			} else {
				self.editor_mode = CurrentEditor::ExitPopup;
			}
		}

		// exit - :q!
		let regex_q = Regex::new(r"^:\s?+q!\s?+$").unwrap();
		if regex_q.is_match(command) {
			reset_terminal().expect("Failed to reset the terminal. Use the `reset` command in your terminal.");
			exit(0);
		}

		// command is hex address (:0x...), we jump to this address
		let hexnum_regex = Regex::new(r"^:\s?+0[xX][0-9a-fA-F]+$").unwrap();
		if hexnum_regex.is_match(command) {

			// strip spaces and the 0x at the start
			command.remove(0); // remove ':' at the start

			// remove the 0x or 0X at the start
			let command =
			if let Some(command) = command.trim().strip_prefix("0x") {
				command
			} else {
				command.trim().strip_prefix("0X").unwrap()
			};

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
			self.add_error_message(WarningLevel::Info, format!("Searched: {}", &command));


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
							"Error: ascii search failed".to_string()
						);
					},
					Ok(Some(search_results)) => {
						self.search_results = Some(search_results);
						self.go_to_next_search_result();
					},
					Ok(None) => {self.search_results = None}
				};
			} else {
				self.add_error_message(WarningLevel::Info, "Hextazy can only search ascii".to_string());
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

		// Interface customization
		// Hide the infobar
		if command == ":hide infobar" || command == ":hexyl" {
			self.show_infobar = false;
		}

		// Hide the infobar
		else if command == ":show infobar" || command == ":!hexyl" {
			self.show_infobar = true;
		}

		// Switch Mode: overwrite, insert
		if command == ":i" || command == ":insert" || command == ":mode insert" {
			self.mode = Mode::Insert
		}

		if command == ":o" || command == ":overwrite" || command == ":mode overwrite" {
			self.mode = Mode::Overwrite
		}

		if command == ":w" {
			match self.save_to_disk() {
				Ok(()) => self.add_error_message(
					WarningLevel::Info,
					"Changes saved successfully".to_string()
				),
				Err(_e) => self.add_error_message(
					WarningLevel::Error,
					"Failed to save the changes".to_string()
				)
			}
		}
	}
}