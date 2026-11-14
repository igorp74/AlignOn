use std::io::{self, BufRead, IsTerminal, Write};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
    Center,
}

/// "1,3-5,7-" -> [(1,1),(3,5),(7,MAX)]
fn parse_ranges(s: &str) -> Option<Vec<(usize, usize)>> {
    s.split(',')
        .map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            let (a, b) = part.split_once('-').unwrap_or((part, part));
            let lo = if a.is_empty() { 1 } else { a.parse().ok()? };
            let hi = if b.is_empty() { usize::MAX } else { b.parse().ok()? };
            (lo >= 1 && lo <= hi).then_some((lo, hi))
        })
        .collect()
}

/// Recognises -l / -r / -c with an optional =SPEC. Bare flag means all columns.
fn align_arg(a: &str) -> Option<(Align, &str)> {
    let al = match a.get(..2)? {
        "-l" => Align::Left,
        "-r" => Align::Right,
        "-c" => Align::Center,
        _ => return None,
    };
    match &a[2..] {
        "" => Some((al, "1-")),
        s => s.strip_prefix('=').map(|s| (al, s)),
    }
}

/// Byte length of a qualified section at the start of `s` (opening through closing
/// qualifier), or None if there is no closing qualifier. Doubled qualifiers are skipped.
fn quoted_len(s: &str, q: &str) -> Option<usize> {
    let mut pos = q.len();
    loop {
        let p = pos + s[pos..].find(q)?;
        let after = p + q.len();
        if s[after..].starts_with(q) {
            pos = after + q.len(); // escaped (doubled) qualifier, keep looking
        } else {
            return Some(after);
        }
    }
}

/// Split a line into (cell, following_delimiter) pairs.
/// The last pair has an empty delimiter. `delims` must be sorted longest-first.
fn tokenize<'a>(line: &'a str, delims: &'a [String], qual: &str) -> Vec<(&'a str, &'a str)> {
    let mut out = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < line.len() {
        let rest = &line[i..];

        // Qualifier only counts at the start of a cell (after optional whitespace).
        if !qual.is_empty() && rest.starts_with(qual) && line[start..i].trim().is_empty() {
            if let Some(n) = quoted_len(rest, qual) {
                i += n;
                continue;
            }
        }

        if let Some(d) = delims.iter().find(|d| rest.starts_with(d.as_str())) {
            out.push((line[start..i].trim(), d.as_str()));
            i += d.len();
            start = i;
        } else {
            i += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    out.push((line[start..].trim(), ""));
    out
}

fn spaces(out: &mut impl Write, n: usize) -> io::Result<()> {
    write!(out, "{:n$}", "")
}

/// Maps long options onto their short forms: --right=1-3 -> -r=1-3.
fn normalize(a: &str) -> String {
    for (long, short) in [
        ("--left", "-l"),
        ("--right", "-r"),
        ("--center", "-c"),
        ("--gap", "-g"),
        ("--qualifier", "-q"),
    ] {
        if let Some(rest) = a.strip_prefix(long) {
            if rest.is_empty() || rest.starts_with('=') {
                return format!("{short}{rest}");
            }
        }
    }
    a.to_string()
}

fn print_help() {
    // Colour only on a terminal, and never when NO_COLOR is set.
    let color = io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let paint = |codes: &str, s: &str| -> String {
        if color { format!("\x1b[{codes}m{s}\x1b[0m") } else { s.to_string() }
    };
    let (heading, flag, long, arg) = ("1;33", "1;34", "34", "32");

    println!("\nalignon - Align text into columns on one or more delimiters (reads stdin)");
    println!("{}", "-".repeat(82));
    println!("{}", paint(heading, "Usage: alignon [OPTIONS] [DELIM ...]"));

    // (short, long, argument, description)
    let opts = [
        ("-l", "--left", "[=SPEC]", "Left-align columns (default for all columns)"),
        ("-r", "--right", "[=SPEC]", "Right-align columns"),
        ("-c", "--center", "[=SPEC]", "Center columns"),
        ("-g", "--gap", "=<N>", "Spaces on each side of a delimiter (default: 1)"),
        ("-q", "--qualifier", "=<QUAL>", "Text qualifier, e.g. -q='\"' (see below)"),
        ("-h", "--help", "", "Display this help message"),
    ];
    let sw = opts.iter().map(|o| o.0.len()).max().unwrap_or(0);
    let lw = opts.iter().map(|o| o.1.len()).max().unwrap_or(0);
    let aw = opts.iter().map(|o| o.2.len()).max().unwrap_or(0);
    for (s, l, a, d) in opts {
        println!(
            " {} | {} {}  {}",
            paint(flag, &format!("{s:<sw$}")),
            paint(long, &format!("{l:<lw$}")),
            paint(arg, &format!("{a:<aw$}")),
            d
        );
    }

    println!("\n{}", paint(heading, "Arguments:"));
    println!(" {}  Delimiter string(s); default is TAB. Longest match wins.", paint(arg, "DELIM ..."));
    println!("            \\t = TAB, \\s = space (quote them in the shell).");
    println!("            Surrounding spaces in an argument are ignored.");

    println!("\n{}", paint(heading, "Column SPEC (1-based):"));
    for (spec, what) in [
        ("1       ", "column 1"),
        ("2-4     ", "columns 2 to 4"),
        ("5-      ", "column 5 to the last"),
        ("-3      ", "columns 1 to 3"),
        ("1,3-5,7-", "comma-separated list of the above"),
    ] {
        println!(" {}  {}", paint(arg, spec), what);
    }
    println!(" A bare flag (-r) applies to all columns. Later flags override earlier ones,");
    println!(" and columns not listed in any flag are left-aligned.");

    println!("\n{}", paint(heading, "Text qualifier:"));
    println!(" A cell that starts with QUAL is not split until the closing QUAL.");
    println!(" A doubled QUAL inside is an escaped quote; an unclosed QUAL is ordinary text.");

    println!("\n{}", paint(heading, "Examples:"));
    println!(" alignon + = -r                 Right-align every column");
    println!(" alignon + = -r=1               Right-align column 1 only");
    println!(" alignon + = -r=1 -c=2 -l=3     Column 1 right, 2 centered, 3 left");
    println!(" alignon '|' -q='\"' < data.txt  Split on |, ignore | inside quotes");
}

fn main() -> io::Result<()> {
    let (mut gap, mut delims) = (1usize, Vec::<String>::new());
    let mut rules: Vec<(Align, Vec<(usize, usize)>)> = Vec::new();
    let mut qual = String::new();

    for arg in std::env::args().skip(1) {
        let norm = normalize(arg.trim());
        let a = norm.as_str();
        if let Some((al, spec)) = align_arg(a) {
            match parse_ranges(spec) {
                Some(r) => rules.push((al, r)),
                None => {
                    eprintln!("alignon: invalid column spec '{spec}'");
                    std::process::exit(2);
                }
            }
            continue;
        }
        match a {
            "" => {}
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            _ => {
                if let Some(n) = a.strip_prefix("-g=") {
                    gap = n.parse().unwrap_or_else(|_| {
                        eprintln!("alignon: invalid gap '{n}'");
                        std::process::exit(2);
                    });
                } else if let Some(q) = a.strip_prefix("-q=") {
                    qual = q.to_string();
                } else {
                    delims.push(a.replace("\\t", "\t").replace("\\s", " "));
                }
            }
        }
    }
    if delims.is_empty() {
        delims.push("\t".into());
    }
    delims.sort_by(|a, b| b.len().cmp(&a.len())); // greedy: longest first

    let lines: Vec<String> = io::stdin().lock().lines().collect::<Result<_, _>>()?;
    let rows: Vec<_> = lines.iter().map(|l| tokenize(l, &delims, &qual)).collect();

    // Phase 1: column widths (display width, so CJK/emoji align)
    let mut widths: Vec<usize> = Vec::new();
    for row in &rows {
        for (c, (cell, _)) in row.iter().enumerate() {
            let w = cell.width();
            if c == widths.len() { widths.push(w) } else if w > widths[c] { widths[c] = w }
        }
    }

    // Resolve each column's alignment once; later rules win, default is left.
    let aligns: Vec<Align> = (0..widths.len())
        .map(|c| {
            let n = c + 1;
            rules
                .iter()
                .rev()
                .find(|(_, rs)| rs.iter().any(|&(lo, hi)| n >= lo && n <= hi))
                .map_or(Align::Left, |(al, _)| *al)
        })
        .collect();

    // Phase 2: render
    let mut out = io::BufWriter::new(io::stdout().lock());
    let gap = " ".repeat(gap);
    for row in &rows {
        for (c, (cell, delim)) in row.iter().enumerate() {
            let pad = widths[c].saturating_sub(cell.width());
            let (l, r) = match aligns[c] {
                Align::Left => (0, pad),
                Align::Right => (pad, 0),
                Align::Center => (pad / 2, pad - pad / 2),
            };
            spaces(&mut out, l)?;
            out.write_all(cell.as_bytes())?;
            if !delim.is_empty() {
                spaces(&mut out, r)?; // last column: no trailing padding
                write!(out, "{gap}{delim}{gap}")?;
            }
        }
        out.write_all(b"\n")?;
    }
    out.flush()
}
