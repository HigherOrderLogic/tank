// SPDX-License-Identifier: EUPL-1.2

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table, value};

pub struct Input {
    pub name: String,
    pub url: String,
    pub submodules: bool,
}

pub fn load(path: &Path) -> Result<DocumentMut> {
    if !path.exists() {
        bail!("no pins.toml at {} (run `tack init`)", path.display());
    }
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    raw.parse()
        .with_context(|| format!("parse {}", path.display()))
}

pub fn save(path: &Path, doc: &DocumentMut) -> Result<()> {
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, doc.to_string())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn shorturls(doc: &DocumentMut) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(t) = doc.get("shorturls").and_then(Item::as_table) {
        for (k, v) in t {
            if let Some(s) = v.as_str() {
                out.insert(k.to_string(), s.to_string());
            }
        }
    }
    out
}

pub fn inputs(doc: &DocumentMut) -> Result<Vec<Input>> {
    let mut out = Vec::new();
    let Some(t) = doc.get("inputs").and_then(Item::as_table) else {
        return Ok(out);
    };
    for (name, item) in t {
        let table = item
            .as_table_like()
            .with_context(|| format!("input '{name}' is not a table"))?;
        let url = table
            .get("url")
            .and_then(Item::as_str)
            .with_context(|| format!("input '{name}' has no url"))?;
        out.push(Input {
            name: name.to_string(),
            url: url.to_string(),
            submodules: table
                .get("submodules")
                .and_then(Item::as_bool)
                .unwrap_or(false),
        });
    }
    Ok(out)
}

pub fn has_input(doc: &DocumentMut, name: &str) -> bool {
    doc.get("inputs")
        .and_then(Item::as_table)
        .is_some_and(|t| t.contains_key(name))
}

pub fn add_input(
    doc: &mut DocumentMut,
    name: &str,
    url: &str,
    flake: bool,
    dir: Option<&str>,
    submodules: bool,
    follows: &[(String, String)],
) {
    let mut t = Table::new();
    t.set_implicit(false);
    t["url"] = value(url);
    if !flake {
        t["flake"] = value(false);
    }
    if let Some(d) = dir {
        t["dir"] = value(d);
    }
    if submodules {
        t["submodules"] = value(true);
    }
    if !follows.is_empty() {
        let mut ft = Table::new();
        for (child, parent) in follows {
            ft[child] = value(parent.as_str());
        }
        t["follows"] = Item::Table(ft);
    }
    if doc.get("inputs").and_then(Item::as_table).is_none() {
        doc["inputs"] = Item::Table(Table::new());
    }
    doc["inputs"][name] = Item::Table(t);
}

pub fn remove_input(doc: &mut DocumentMut, name: &str) -> bool {
    doc.get_mut("inputs")
        .and_then(Item::as_table_mut)
        .and_then(|t| t.remove(name))
        .is_some()
}

pub fn set_alias(doc: &mut DocumentMut, name: &str, template: &str) {
    if doc.get("shorturls").and_then(Item::as_table).is_none() {
        doc["shorturls"] = Item::Table(Table::new());
    }
    doc["shorturls"][name] = value(template);
}

pub fn remove_alias(doc: &mut DocumentMut, name: &str) -> bool {
    doc.get_mut("shorturls")
        .and_then(Item::as_table_mut)
        .and_then(|t| t.remove(name))
        .is_some()
}
