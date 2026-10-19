//! L16: the knowledge-graph file types (spec 03 §2, §3) and a loader for `<packs>/graph/`. Mirrors `tools/packs/schemas/graph-*.schema.json`;
//! every struct denies unknown fields, so a protected ground (or anything else) cannot enter as a key. A pack is data; nothing here decides anything.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SectorsFile {
    pub version: u32,
    pub onet_release: String,
    pub sectors: Vec<Sector>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sector {
    pub id: String,
    pub title: String,
    pub regulated: bool,
    pub job_families: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OccupationsFile {
    pub version: u32,
    pub occupations: Vec<Occupation>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Occupation {
    pub id: String,
    pub title: String,
    pub description: String,
    pub job_family: String,
    pub job_zone: u32,
    pub part_of: String,
    pub top: Vec<TopElement>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TopElement {
    pub comp: String,
    pub element: String,
    pub importance: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompetenciesFile {
    pub version: u32,
    pub competencies: Vec<Competency>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Competency {
    pub id: String,
    pub title: String,
    pub kind: String,
    #[serde(default)]
    pub onet_element: Option<String>,
    pub part_of: String,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub occupations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemsFile {
    pub version: u32,
    pub competency: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub sources: Vec<ItemSource>,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    #[serde(default)]
    pub valid_until: Option<String>,
    #[serde(default)]
    pub badge: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemSource {
    pub url: String,
    pub title: String,
    pub licence: String,
    pub retrieved_at: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub url: String,
    pub title: String,
    pub licence: String,
    pub retrieved_at: String,
    pub sha256: String,
    pub bytes: u64,
    #[serde(default)]
    pub normalize: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LessonsFile {
    pub version: u32,
    pub lessons: Vec<Lesson>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    pub id: String,
    pub path: String,
    pub node: String,
    pub title: String,
    pub steps: Vec<LessonStep>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LessonStep {
    pub prompt: String,
    pub evidence_hint: String,
    pub item: String,
}

/// One graph, each row tagged with the pack-relative file it came from (for findings).
#[derive(Debug, Default)]
pub struct Graph {
    pub onet_release: String,
    pub sectors: Vec<Sector>,
    pub occupations: Vec<(String, Occupation)>,
    pub competencies: Vec<(String, Competency)>,
    pub items: Vec<(String, ItemsFile)>,
    pub lessons: Vec<(String, LessonsFile)>,
    pub sources: Vec<SourceRecord>,
}

fn json_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
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

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// `Ok(None)` when the pack root has no `graph/` folder.
pub fn load_graph(pack_root: &Path) -> Result<Option<Graph>, String> {
    let dir = pack_root.join("graph");
    if !dir.exists() {
        return Ok(None);
    }
    let mut g = Graph::default();
    let sectors: SectorsFile = read(&dir.join("sectors.json"))?;
    g.onet_release = sectors.onet_release;
    g.sectors = sectors.sectors;
    for p in json_files(&dir.join("occupations"))? {
        let f: OccupationsFile = read(&p)?;
        let name = rel(pack_root, &p);
        g.occupations.extend(f.occupations.into_iter().map(|o| (name.clone(), o)));
    }
    for p in json_files(&dir.join("competencies"))? {
        let f: CompetenciesFile = read(&p)?;
        let name = rel(pack_root, &p);
        g.competencies.extend(f.competencies.into_iter().map(|c| (name.clone(), c)));
    }
    for p in json_files(&dir.join("items"))? {
        g.items.push((rel(pack_root, &p), read(&p)?));
    }
    let lessons = dir.join("lessons");
    if lessons.exists() {
        let mut subs: Vec<_> = fs::read_dir(&lessons).map_err(|e| format!("{}: {e}", lessons.display()))?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_dir()).collect();
        subs.sort();
        for s in subs {
            for p in json_files(&s)? {
                g.lessons.push((rel(pack_root, &p), read(&p)?));
            }
        }
    }
    for p in json_files(&dir.join("sources"))? {
        g.sources.push(read(&p)?);
    }
    Ok(Some(g))
}

/// The counts the CLI and the JS validator print: sectors, occupations, competencies, items.
pub fn counts(g: &Graph) -> (usize, usize, usize, usize) {
    (g.sectors.len(), g.occupations.len(), g.competencies.len(), g.items.iter().map(|(_, f)| f.items.len()).sum())
}
