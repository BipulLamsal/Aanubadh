use std::{
    fs::File,
    io::{BufReader, Read},
};

use docx_rs::{self, DocumentChild, ParagraphChild, RunChild};

fn walk_run_children(run_children: &mut Vec<RunChild>) {
    for child in run_children {
        match child {
            RunChild::Text(txt) => {
                println!("{:?}", txt.text);
            }
            _ => {}
        }
    }
}

fn walk_paragraph_children(paragraph_children: &mut Vec<ParagraphChild>) {
    for child in paragraph_children {
        match child {
            ParagraphChild::Run(run_data) => walk_run_children(&mut run_data.children),
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/home/bedgirb/Downloads/test.docx")?;
    let mut buffer = Vec::new();
    let _ = BufReader::new(file).read_to_end(&mut buffer);
    let mut doc = docx_rs::read_docx(&buffer)?;

    for child in &mut doc.document.children {
        match child {
            DocumentChild::Paragraph(p) => {
                let paragraph = &mut **p;
                let paragraph_children = &mut paragraph.children;
                walk_paragraph_children(paragraph_children);
            }
            _ => {}
        }
    }

    Ok(())
}
