use clap::{App, Arg};

use hyperpolyglot::{get_language_breakdown, get_language_info, Breakdown, LanguageType};

fn main() {
    let matches = get_cli().get_matches();
    let path = matches.value_of("PATH").unwrap();
    let breakdown = get_language_breakdown(path);

    let mut language_count: Vec<(&&'static str, &Breakdown)> = breakdown
        .iter()
        .filter(
            |(language_name, _)| match get_language_info(language_name).map(|l| &l.type_of) {
                Some(LanguageType::Markup) | Some(LanguageType::Programming) => true,
                _ => false,
            },
        )
        .collect();
    language_count.sort_by(|(_, a), (_, b)| b.count.cmp(&a.count));
    print_language_split(&language_count);

    if matches.is_present("file-breakdown") {
        println!("");
        print_file_breakdown(&language_count);
    }
}

fn get_cli<'a, 'b>() -> App<'a, 'b> {
    App::new("Hyperpolyglot")
        .version("0.1.0")
        .about("Get the programming language breakdown for a file.")
        .arg(
            Arg::with_name("file-breakdown")
                .short("b")
                .long("breakdown")
                .help("prints the language detected for each file it visits"),
        )
        .arg(Arg::with_name("PATH").index(1).default_value("./"))
}

fn print_language_split(language_counts: &Vec<(&&'static str, &Breakdown)>) {
    let total = language_counts
        .iter()
        .fold(0, |acc, (_, breakdown)| acc + breakdown.count) as f64;
    for (language, breakdown) in language_counts.iter() {
        let percentage = ((breakdown.count * 100) as f64) / total;
        println!("{:.2}% {}", percentage, language);
    }
}

fn print_file_breakdown(language_counts: &Vec<(&&'static str, &Breakdown)>) {
    for (language, breakdown) in language_counts.iter() {
        println!("{}", language);
        for file in breakdown.files.iter() {
            println!("{}", file.to_str().unwrap_or("Error"));
        }
        println!("");
    }
}
