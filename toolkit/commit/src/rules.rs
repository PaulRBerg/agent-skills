use crate::state::MessageFormat;

pub const CONVENTIONAL: &str = include_str!("rules/conventional-prefix-format.md");
pub const NATURAL: &str = include_str!("rules/natural-language-format.md");

pub const fn for_format(format: MessageFormat) -> &'static str {
    match format {
        MessageFormat::Conventional => CONVENTIONAL,
        MessageFormat::Natural => NATURAL,
    }
}
