#[derive(Debug, Clone)]
pub struct Document {
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub properties: SectionProperties,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Default)]
pub struct SectionProperties {
    pub page_size: (f32, f32),
    pub orientation: Orientation,
    pub margins: Margin,
}

#[derive(Debug, Clone, Default)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

impl Document {
    pub fn new() -> Self {
        Document {
            sections: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<TextSpan>,
        style: BlockStyle,
    },
    Paragraph {
        content: Vec<TextSpan>,
        style: BlockStyle,
    },
    List {
        items: Vec<ListItem>,
        list_type: ListType,
        style: BlockStyle,
    },
    Image {
        data: Vec<u8>,
        extension: String,
        dimensions: (Option<f32>, Option<f32>),
        layout: ImageLayout,
        caption: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub content: Vec<TextSpan>,
    pub sub_list: Option<Box<Block>>,
}

#[derive(Debug, Clone)]
pub enum ListType {
    Unordered(String),
    Ordered(NumberingStyle),
}

#[derive(Debug, Clone)]
pub enum NumberingStyle {
    Decimal,
    LowerRoman,
    UpperRoman,
    LowerAlpha,
}

#[derive(Debug, Clone)]
pub struct ImageLayout {
    pub alignment: Alignment,
    pub is_inline: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    pub font_size: Option<f32>,
    pub color: Option<Color>,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underline: bool,
    pub is_strikethrough: bool,
}

#[derive(Debug, Clone)]
pub enum Color {
    Rgb(u8, u8, u8),
    Hex(String),
}

#[derive(Debug, Clone, Default)]
pub struct BlockStyle {
    pub alignment: Alignment,
    pub indent: Option<f32>,
    pub line_spacing: Option<f32>,
    pub background_color: Option<Color>,
    pub margin: Margin,
}

#[derive(Debug, Clone, Default)]
pub struct Margin {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}
