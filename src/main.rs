use clap::{App, Arg};
use std::{collections::HashMap, io::Write, path::PathBuf};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use hyperpolyglot::{get_language_breakdown, get_language_info, Detection, LanguageType};

struct CLIOptions {
    condensed_output: bool,
}

fn main() {
    let matches = get_cli().get_matches();
    let path = matches.value_of("PATH").unwrap();
    let breakdown = get_language_breakdown(path);

    let mut language_count: Vec<(&&'static str, &Vec<(Detection, PathBuf)>)> = breakdown
        .iter()
        .filter(|(language_name, _)| {
            match get_language_info(language_name).map(|l| &l.language_type) {
                Some(LanguageType::Markup) | Some(LanguageType::Programming) => true,
                _ => false,
            }
        })
        .collect();
    language_count.sort_by(|(_, a), (_, b)| b.len().cmp(&a.len()));
    print_language_split(&language_count);

    let cli_options = CLIOptions {
        condensed_output: matches.is_present("condensed"),
    };

    if matches.is_present("file-breakdown") {
        println!("");
        print_file_breakdown(&language_count);
    }

    if matches.is_present("strategy-breakdown") {
        println!("");
        print_strategy_breakdown(&language_count, cli_options);
    }
}

fn get_cli<'a, 'b>() -> App<'a, 'b> {
    App::new("Hyperpolyglot")
        .version("0.1.0")
        .about("Get the programming language breakdown for a file.")
        .arg(Arg::with_name("PATH").index(1).default_value("."))
        .arg(
            Arg::with_name("file-breakdown")
                .short("b")
                .long("breakdown")
                .help("prints the language detected for each file it visits"),
        )
        .arg(
            Arg::with_name("strategy-breakdown")
                .short("s")
                .long("strategies")
                .help(
                    "Prints each strategy used and what files were determined using that strategy",
                ),
        )
        .arg(
            Arg::with_name("condensed")
                .short("c")
                .long("condensed")
                .help("Condenses the output for the breakdowns to only show the counts"),
        )
}

fn print_language_split(language_counts: &Vec<(&&'static str, &Vec<(Detection, PathBuf)>)>) {
    let total = language_counts
        .iter()
        .fold(0, |acc, (_, files)| acc + files.len()) as f64;
    for (language, files) in language_counts.iter() {
        let percentage = ((files.len() * 100) as f64) / total;
        println!("{:.2}% {}", percentage, language);
    }
}

fn print_file_breakdown(language_counts: &Vec<(&&'static str, &Vec<(Detection, PathBuf)>)>) {
    for (language, files) in language_counts.iter() {
        println!("{} ({})", language, files.len());
        for (detection, file) in files.iter() {
            println!(
                "{} ({})",
                file.to_str().unwrap_or("Error"),
                detection.variant()
            );
        }
        println!("");
    }
}

fn print_strategy_breakdown(
    language_counts: &Vec<(&&'static str, &Vec<(Detection, PathBuf)>)>,
    options: CLIOptions,
) {
    let mut strategy_breakdown = HashMap::new();
    for (language, files) in language_counts.iter() {
        for (detection, file) in files.iter() {
            let files = strategy_breakdown
                .entry(detection.variant())
                .or_insert(vec![]);
            files.push((file, language));
        }
    }

    let mut strategy_breakdowns: Vec<(&String, &Vec<(&PathBuf, &&&str)>)> =
        strategy_breakdown.iter().collect();
    strategy_breakdowns.sort_by(|(_, a), (_, b)| b.len().cmp(&a.len()));

    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let default_color = ColorSpec::default();

    let mut title_color = ColorSpec::new();
    title_color.set_fg(Some(Color::Magenta));

    let mut language_color = ColorSpec::new();
    language_color.set_fg(Some(Color::Green));

    for (strategy, breakdowns) in strategy_breakdowns.iter() {
        stdout.set_color(&title_color).unwrap();
        write!(stdout, "{}", strategy).unwrap();

        stdout.set_color(&default_color).unwrap();
        writeln!(stdout, " ({})", breakdowns.len()).unwrap();
        if !options.condensed_output {
            for (file, language) in breakdowns.iter() {
                stdout.set_color(&default_color).unwrap();
                write!(stdout, "{}", file.to_str().unwrap_or("Error")).unwrap();

                stdout.set_color(&language_color).unwrap();
                writeln!(stdout, " ({})", language).unwrap();
            }
            writeln!(stdout, "").unwrap();
        }
    }
}
