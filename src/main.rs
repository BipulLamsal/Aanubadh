use docx_rs::*;

use std::{
    fs::File,
    io::Result,
    io::{BufReader, Read},
};

use docx_rs::read_docx;
use tmt::{
    parser::{Parser, document::DocumentParser},
    types::state::{FormatType, PipeLineState},
};

struct Extractor {
    state: PipeLineState,
    ext_type: FormatType,
}

impl Extractor {
    fn extract_from_path(&self, path: &str) -> Result<()> {
        let file = File::open(path)?;
        self.extract_from_reader(&file);
        Ok(())
    }

    fn extract_from_reader(&self, reader: impl Read) {
        let buf_reader = BufReader::new(reader);
        let _ = match self.ext_type {
            FormatType::Docx => DocumentParser.parse(&buf_reader.buffer()),
            _ => todo!(),
        };
    }
}

fn parse_docx() {}

fn main() {
    let file = File::open("/home/bedgirb/Downloads/test.docx").unwrap();
    let mut reader = BufReader::new(file);
    let mut output: Vec<u8> = Vec::new();
    reader.read_to_end(&mut output).unwrap();
    let docsx = read_docx(&output).unwrap();
    println!("{:#?}", docsx);
}
