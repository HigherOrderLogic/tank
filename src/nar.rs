// SPDX-License-Identifier: EUPL-1.2

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// sri nar hash of `root`, matching builtins.fetchTree
pub fn hash_path(root: &Path) -> Result<String> {
    let mut h = Sha256::new();
    emit_bytes(&mut h, b"nix-archive-1");
    emit_node(&mut h, root)?;
    Ok(format!("sha256-{}", b64(&h.finalize())))
}

fn emit_node(h: &mut Sha256, path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path).with_context(|| format!("stat {}", path.display()))?;
    emit_bytes(h, b"(");
    if meta.is_symlink() {
        let target = fs::read_link(path)?;
        emit_bytes(h, b"type");
        emit_bytes(h, b"symlink");
        emit_bytes(h, b"target");
        emit_bytes(h, target.as_os_str().as_bytes());
    } else if meta.is_dir() {
        emit_bytes(h, b"type");
        emit_bytes(h, b"directory");
        let mut entries: Vec<_> = fs::read_dir(path)?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<std::io::Result<_>>()?;
        entries.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        for name in entries {
            emit_bytes(h, b"entry");
            emit_bytes(h, b"(");
            emit_bytes(h, b"name");
            emit_bytes(h, name.as_bytes());
            emit_bytes(h, b"node");
            emit_node(h, &path.join(&name))?;
            emit_bytes(h, b")");
        }
    } else {
        emit_bytes(h, b"type");
        emit_bytes(h, b"regular");
        if meta.permissions().mode() & 0o111 != 0 {
            emit_bytes(h, b"executable");
            emit_bytes(h, b"");
        }
        emit_bytes(h, b"contents");
        emit_contents(h, path, meta.len())?;
    }
    emit_bytes(h, b")");
    Ok(())
}

fn emit_contents(h: &mut Sha256, path: &Path, len: u64) -> Result<()> {
    h.update(len.to_le_bytes());
    let mut f = fs::File::open(path)?;
    let mut buf = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        total += n as u64;
    }
    if total != len {
        anyhow::bail!("{} changed size during hashing", path.display());
    }
    pad(h, len);
    Ok(())
}

fn emit_bytes(h: &mut Sha256, b: &[u8]) {
    h.update((b.len() as u64).to_le_bytes());
    h.update(b);
    pad(h, b.len() as u64);
}

fn pad(h: &mut Sha256, len: u64) {
    let rem = (len % 8) as usize;
    if rem != 0 {
        h.update(&[0u8; 8][..8 - rem]);
    }
}

fn b64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(B64[(n >> 18 & 63) as usize] as char);
        out.push(B64[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
