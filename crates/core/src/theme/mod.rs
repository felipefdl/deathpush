pub mod palette;
pub mod spec;
pub mod syntax;

pub use palette::UiPalette;
pub use spec::{Rgba, Scope, ThemeKind, ThemeSpec, TokenColor, TokenSettings, parse_theme};
pub use syntax::{SyntaxStyle, syntax_styles};
