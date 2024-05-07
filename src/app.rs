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
}