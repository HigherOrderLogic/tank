// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeMap;

/// expand scheme:rest via the {path} template; non-shorturl urls pass through
pub fn expand(url: &str, shorturls: &BTreeMap<String, String>) -> String {
    let Some((scheme, rest)) = url.split_once(':') else {
        return url.to_string();
    };
    let Some(template) = shorturls.get(scheme) else {
        return url.to_string();
    };
    normalize_git_ref(&template.replace("{path}", rest))
}

/// nix reads git+host/owner/repo/branch as a deeper path not a ref; remap the
/// trailing segment to ?ref=
fn normalize_git_ref(url: &str) -> String {
    if !url.starts_with("git+") || url.contains('?') {
        return url.to_string();
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let segs: Vec<&str> = rest.split('/').collect();
    if segs.len() < 4 {
        return url.to_string();
    }
    let (base, reff) = segs.split_at(segs.len() - 1);
    format!("{scheme}://{}?ref={}", base.join("/"), reff[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls() -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "atagen".into(),
                "git+https://git.lobotomise.me/atagen/{path}".into(),
            ),
            ("amaan".into(), "github:amaanq/{path}".into()),
            ("wry".into(), "git+ssh://forgejo@git.wry.land/{path}".into()),
        ])
    }

    #[test]
    fn passthrough_and_github() {
        let u = urls();
        assert_eq!(
            expand("github:NixOS/nixpkgs/nixos-unstable", &u),
            "github:NixOS/nixpkgs/nixos-unstable"
        );
        assert_eq!(
            expand("amaan:helium-flake", &u),
            "github:amaanq/helium-flake"
        );
    }

    #[test]
    fn git_triple_slash_remapped() {
        let u = urls();
        assert_eq!(
            expand("atagen:meat", &u),
            "git+https://git.lobotomise.me/atagen/meat"
        );
        assert_eq!(
            expand("wry:entailz/toes", &u),
            "git+ssh://forgejo@git.wry.land/entailz/toes"
        );
        assert_eq!(
            expand("atagen:proj/branch", &u),
            "git+https://git.lobotomise.me/atagen/proj?ref=branch"
        );
        assert_eq!(
            expand("wry:owner/repo/branch", &u),
            "git+ssh://forgejo@git.wry.land/owner/repo?ref=branch"
        );
    }

    #[test]
    fn existing_query_preserved() {
        let u = urls();
        assert_eq!(
            expand("wry:wry/wry?ref=anims-multiphase", &u),
            "git+ssh://forgejo@git.wry.land/wry/wry?ref=anims-multiphase"
        );
    }
}
