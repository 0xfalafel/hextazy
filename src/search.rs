use crate::app::App;
use std::fs::File;
use std::io::{Seek, SeekFrom, BufReader, Read};

#[derive(PartialEq)]
pub struct SearchResults {
	pub match_addresses: Vec<u64>, // vector of addresses where the text searched has been found
	pub query_length: usize			// len of the searched text, used to highlight search results
}

/// use to append an address to the search results in the different search functions
fn add_to_search_results(address: u64, searchresults: Option<SearchResults>, len: usize) -> Option<SearchResults> {
    let searchresults = match searchresults {
        None => {
            SearchResults {
                match_addresses: vec!(address),
                query_length: len
            }
        },
        Some(mut search_results) => {
            search_results.match_addresses.push(address);
            search_results
        }
    };

    Some(searchresults)
}

/// search an ascii string in a File. Return a SearchResult containing the addresses found.
/// The search is case sensitive
pub fn search_ascii(mut file: File, search: &str) -> Result<(Option<SearchResults>), std::io::Error> {
    // create a new file reader and buffer, so we don't disrupt our display loop with reads() and seek()
    // let mut file = file.try_clone().unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut reader = BufReader::new(file);
    
    let first_char = search.as_bytes()[0];
    let mut buf: [u8; 1] = [0; 1]; // apparently we are supposed to use a buffer, don't juge me
    
    let mut search_results: Option<SearchResults> = None;
    let search_len = search.len();

    // read the whole file, and see if a byte match the first char of the search
    // if it's a match, we go in a more in depth search
    loop {
        let read_len = reader.read(&mut buf)?;
        
        if read_len == 0 { // didn't read anything, must be eof
            return Ok(search_results);
        }
    
        // we have a match !
        if buf[0] == first_char as u8 {
            
            // store where we found the first char
            let match_address = reader.stream_position().unwrap() - 1;
            
            // check if we have really found the string searched
            let found_string = self::is_ascii_string_matched(& mut reader, search);
            
            if found_string { // that's our search result
                search_results = add_to_search_results(match_address, search_results, search_len);
            }
    
            // continue the search
            reader.seek(SeekFrom::Start(match_address+1))?;
        }
    }

    Ok(search_results)
}

/// used by search_ascii(), check if the rest of the ascii string searched is matched
fn is_ascii_string_matched(reader: &mut BufReader<File>, search: &str) -> bool {
    let search_len = search.len();
    let mut buf: [u8; 1] = [0; 1];
    
    // check if the rest of the string also matches
    for i in 1..search_len {
        
        // read one char
        match reader.read(&mut buf) {
            Err(e) => {return false;}
            Ok(len) if len == 0 => {return false;}
            _ => {}
        }
        
        let c = buf[0];
        
        if c != search.as_bytes()[i] {
            return false;
        }
    }
    
    true
}

/// search hex values in a File. Return a SearchResult containing the addresses found.
pub fn search_hex(app: &mut App, mut file: File, search: Vec<u8>) -> Result<(), std::io::Error> {

    // create a new file reader and buffer, so we don't disrupt our display loop with reads() and seek()
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut reader = BufReader::new(file);
    
    let first_byte = search[0];
    let mut buf: [u8; 1] = [0; 1]; // apparently we are supposed to use a buffer, don't juge me
    
    
    // read the whole file, and see if a byte match the first byte of the search
    // if it's a match, we go in a more in depth search
    loop {
        let read_len = reader.read(&mut buf)?;
        
        if read_len == 0 { // didn't read anything, must be eof
            return Ok(());
        }
    
        // we have a match !
        if buf[0] == first_byte {
            
            // store where we found the first char
            let match_address = reader.stream_position().unwrap() - 1;
            
            // check if we have really found the bytes searched
            let found_search = self::is_byte_search_matched(& mut reader, &search);
            
            if found_search { // that's our search result
                app.add_to_search_results(match_address, search.len())
            }
        
            // continue the search
            reader.seek(SeekFrom::Start(match_address+1))?;
        }
    }

    Ok(())
}

fn is_byte_search_matched(reader: &mut BufReader<File>, search: &Vec<u8>) -> bool {
    let search_len = search.len();
    let mut buf: [u8; 1] = [0; 1];
    
    // check if the rest of the string also matches
    for i in 1..search_len {
        
        // read one char
        match reader.read(&mut buf) {
            Err(e) => {return false;}
            Ok(len) if len == 0 => {return false;}
            _ => {}
        }
        
        let c = buf[0];
        
        if c != search[i] {
            return false;
        }
    }
    
    true
}