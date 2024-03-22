pub mod classifier;
pub mod extensions;
pub mod filenames;
pub mod heuristics;
pub mod interpreters;

pub use classifier::classify;
pub use extensions::{get_extension, get_languages_from_extension};
pub use filenames::get_language_from_filename;
pub use heuristics::get_languages_from_heuristics;
pub use interpreters::get_languages_from_shebang;
