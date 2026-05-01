mod docx;
mod pdf;

use tmt::types::request::Language;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    docx::translate_docx(
        "/home/bedgirb/Downloads/swd.docx",
        "output.docx",
        Language::English,
        Language::Nepali,
    )
    .await?;

    pdf::translate_pdf(
        "/home/bedgirb/Downloads/sample_test.pdf",
        "output_from_pdf.docx",
        Language::English,
        Language::Nepali,
    )
    .await?;

    Ok(())
}
