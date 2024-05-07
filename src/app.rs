use std::io::prelude::*;
use std::io::Error;
use std::io::BufReader;
use std::fs::File;

pub struct App {
	pub filename: String,	// TODO: replace with file descriptior
	pub file: BufReader<File>,
	pub offset: u64		// where are we currently reading the file
}

impl App {
	// add code here
	pub fn new(filename: String) -> Result<App, std::io::Error> {
		let f = File::open(&filename)?;
		let app = App {
			filename: filename,
			file: BufReader::new(f),
			offset: 0
		};
		Ok(app)
	}

	// reset the cursor to it's intial position
	pub fn reset(&mut self) {
		let seek_addr = std::io::SeekFrom::Start(self.offset);
		self.file.seek(seek_addr);
	}

	// read 8 bytes
	pub fn read_8(&mut self) -> [u8; 8] {
		let mut buf = [0;8];
		&self.file.read(&mut buf);
		buf
	}

	// read 16 bytes
	pub fn read_16(&mut self) -> [u8; 16] {
		let mut buf = [0;16];
		&self.file.read(&mut buf);
		buf
	}
}