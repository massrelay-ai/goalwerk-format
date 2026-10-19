//! `atingo-path check <packs dir> [--pins <file>]` — the cross rules over a pack root. One JSON line on stdout
//! (`{"findings":[...],"counts":{...}}`); exit 1 on any error finding, 2 when a file cannot be read or parsed, else 0.
use atingo_path::pins::Pins;
use atingo_path::loop_data::load_dir;
use atingo_path::graph::{counts, load_graph};
use atingo_path::i18n::{load_i18n, validate_i18n};
use atingo_path::{load_prices, validate_price, validate, validate_graph, validate_loop, GoalKinds, PathFile, Severity};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn json_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut v: Vec<_> = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    v.sort();
    Ok(v)
}

fn read<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T, String> {
    let s = fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(s.trim_start_matches('\u{feff}')).map_err(|e| format!("{}: {e}", p.display()))
}

/// Today as `YYYY-MM-DD` (UTC): `ATINGO_TODAY` overrides it (tests, reproducible runs). Civil-from-days, no date crate.
fn today() -> String {
    if let Ok(t) = std::env::var("ATINGO_TODAY") {
        return t;
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn run(args: &[String]) -> Result<bool, String> {
    if args.first().map(String::as_str) != Some("check") || args.len() < 2 {
        return Err("usage: atingo-path check <packs dir> [--pins <file>]".into());
    }
    let dir = Path::new(&args[1]);
    let mut pins_file = "tools/packs/kernel-pins.json".to_string();
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--pins" && i + 1 < args.len() {
            pins_file = args[i + 1].clone();
            i += 2;
        } else {
            return Err(format!("unexpected argument {}", args[i]));
        }
    }
    let pins: Pins = read(Path::new(&pins_file))?;
    let mut kinds = GoalKinds { version: 1, family: String::new(), kinds: Vec::new() };
    for f in json_files(&dir.join("goal-kinds"))? {
        let g: GoalKinds = read(&f)?;
        if kinds.family.is_empty() {
            kinds.family = g.family.clone();
        }
        kinds.kinds.extend(g.kinds);
    }
    let mut findings = Vec::new();
    let mut parsed: Vec<PathFile> = Vec::new();
    let mut all_templates = std::collections::BTreeMap::new();
    let (mut paths, mut nodes, mut personas, mut templates) = (0usize, 0usize, 0usize, 0usize);
    for f in json_files(&dir.join("paths"))? {
        let p: PathFile = read(&f)?;
        paths += 1;
        nodes += p.nodes.len();
        let label = format!("paths/{}", f.file_name().unwrap_or_default().to_string_lossy());
        let data = load_dir(dir, &p.id)?;
        for mut x in validate(&p, &kinds, &pins) {
            x.path = label.clone();
            findings.push(x);
        }
        findings.extend(validate_loop(&p, &kinds, &pins, &data));
        personas = data.personas.len();
        templates = data.templates.len();
        all_templates.extend(data.templates);
        parsed.push(p);
    }
    let prices = load_prices(dir)?;
    findings.extend(validate_price(&parsed, &prices, &pins));
    let locale_files = load_i18n(dir)?;
    let locales = locale_files.len();
    findings.extend(validate_i18n(&locale_files, &parsed, &all_templates, &pins));
    let mut graph_counts = (0usize, 0usize, 0usize, 0usize);
    if let Some(g) = load_graph(dir)? {
        graph_counts = counts(&g);
        findings.extend(validate_graph(&g, &today()));
    }
    let failed = findings.iter().any(|f| f.severity == Severity::Error);
    let out = serde_json::json!({ "findings": findings, "counts": { "kinds": kinds.kinds.len(), "paths": paths, "nodes": nodes, "personas": personas, "templates": templates, "locales": locales, "prices": prices.len(), "graph": { "sectors": graph_counts.0, "occupations": graph_counts.1, "competencies": graph_counts.2, "items": graph_counts.3 } } });
    println!("{out}");
    Ok(!failed)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
