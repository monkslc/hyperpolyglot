use std::collections::HashMap;

fn main() {
    let breakdown = hyperpolyglot::get_language_breakdown("./");
    print_breakdown(breakdown);
}

fn print_breakdown(languages: HashMap<&'static str, i32>) {
    let mut language_counts: Vec<(&&'static str, &i32)> = languages.iter().collect();
    let total = language_counts.iter().fold(0, |acc, (_, x)| acc + **x) as f64;
    language_counts.sort_by(|(_, a), (_, b)| b.cmp(a));
    for (language, count) in language_counts.iter() {
        let percentage = ((**count * 100) as f64) / total;
        println!("{:.2}% {}", percentage, language);
    }
}
