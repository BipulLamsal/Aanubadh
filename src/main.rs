use docx_rs::{self, DocumentChild, Docx, ParagraphChild, RunChild};
use std::{
    fs::File,
    io::{BufReader, Read},
    sync::{Arc, RwLock},
    thread,
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
struct DocXReader {
    doc: Docx,
    chunk_size: usize,
    concurrent_request_size: usize,
}
impl DocXReader {
    pub fn from_reader<T: Read>(reader: T) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer = Vec::new();
        let _ = BufReader::new(reader).read_to_end(&mut buffer);
        let doc = docx_rs::read_docx(&buffer)?;
        Ok(DocXReader {
            doc,
            chunk_size: 10,
            concurrent_request_size: 5,
        })
    }

    pub fn start_walk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut chunk: Vec<*mut String> = Vec::new();
        let mut con_request_handler: Vec<Vec<*mut String>> =
            Vec::with_capacity(self.concurrent_request_size);

        for child in &mut self.doc.document.children {
            let para = match child {
                DocumentChild::Paragraph(p) => &mut **p,
                _ => continue,
            };
            for pchild in &mut para.children {
                let run = match pchild {
                    ParagraphChild::Run(r) => &mut **r,
                    _ => continue,
                };
                for rchild in &mut run.children {
                    let txt = match rchild {
                        RunChild::Text(t) => t,
                        _ => continue,
                    };
                    if txt.text.is_empty() {
                        continue;
                    }
                    println!("{:?}", txt.text);
                    // we keep of adding until we hit the chunk size limit
                    chunk.push(&mut txt.text as *mut String);

                    // if we hit the limit
                    // its the time to combine the chunks and make it as string with a sep, run a thread able to call tranlsation
                    if chunk.len() >= self.chunk_size {
                        // we push this to our request_handler
                        con_request_handler.push(chunk);
                        // and we reset the chunk for next iteration
                        chunk = Vec::new();
                    }
                }
            }
            // last chunk logic
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/home/bedgirb/Downloads/test.docx")?;
    let mut docx_reader = DocXReader::from_reader(file)?;
    docx_reader.start_walk()?;
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
