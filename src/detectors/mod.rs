mod classifier;
mod extensions;
mod filenames;
mod heuristics;
mod interpreters;

pub use classifier::classify;
pub use extensions::{get_extension, get_languages_from_extension};
pub use filenames::get_language_from_filename;
pub use heuristics::get_languages_from_heuristics;
pub use interpreters::get_languages_from_shebang;
