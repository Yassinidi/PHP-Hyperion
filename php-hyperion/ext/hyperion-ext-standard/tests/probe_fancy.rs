//! Temporary probe: what does fancy-regex 0.19 accept and how does it behave?
use fancy_regex::{Regex, RegexBuilder};

fn try_compile(label: &str, pat: &str) {
    match Regex::new(pat) {
        Ok(_) => println!("OK    {label}  {pat}"),
        Err(e) => println!("ERR   {label}  {pat}  -> {e}"),
    }
}

#[test]
fn probe() {
    println!("--- construct support ---");
    try_compile("dotenv lookahead", r#"(?=([^\\]"))"#);
    try_compile("dotenv unicode", r"\A[\p{Ll}\p{Lu}\p{M}\p{N}_.]+\z");
    try_compile("ungreedy flag", r"(?U)a+");
    try_compile("backslash-Z", r"foo\Z");
    try_compile("horiz ws", r"\h+");
    try_compile("any newline", r"\R");
    try_compile("possessive", r"a++b");
    try_compile("atomic group", r"(?>a+)b");
    try_compile("quoted name", r"(?'y'\d+)");
    try_compile("P-name", r"(?P<y>\d+)");
    try_compile("angle name", r"(?<y>\d+)");
    try_compile("backref", r"(a)\1");
    try_compile("lookbehind", r"(?<=x)y");
    try_compile("verb UTF8", r"(*UTF8)abc");
    try_compile("named + numbered", r"(?<y>\d)(\w)");
    try_compile("inline i", r"(?i)ABC");
    try_compile("dollar endonly", r"foo$");
    try_compile("conditional", r"(a)?(?(1)b|c)");
    try_compile("recursion", r"\((?:[^()]|(?R))*\)");
    try_compile("K escape", r"foo\Kbar");
    try_compile("hex esc", r"\x41");
    try_compile("octal", r"\101");
    try_compile("posix class", r"[[:alpha:]]+");
    try_compile("nested quant", r"^(\d+)*$");

    println!("--- zero-width captures_iter (dotenv Lines.php) ---");
    let re = Regex::new(r#"(?=([^\\]"))"#).unwrap();
    let subj = r#"a"b"c"#;
    let mut n = 0;
    for c in re.captures_iter(subj) {
        let c = c.unwrap();
        let m0 = c.get(0).unwrap();
        println!(
            "  match {n}: range {}..{} full={:?} g1={:?}",
            m0.start(),
            m0.end(),
            m0.as_str(),
            c.get(1).map(|m| m.as_str())
        );
        n += 1;
        if n > 10 {
            println!("  RUNAWAY");
            break;
        }
    }
    println!("  total={n}");

    println!("--- named group numbering + capture_names order ---");
    let re = Regex::new(r"(?<year>\d{4})-(\d{2})").unwrap();
    println!("  captures_len={}", re.captures_len());
    for (i, name) in re.capture_names().enumerate() {
        println!("  group {i} name={name:?}");
    }
    let c = re.captures("2024-06").unwrap().unwrap();
    println!(
        "  g0={:?} g1={:?} g2={:?} byname={:?}",
        c.get(0).map(|m| m.as_str()),
        c.get(1).map(|m| m.as_str()),
        c.get(2).map(|m| m.as_str()),
        c.name("year").map(|m| m.as_str())
    );

    println!("--- unmatched trailing group ---");
    let re = Regex::new(r"(a)(b)?(c)?").unwrap();
    let c = re.captures("a").unwrap().unwrap();
    println!("  len={} g1={:?} g2={:?} g3={:?}", c.len(), c.get(1).map(|m| m.as_str()), c.get(2).map(|m| m.as_str()), c.get(3).map(|m| m.as_str()));

    println!("--- captures_from_pos (preg offset arg) ---");
    let re = Regex::new(r"\d+").unwrap();
    let c = re.captures_from_pos("ab 12 cd 34", 6).unwrap().unwrap();
    let m = c.get(0).unwrap();
    println!("  found {:?} at {}..{}", m.as_str(), m.start(), m.end());

    println!("--- anchored ^ with from_pos: does ^ match at pos? ---");
    let re = Regex::new(r"^\d+").unwrap();
    println!("  {:?}", re.captures_from_pos("ab 12", 3).unwrap().map(|c| c.get(0).unwrap().as_str().to_string()));

    println!("--- builder flags ---");
    let re = RegexBuilder::new(r"a+")
        .case_insensitive(true)
        .multi_line(true)
        .dot_matches_new_line(true)
        .ignore_whitespace(true)
        .build();
    println!("  builder combo: {:?}", re.is_ok());

    println!("--- runtime backtrack limit error ---");
    let re = RegexBuilder::new(r"^(a+)+$").backtrack_limit(1000).build().unwrap();
    let subj = "a".repeat(40) + "b";
    match re.is_match(&subj) {
        Ok(v) => println!("  is_match ok: {v}"),
        Err(e) => println!("  is_match err: {e:?}"),
    }

    println!("--- split behavior ---");
    let re = Regex::new(r"(\r\n|\n|\r)").unwrap();
    let parts: Vec<_> = re.split("a\nb\r\nc").map(|p| p.unwrap().to_string()).collect();
    println!("  split with capture group: {parts:?}");
}
