use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::docx::translate_docx_via_xml;
use tmt::types::request::Language;

static PDF_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub async fn translate_pdf(
    input_pdf: impl AsRef<Path>,
    output_pdf: impl AsRef<Path>,
    src_lang: Language,
    tgt_lang: Language,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Starting PDF translation pipeline");

    // creating temporary directory to store files
    let temp_dir = tempfile::tempdir()?;
    let job_id = format!("pdf_{}", PDF_COUNTER.fetch_add(1, Ordering::SeqCst));
    
    let pdf_input_path   = input_pdf.as_ref();
    let docx_temp_path   = temp_dir.path().join(format!("{}_temp.docx", job_id));
    let docx_xlated_path = temp_dir.path().join(format!("{}_translated.docx", job_id));
    let pdf_output_path  = output_pdf.as_ref();

    // converting pdf to docx to preserve the layout exactly
    let py_pdf2docx = format!(
        "from pdf2docx import Converter; \
         cv = Converter(r'{}'); cv.convert(r'{}'); cv.close()",
        pdf_input_path.to_str().unwrap(), docx_temp_path.to_str().unwrap()
    );
    let s1 = std::process::Command::new("python")
        .args(["-c", &py_pdf2docx]).status()?;
        
    if !s1.success() {
        return Err("pdf2docx failed".into());
    }

    // translating text and text boxes using xml pipeline
    let docx_bytes = std::fs::read(&docx_temp_path)?;
    let translated_docx = translate_docx_via_xml(&docx_bytes, src_lang, tgt_lang, true).await?;
    std::fs::write(&docx_xlated_path, &translated_docx)?;

    // generating final translated pdf file
    let py_docx2pdf = format!(
        "from docx2pdf import convert; convert(r'{}', r'{}')",
        docx_xlated_path.to_str().unwrap(), pdf_output_path.to_str().unwrap()
    );
    let s2 = std::process::Command::new("python")
        .args(["-c", &py_docx2pdf]).status()?;
        
    if !s2.success() {
        return Err("docx2pdf failed".into());
    }

    Ok(())
}
