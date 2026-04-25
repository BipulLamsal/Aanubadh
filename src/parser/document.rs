use crate::{parser::Parser, types::document::Document};
use docx_rs::{DocumentChild, ParagraphChild, read_docx};

pub struct DocumentParser;

impl Parser for DocumentParser {
    fn parse(&self, buffer: &[u8]) -> Document {
        // remove this unwrap
        let reader = read_docx(buffer).unwrap();
        let document = Document::new();

        for child in &reader.document.children {
            match &child {
                DocumentChild::Paragraph(para) => {
                    for item in para.children() {
                        match item {
                            ParagraphChild::Run(run) => todo!(),
                            _ => todo!(),
                        }
                    }
                }
                _ => todo!(),
            }
        }
        println!("{:#?}", reader);
        document
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::{Parser, document::DocumentParser};
    use std::{fs::File, io::BufReader};

    #[test]
    fn test_document_parser() {
        let file = File::open("/home/bedgirb/Downloads/test.docx").unwrap();
        let reader = BufReader::new(file);
        let _ = DocumentParser.parse(reader.buffer());
    }
}
