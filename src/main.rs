use docx_rs::{self, Document, DocumentChild, Docx, ParagraphChild, RunChild};
use std::{
    fs::File,
    io::{BufReader, Read},
};

const sep: &str = "@#@";

fn walk_run_children(run_children: &mut Vec<RunChild>) {
    for child in run_children {
        match child {
            RunChild::Text(txt) => {
                txt.text.clear();
                txt.text.push_str("World");
            }
            _ => {}
        }
    }
}
#[derive(Default)]
struct DocXReader(Docx);

impl DocXReader {
    pub fn new<T: Read>(self, reader: T) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer = Vec::new();
        let _ = BufReader::new(reader).read_to_end(&mut buffer);
        let doc = docx_rs::read_docx(&buffer)?;
        Ok(DocXReader(doc))
    }

    pub fn start_walk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for child in &mut self.0.document.children {
            match child {
                DocumentChild::Paragraph(p) => {
                    let paragraph = &mut **p;
                    let paragraph_children = &mut paragraph.children;
                    Self::walk_paragraph_children(paragraph_children)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn walk_paragraph_children(
        paragraph_children: &mut Vec<ParagraphChild>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for child in paragraph_children {
            match child {
                ParagraphChild::Run(run_data) => Self::walk_run_children(&mut run_data.children)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn walk_run_children(
        run_children: &mut Vec<RunChild>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let text_vec: Vec<&str> = Vec::new();
        for child in run_children {
            match child {
                RunChild::Text(txt) => {}
                _ => {}
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/home/bedgirb/Downloads/test.docx")?;

    /*
    let file = File::create("output.docx")?;

    let pdf_file = File::open("/home/bedgirb/Downloads/test.pdf")?;
    let mut pdf_buffer = Vec::new();
    let _ = BufReader::new(pdf_file).read_to_end(&mut pdf_buffer);
    let mut pdf_doc = PdfDocument::from_bytes(pdf_buffer)?;
    let extractor = pdf_doc.extract_words(0);
    println!("{:?}", extractor);
    doc.build().pack(file)?;
    */

    Ok(())
}
