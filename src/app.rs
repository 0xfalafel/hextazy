use std::io::prelude::*;
use std::io::Error;
use std::io::{BufReader, ErrorKind};
use std::io::BufWriter;
use std::fs::{File, OpenOptions};

pub struct App {
	pub filename: String,	// 
	pub reader: BufReader<File>,
	pub file: File,
	pub offset: u64,		// where are we currently reading the file
	pub file_size: u64,		// size of the file
	pub cursor: u64			// position of the cursor on the interface
}

impl App {

	pub fn new(filename: String) -> Result<App, std::io::Error> {
		let f = File::open(&filename)?;

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
			cursor: 0
		};
		Ok(app)
	}

	// reset the "file cusor" to it's intial position (the app.offset value)
	pub fn reset(&mut self) {
		let seek_addr = std::io::SeekFrom::Start(self.offset);
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

	pub fn write(&mut self, offset: u64, value: u8) {
		let seek_addr = std::io::SeekFrom::Start(offset);
		self.file.seek(seek_addr);
		self.file.write(b"\x42");

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
		// check is result is bellow 0
		if direction.wrapping_add_unsigned(self.offset.into()) < 0 {
			self.offset = 0;
			return;
		}

		// check if result is longer than the file
		if self.offset.wrapping_add_signed(direction.into()) > self.file_size {
			return;
		}

		self.offset = self.offset.wrapping_add_signed(direction.into());
	}

	// self.cursor = self.cursor + direction
	// but we check if the address is bellow 0 or lager than the file
	pub fn change_cursor(&mut self, direction:i64){
		// check the address is bellow 0
		if direction.wrapping_add_unsigned(self.cursor.into()) < 0 {
			self.cursor = 0;
			return;
		}

		// check if the new cursor address is longer than the file
		// (file_size * 2) - 1 because we have 2 chars for each hex number.
		if self.cursor.wrapping_add_signed(direction.into()) > (self.file_size * 2) - 1 {
			return;
		}

		self.cursor = self.cursor.wrapping_add_signed(direction.into());		
	}

}