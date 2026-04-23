use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
pub enum FormatType {
    Docx,
    Csv,
    Pdf,
}

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Nep,
    Eng,
    Tmg,
}

#[derive(Debug, Clone)]
pub struct PipeLineState {
    pub from: (FormatType, Language),
    pub to: (FormatType, Language),
    pub name: Rc<str>,
}
