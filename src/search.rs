use std::fs::File;
use std::io::{BufReader, Error, Read, Seek, SeekFrom};

#[derive(PartialEq)]
pub struct SearchResults {
	pub match_addresses: Vec<u64>, // vector of addresses where the text searched has been found
	pub query_length: usize			// len of the searched text, used to highlight search results
}

pub fn convert_hexstring_to_vec(hex_string: &str) -> Vec<u8> {
    let mut vec: Vec<u8> = vec![];

    // convert the searched hex string to a vector of u8
    let searched_len = hex_string.len();

    for i in (0..searched_len).step_by(2) {
        let hex_byte = &hex_string[i..i+2];
        let byte = u8::from_str_radix(hex_byte, 16).unwrap();

        vec.push(byte);
    }

    vec
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
pub fn search_ascii(mut file: File, search: &str) -> Result<Option<SearchResults>, std::io::Error> {

    // go to the start, so we don't miss any strings
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
            let found_string = self::is_ascii_string_matched(& mut reader, search)?;
            
            if found_string { // that's our search result
                search_results = add_to_search_results(match_address, search_results, search_len);
            }
    
            // continue the search
            reader.seek(SeekFrom::Start(match_address+1))?;
        }
    }

    // TODO, make the search async, and return search_results
    // Ok(search_results)
}

/// used by search_ascii(), check if the rest of the ascii string searched is matched
fn is_ascii_string_matched(reader: &mut BufReader<File>, search: &str) -> Result<bool, Error> {
    let search_len = search.len();
    let mut buf: [u8; 1] = [0; 1];
    
    // check if the rest of the string also matches
    for i in 1..search_len {
        
        // read one char
        match reader.read(&mut buf) {
            Err(e) => {return Err(e);}
            Ok(len) if len == 0 => {return Ok(false);}
            _ => {}
        }
        
        let c = buf[0];
        
        if c != search.as_bytes()[i] {
            return Ok(false);
        }
    }
    
    Ok(true)
}

/// search hex values in a File. Return a SearchResult containing the addresses found.
pub fn search_hex(mut file: File, search: Vec<u8>) -> Result<Option<SearchResults>, std::io::Error> {

    // go to the start of the file, to not miss any bytes
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut reader = BufReader::new(file);
    
    let first_byte = search[0];
    let mut buf: [u8; 1] = [0; 1]; // apparently we are supposed to use a buffer, don't juge me

    let mut search_results: Option<SearchResults> = None;
    let search_len = search.len();
    
    // read the whole file, and see if a byte match the first byte of the search
    // if it's a match, we go in a more in depth search
    loop {
        let read_len = reader.read(&mut buf)?;
        
        if read_len == 0 { // didn't read anything, must be eof
            return Ok(search_results);
        }
    
        // we have a match !
        if buf[0] == first_byte {
            
            // store where we found the first char
            let match_address = reader.stream_position().unwrap() - 1;
            
            // check if we have really found the bytes searched
            let found_search = self::is_byte_search_matched(& mut reader, &search)?;
            
            if found_search { // that's our bytes
                search_results = add_to_search_results(match_address, search_results, search_len);
            }
        
            // continue the search
            reader.seek(SeekFrom::Start(match_address+1))?;
        }
    }

    // TODO, make the search async, and return search_results
    // Ok(search_results)
}

/// used by search_hex(), check if the rest of the hex string searched is matched
fn is_byte_search_matched(reader: &mut BufReader<File>, search: &Vec<u8>) -> Result<bool, Error> {
    let search_len = search.len();
    let mut buf: [u8; 1] = [0; 1];
    
    // check if the rest of the string also matches
    for i in 1..search_len {
        
        // read one char
        match reader.read(&mut buf) {
            Err(e) => {return Err(e);}
            Ok(len) if len == 0 => {return Ok(false);}
            _ => {}
        }
        
        let c = buf[0];
        
        if c != search[i] {
            return Ok(false);
        }
    }
    
    Ok(true)
}

/// search hex values in a File. Return a SearchResult containing the addresses found.
pub fn search_hex_reverse(file: File, search: Vec<u8>) -> Result<Option<SearchResults>, std::io::Error> {

    // let jump reverse the vector, and call search_hex()
    let mut search = search.clone();
    search.reverse();

    search_hex(file, search)
}


/// search both ascii text and bytes in a file. Return a SearchResult with the addresses found
pub fn search_hex_ascii(mut file: File, search_ascii: &str, search_bytes: Vec<u8>) -> Result<Option<SearchResults>, std::io::Error> {

    // create a BufReader with the file, with a cursor at the first byte
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut reader = BufReader::new(file);

    // used to read the file, byte by byte
    let mut buf: [u8; 1] = [0; 1];

    // store the results
    let mut search_results: Option<SearchResults> = None;
    let search_len = search_ascii.len();


    // read the whole file, and see if a byte match the first byte of the search
    // if it's a match, we go in a more in depth search
    let first_byte = search_bytes[0];
    let first_ascii_char = search_ascii.as_bytes()[0];
    
    loop {
        let read_len = reader.read(&mut buf)?;
        
        if read_len == 0 { // didn't read anything, must be eof
            return Ok(search_results);
        }
    
        // we have a match !
        if buf[0] == first_byte {
            
            // store where we found the first char
            let match_address = reader.stream_position().unwrap() - 1;
            
            // check if we have found the hex string
            let found_bytes = self::is_byte_search_matched(& mut reader, &search_bytes)?;
            
            if found_bytes { // that's our bytes
                search_results = add_to_search_results(match_address, search_results, search_len);
            }

            // continue the search
            reader.seek(SeekFrom::Start(match_address+1))?;
        }

        if buf[0] == first_ascii_char as u8 {
            // store where we found the first char
            let match_address = reader.stream_position().unwrap() - 1;

            // check if we have found the ascii string
            reader.seek(SeekFrom::Start(match_address+1))?;
            
            let found_ascii = self::is_ascii_string_matched(& mut reader, &search_ascii)?;
            
            if found_ascii { // that's our bytes
                search_results = add_to_search_results(match_address, search_results, search_len);
            }         
            
            // continue the search
            reader.seek(SeekFrom::Start(match_address+1))?;
        }

    }
    
    // TODO, make the search async, and return search_results
    // Ok(search_results)
}