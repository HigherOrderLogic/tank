// SPDX-License-Identifier: EUPL-1.2

use anyhow::{Context, Result, bail};

pub enum Command {
    Init {
        force: bool,
    },
    Update {
        names: Vec<String>,
        accept: bool,
    },
    Look {
        names: Vec<String>,
    },
    Add {
        name: String,
        url: String,
        flake: bool,
        dir: Option<String>,
        submodules: bool,
        follows: Vec<(String, String)>,
    },
    Rm {
        name: String,
    },
    Alias {
        name: String,
        template: Option<String>,
        rm: bool,
    },
    Help,
}

pub fn parse() -> Result<Command> {
    use lexopt::prelude::*;
    let mut p = lexopt::Parser::from_env();

    let sub = match p.next()? {
        Some(Value(v)) => v
            .string()
            .map_err(|_| anyhow::anyhow!("invalid subcommand"))?,
        Some(a) => return Err(a.unexpected().into()),
        None => return Ok(Command::Help),
    };

    match sub.as_str() {
        "init" => {
            let mut force = false;
            while let Some(a) = p.next()? {
                match a {
                    Long("force") => force = true,
                    _ => return Err(a.unexpected().into()),
                }
            }
            Ok(Command::Init { force })
        }
        "update" | "look" => {
            let mut names = Vec::new();
            let mut accept = false;
            while let Some(a) = p.next()? {
                match a {
                    Long("accept") if sub == "update" => accept = true,
                    Value(v) => names.push(v.string().map_err(|_| anyhow::anyhow!("bad name"))?),
                    _ => return Err(a.unexpected().into()),
                }
            }
            Ok(if sub == "update" {
                Command::Update { names, accept }
            } else {
                Command::Look { names }
            })
        }
        "add" => {
            let (mut name, mut url) = (None, None);
            let mut flake = true;
            let mut dir = None;
            let mut submodules = false;
            let mut follows = Vec::new();
            while let Some(a) = p.next()? {
                match a {
                    Long("no-flake") => flake = false,
                    Long("submodules") => submodules = true,
                    Long("dir") => {
                        dir = Some(
                            p.value()?
                                .string()
                                .map_err(|_| anyhow::anyhow!("bad dir"))?,
                        )
                    }
                    Long("follows") => {
                        let s = p
                            .value()?
                            .string()
                            .map_err(|_| anyhow::anyhow!("bad follows"))?;
                        follows.push(parse_follows(&s));
                    }
                    Value(v) => {
                        let s = v.string().map_err(|_| anyhow::anyhow!("bad argument"))?;
                        if name.is_none() {
                            name = Some(s);
                        } else if url.is_none() {
                            url = Some(s);
                        } else {
                            bail!("add takes at most <name> <url>");
                        }
                    }
                    _ => return Err(a.unexpected().into()),
                }
            }
            Ok(Command::Add {
                name: name.context("add: missing <name>")?,
                url: url.context("add: missing <url>")?,
                flake,
                dir,
                submodules,
                follows,
            })
        }
        "rm" => {
            let mut name = None;
            while let Some(a) = p.next()? {
                match a {
                    Value(v) if name.is_none() => {
                        name = Some(v.string().map_err(|_| anyhow::anyhow!("bad name"))?)
                    }
                    _ => return Err(a.unexpected().into()),
                }
            }
            Ok(Command::Rm {
                name: name.context("rm: missing <name>")?,
            })
        }
        "alias" => {
            let mut rm = false;
            let (mut name, mut template) = (None, None);
            while let Some(a) = p.next()? {
                match a {
                    Long("rm") => rm = true,
                    Value(v) => {
                        let s = v.string().map_err(|_| anyhow::anyhow!("bad argument"))?;
                        if name.is_none() {
                            name = Some(s);
                        } else if template.is_none() {
                            template = Some(s);
                        } else {
                            bail!("alias takes at most <name> <template>");
                        }
                    }
                    _ => return Err(a.unexpected().into()),
                }
            }
            let name = name.context("alias: missing <name>")?;
            if !rm && template.is_none() {
                bail!("alias: missing <template> (or pass --rm)");
            }
            Ok(Command::Alias { name, template, rm })
        }
        _ => Ok(Command::Help),
    }
}

/// child=parent, or bare child -> follows the same-named pin
fn parse_follows(s: &str) -> (String, String) {
    match s.split_once('=') {
        Some((c, p)) => (c.to_string(), p.to_string()),
        None => (s.to_string(), s.to_string()),
    }
}
