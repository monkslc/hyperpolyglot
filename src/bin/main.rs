use clap::{App, Arg};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    convert::TryFrom,
    io::{self, Write},
    path::PathBuf,
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use hyperpolyglot::{get_language_breakdown, Detection, Language, LanguageType};

struct CLIOptions {
    color: bool,
    condensed_output: bool,
    filters: Option<Vec<Regex>>,
}

impl CLIOptions {
    fn matches_filter(&self, pattern: &str) -> bool {
        if let Some(filters) = &self.filters {
            filters.iter().any(|filter| filter.is_match(pattern))
        } else {
            true
        }
    }

    fn color_option(&self) -> ColorChoice {
        if self.color {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        }
    }
}

fn main() {
    let matches = get_cli().get_matches();
    let path = matches.value_of("PATH").unwrap();
    let breakdown = get_language_breakdown(path);

    let mut language_count: Vec<(&'static str, Vec<(Detection, PathBuf)>)> = breakdown
        .into_iter()
        .filter(|(language_name, _)| {
            match Language::try_from(*language_name).map(|l| l.language_type) {
                Ok(LanguageType::Markup) | Ok(LanguageType::Programming) => true,
                _ => false,
            }
        })
        .collect();
    language_count.sort_by(|(_, a), (_, b)| b.len().cmp(&a.len()));
    if let Err(_) = print_language_split(&language_count) {
        std::process::exit(1);
    }

    let cli_options = CLIOptions {
        color: !matches.is_present("no-color"),
        condensed_output: matches.is_present("condensed"),
        filters: matches
            .values_of("filter")
            .map(|filters| {
                filters.map(|f| Regex::new(f).expect(&format!("Invalid filter: {}", f)[..]))
            })
            .map(|filters| filters.collect()),
    };

    if matches.is_present("file-breakdown") {
        writeln!(io::stdout(), "").unwrap_or_else(|_| std::process::exit(1));
        if let Err(_) = print_file_breakdown(&language_count, &cli_options) {
            std::process::exit(1);
        }
    }

    if matches.is_present("strategy-breakdown") {
        writeln!(io::stdout(), "").unwrap_or_else(|_| std::process::exit(1));
        if let Err(_) = print_strategy_breakdown(&language_count, &cli_options) {
            std::process::exit(1);
        }
    }
}

fn get_cli<'a, 'b>() -> App<'a, 'b> {
    App::new("Hyperpolyglot")
        .version("0.1.7")
        .about("Hyperpolyglot is a programming language detector. It supports detecting the programming language of a file or the programming language makeup of a directory.")
        .arg(Arg::with_name("PATH").index(1).default_value("."))
        .arg(
            Arg::with_name("file-breakdown")
                .short("b")
                .long("breakdown")
                .help("prints the language detected for each file visited"),
        )
        .arg(
            Arg::with_name("strategy-breakdown")
                .short("s")
                .long("strategies")
                .help(
                    "Prints each strategy used and what files were detected using that strategy",
                ),
        )
        .arg(
            Arg::with_name("condensed")
                .short("c")
                .long("condensed")
                .help("Condenses the output for the breakdowns to only show the headers"),
        )
        .arg(
            Arg::with_name("filter").short("f").long("filter").help(
                "A regex that is used to filter by header which sections get printed for the file and strategy breakdown.",
            ).takes_value(true).multiple(true),
        )
        .arg(
            Arg::with_name("no-color").short("n").long("no-color").help(
                "Don't color code the output of the breakdowns. This is useful when piping/redirecting the output of hyperpolyglot.",
            ),
        )
}

fn print_language_split(
    language_counts: &Vec<(&'static str, Vec<(Detection, PathBuf)>)>,
) -> Result<(), io::Error> {
    let total = language_counts
        .iter()
        .fold(0, |acc, (_, files)| acc + files.len()) as f64;
    for (language, files) in language_counts.iter() {
        let percentage = ((files.len() * 100) as f64) / total;
        writeln!(io::stdout(), "{:.2}% {}", percentage, language)?;
    }

    Ok(())
}

fn print_file_breakdown(
    language_counts: &Vec<(&'static str, Vec<(Detection, PathBuf)>)>,
    options: &CLIOptions,
) -> Result<(), io::Error> {
    let mut stdout = StandardStream::stdout(options.color_option());
    for (language, breakdowns) in language_counts.iter() {
        if options.matches_filter(language) {
            stdout.set_color(&TITLE_COLOR)?;
            write!(stdout, "{}", language)?;

            stdout.set_color(&DEFAULT_COLOR)?;
            writeln!(stdout, " ({})", breakdowns.len())?;
            if !options.condensed_output {
                for (_, file) in breakdowns.iter() {
                    let path = strip_relative_parts(file);
                    writeln!(stdout, "{}", path.display())?;
                }
                writeln!(stdout, "")?;
            }
        }
    }
    Ok(())
}

fn print_strategy_breakdown(
    language_counts: &Vec<(&'static str, Vec<(Detection, PathBuf)>)>,
    options: &CLIOptions,
) -> Result<(), io::Error> {
    let mut strategy_breakdown = HashMap::new();
    for (language, files) in language_counts.into_iter() {
        for (detection, file) in files.into_iter() {
            let files = strategy_breakdown
                .entry(detection.variant())
                .or_insert(BinaryHeap::new());
            files.push(Reverse((language, file)));
        }
    }

    let mut strategy_breakdowns: Vec<(&str, BinaryHeap<Reverse<(&&str, &PathBuf)>>)> =
        strategy_breakdown.into_iter().collect();
    strategy_breakdowns.sort_by(|(_, a), (_, b)| b.len().cmp(&a.len()));

    let mut stdout = StandardStream::stdout(options.color_option());
    for (strategy, mut breakdowns) in strategy_breakdowns.into_iter() {
        if options.matches_filter(&strategy[..]) {
            stdout.set_color(&TITLE_COLOR)?;
            write!(stdout, "{}", strategy)?;

            stdout.set_color(&DEFAULT_COLOR)?;
            writeln!(stdout, " ({})", breakdowns.len())?;
            if !options.condensed_output {
                while let Some(Reverse((language, file))) = breakdowns.pop() {
                    stdout.set_color(&DEFAULT_COLOR)?;
                    let path = strip_relative_parts(file);
                    write!(stdout, "{}", path.display())?;

                    stdout.set_color(&LANGUAGE_COLOR)?;
                    writeln!(stdout, " ({})", language)?;
                }
                writeln!(stdout, "")?;
            }
        }
    }
    Ok(())
}

fn strip_relative_parts<'a>(path: &'a PathBuf) -> &'a std::path::Path {
    if path.starts_with("./") {
        path.strip_prefix("./").unwrap()
    } else {
        path.as_path()
    }
}

lazy_static! {
    static ref TITLE_COLOR: ColorSpec = {
        let mut title_color = ColorSpec::new();
        title_color.set_fg(Some(Color::Magenta));
        title_color
    };
    static ref DEFAULT_COLOR: ColorSpec = ColorSpec::default();
    static ref LANGUAGE_COLOR: ColorSpec = {
        let mut language_color = ColorSpec::new();
        language_color.set_fg(Some(Color::Green));
        language_color
    };
}
