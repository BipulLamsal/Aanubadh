use docx_rs::{self, DocumentChild, Docx, ParagraphChild, RunChild};
use std::{
    fs::File,
    io::{BufReader, Read},
    sync::Arc,
};
use tokio::sync::Semaphore;

use tmt::{send_translation_request, types::request::Language};

const SEP: &str = " Aledrip ";

struct SendPtr(*mut String);
// SAFTEY: they are send seperately for each threads so its safe for this case
unsafe impl Send for SendPtr {}

struct DocXReader {
    doc: Docx,
    chunk_size: usize,
    concurrent_request_size: usize,
    src: Language,
    tgt: Language,
}
impl DocXReader {
    pub fn from_reader<T: Read>(
        reader: T,
        src: Language,
        tgt: Language,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer = Vec::new();
        let _ = BufReader::new(reader).read_to_end(&mut buffer);
        let doc = docx_rs::read_docx(&buffer)?;
        Ok(DocXReader {
            doc,
            chunk_size: 10,
            concurrent_request_size: 5,
            src,
            tgt,
        })
    }

    pub async fn start_walk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut chunk: Vec<SendPtr> = Vec::with_capacity(self.chunk_size);
        // let mut con_request_handler: Vec<Vec<*mut String>> =
        //    Vec::with_capacity(self.concurrent_request_size);

        let mut handler = Vec::new();

        let semaphore = Arc::new(Semaphore::new(self.concurrent_request_size));

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
                    // we keep of adding until we hit the chunk size limit
                    chunk.push(SendPtr(&mut txt.text as *mut String));

                    // if we hit the limit
                    // its the time to combine the chunks and make it as string with a sep, run a thread able to call tranlsation
                    if chunk.len() >= self.chunk_size {
                        // we reset the chunk for next iteration
                        let ret =
                            std::mem::replace(&mut chunk, Vec::with_capacity(self.chunk_size));

                        handler.push(spawn_task_for_chunk(
                            ret,
                            semaphore.clone(),
                            self.src,
                            self.tgt,
                        ));
                    }
                }
            }
        }

        // last chunk logic
        if !chunk.is_empty() {
            handler.push(spawn_task_for_chunk(
                chunk,
                semaphore.clone(),
                self.src,
                self.tgt,
            ));
        }

        for handle in handler {
            handle
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

fn build_payload(chunk: &[SendPtr]) -> String {
    unsafe {
        let texts: Vec<&str> = chunk.iter().map(|p| (*p.0).as_str()).collect();
        texts.join(SEP)
    }
}
fn parse_response(translated: &str) -> Vec<&str> {
    // this is an api hack that  preserves Aledrip to एलेड्रिप in nepali, so spsplitting on that
    translated.split("एलेड्रिप").map(|s| s.trim()).collect()
}

fn spawn_task_for_chunk(
    chunk: Vec<SendPtr>,
    sem: Arc<Semaphore>,
    src: Language,
    tgt: Language,
) -> tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    tokio::spawn(async move {
        // only _ would drop but with names we can make it live till async block
        let _check = sem.acquire().await?;
        let payload = build_payload(&chunk);
        let response = send_translation_request(&payload, src, tgt).await?;
        let translated = &response.output;

        let parts = parse_response(translated);

        // write directly into doc memory
        unsafe {
            for (ptr, part) in chunk.iter().zip(parts.iter()) {
                let s = &mut *ptr.0;
                s.clear();
                s.push_str(part);
            }
        }

        Ok(())
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("/home/bedgirb/Downloads/english.docx")?;
    let mut docx_reader = DocXReader::from_reader(file, Language::English, Language::Nepali)?;
    docx_reader.start_walk().await?;
    let output = File::create("output.docx")?;
    docx_reader.doc.build().pack(output)?;

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
