use std::io::prelude::*;
use std::io::Error;
use std::io::BufReader;
use std::fs::File;

pub struct App {
	pub filename: String,	// 
	pub reader: BufReader<File>,
	pub offset: u64,		// where are we currently reading the file
	pub file_size: u64,		// size of the file
	pub cursor: u64			// position of the cursor on the interface
}

impl App {
	// add code here
	pub fn new(filename: String) -> Result<App, std::io::Error> {
		let f = File::open(&filename)?;
		let size = f.metadata()?.len();
		let app = App {
			filename: filename,
			reader: BufReader::new(f),
			offset: 0,
			file_size: size,
			cursor: 0
		};
		Ok(app)
	}

	// reset the cursor to it's intial position
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
		&self.reader.read(&mut buf);
		buf
	}

	// self.offset = self.offset + direction
	// but we check if the result is bellow 0 or lager than the file
	pub fn change_offset(&mut self, direction:i64) {
		// check is result is bellow 0
		if direction.wrapping_add_unsigned(self.offset.into()) < 0 {
			return;
		}

		// check if result is longer than the file
		if self.offset.wrapping_add_signed(direction.into()) > self.file_size {
			return;
		}

		self.offset = self.offset.wrapping_add_signed(direction.into());
	}
}