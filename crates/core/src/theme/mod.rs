pub mod palette;
pub mod spec;
pub mod syntax;

pub use palette::UiPalette;
pub use spec::{Player, Rgba, SyntaxToken, ThemeFamily, ThemeKind, ThemeSpec, ThemeStyle, parse_theme_family};
pub use syntax::{SyntaxStyle, syntax_styles};
