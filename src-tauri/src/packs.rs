// Packs: what Claw'd is *for*. Each is a folder in the Clawd folder,
// `packs/<id>/`, holding PACK.md (plain-English instructions for the big
// brain: setup notes, rules for the sweep, jobs) and an optional pack.json
// (a name and a line for the gear). The built-in ones (office, home, dev) are
// written there at every start; anyone can add their own beside them.
//
// Which are on lives in `packs.json` in the Clawd folder, so a Claude that
// reads the folder (the Cowork fallback) sees it too. Missing means just the
// office pack: that's what every install before packs was.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// id, PACK.md, pack.json
const BUILTIN: &[(&str, &str, &str)] = &[
    ("office", include_str!("../../skill/clawd/packs/office/PACK.md"), include_str!("../../skill/clawd/packs/office/pack.json")),
    ("home", include_str!("../../skill/clawd/packs/home/PACK.md"), include_str!("../../skill/clawd/packs/home/pack.json")),
    ("dev", include_str!("../../skill/clawd/packs/dev/PACK.md"), include_str!("../../skill/clawd/packs/dev/pack.json")),
];

const DEFAULT: &[&str] = &["office"];
/// a pack is instructions, not a novel: anything past this is cut
const MAX_PACK_BYTES: usize = 40_000;

#[derive(Debug, Clone, Serialize)]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub icon: String,
    /// listens for Claude Code sessions (the dev pack)
    pub sessions: bool,
    pub builtin: bool,
    pub enabled: bool,
}

#[derive(Default, Deserialize)]
struct Meta {
    name: Option<String>,
    blurb: Option<String>,
    icon: Option<String>,
    #[serde(default)]
    sessions: bool,
}

#[derive(Serialize, Deserialize)]
struct Enabled {
    enabled: Vec<String>,
}

/// a pack id is a folder name: letters, digits, - and _ only
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Refresh the built-in packs in the Clawd folder (edits to them don't stick;
/// a pack of your own does).
pub fn write_builtins(dir: &Path) {
    for (id, md, meta) in BUILTIN {
        let p = dir.join("packs").join(id);
        let _ = std::fs::create_dir_all(&p);
        for (name, body) in [("PACK.md", *md), ("pack.json", *meta)] {
            let f = p.join(name);
            if std::fs::read_to_string(&f).map_or(true, |s| s != body) {
                let _ = std::fs::write(&f, body);
            }
        }
    }
}

pub fn enabled(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("packs.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Enabled>(s.trim_start_matches('\u{feff}')).ok())
        .map(|e| e.enabled.into_iter().filter(|id| valid_id(id)).collect())
        .unwrap_or_else(|| DEFAULT.iter().map(|s| s.to_string()).collect())
}

pub fn set_enabled(dir: &Path, ids: &[String]) -> bool {
    let mut list: Vec<String> = Vec::new();
    for id in ids {
        if valid_id(id) && !list.contains(id) {
            list.push(id.clone());
        }
    }
    let body = serde_json::to_string_pretty(&Enabled { enabled: list }).unwrap_or_default();
    std::fs::write(dir.join("packs.json"), body).is_ok()
}

fn read_pack(dir: &Path, id: &str) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("packs").join(id).join("PACK.md")).ok()?;
    let text = text.trim_start_matches('\u{feff}');
    Some(if text.len() > MAX_PACK_BYTES {
        let mut end = MAX_PACK_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text[..end].to_string()
    } else {
        text.to_string()
    })
}

/// Every pack in the folder (built-in or not), with whether it's on.
pub fn list(dir: &Path) -> Vec<PackInfo> {
    let on = enabled(dir);
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir.join("packs")) else { return out };
    for e in rd.flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        if !valid_id(&id) || !e.path().join("PACK.md").is_file() {
            continue;
        }
        let meta: Meta = std::fs::read_to_string(e.path().join("pack.json"))
            .ok()
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default();
        out.push(PackInfo {
            name: meta.name.unwrap_or_else(|| id.clone()),
            blurb: meta.blurb.unwrap_or_default(),
            icon: meta.icon.unwrap_or_default(),
            sessions: meta.sessions,
            builtin: BUILTIN.iter().any(|(b, _, _)| *b == id),
            enabled: on.contains(&id),
            id,
        });
    }
    // built-ins first, in their own order, then the rest by name
    out.sort_by_key(|p| (BUILTIN.iter().position(|(b, _, _)| *b == p.id).unwrap_or(usize::MAX), p.name.to_lowercase()));
    out
}

/// What the big brain is told on every run: the skill, then each enabled
/// pack's PACK.md.
pub fn system_prompt(dir: &Path, skill: &str) -> String {
    let on = enabled(dir);
    let mut s = String::from(skill);
    let packs: Vec<(String, String)> = on.iter().filter_map(|id| read_pack(dir, id).map(|t| (id.clone(), t))).collect();
    s.push_str("\n\n---\n\n# Enabled packs: ");
    if packs.is_empty() {
        s.push_str("none (only custom rules and requests)\n");
    } else {
        s.push_str(&packs.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>().join(", "));
        s.push('\n');
        for (id, text) in packs {
            s.push_str(&format!("\n<!-- pack: {id} -->\n\n{}\n", text.trim()));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("clawd-packs-{}-{}", std::process::id(), crate::feed::new_token()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn old_installs_get_the_office_pack() {
        let d = temp();
        write_builtins(&d);
        assert_eq!(enabled(&d), vec!["office".to_string()]);
        let p = system_prompt(&d, "SKILL");
        assert!(p.starts_with("SKILL"));
        assert!(p.contains("# Office pack"));
        assert!(!p.contains("# Home pack"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn only_enabled_packs_are_sent_and_custom_ones_list() {
        let d = temp();
        write_builtins(&d);
        std::fs::create_dir_all(d.join("packs").join("minecraft")).unwrap();
        std::fs::write(d.join("packs").join("minecraft").join("PACK.md"), "# Minecraft\nTell me when the server empties.").unwrap();
        assert!(set_enabled(&d, &["home".into(), "minecraft".into(), "../evil".into(), "home".into()]));
        assert_eq!(enabled(&d), vec!["home".to_string(), "minecraft".to_string()]);
        let p = system_prompt(&d, "SKILL");
        assert!(p.contains("# Home pack") && p.contains("server empties") && !p.contains("# Office pack"));
        let l = list(&d);
        let ids: Vec<&str> = l.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["office", "home", "dev", "minecraft"]);
        assert!(l.iter().find(|p| p.id == "dev").unwrap().sessions);
        assert!(!l.iter().find(|p| p.id == "minecraft").unwrap().builtin);
        let _ = std::fs::remove_dir_all(&d);
    }
}
