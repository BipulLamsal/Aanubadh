use crate::types::document::Document;

pub mod document;

pub trait Parser {
    fn parse(&self, buffer: &[u8]) -> Document;
}
