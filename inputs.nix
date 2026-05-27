# SPDX-License-Identifier: EUPL-1.2

let
  pins = builtins.fromTOML (builtins.readFile ./pins.toml);
  lock = builtins.fromJSON (builtins.readFile ./pins.lock.json);
  all_follow = pins.all_follow or {};

  fetchPin = name: builtins.fetchTree lock.${name};

  resolveSpec =
    upLock: spec:
    if builtins.isList spec then walkPath upLock upLock.root spec else spec;

  walkPath =
    upLock: nodeName: path:
    if path == [ ] then
      nodeName
    else
      walkPath upLock (resolveSpec upLock upLock.nodes.${nodeName}.inputs.${builtins.head path}) (
        builtins.tail path
      );

  evalTransitive =
    upLock: nodeName: sourceInfo:
    let
      raw = import (sourceInfo.outPath + "/flake.nix");
      node = upLock.nodes.${nodeName};
      callerInputs = builtins.mapAttrs (
        n: _decl:
        let
          ref =
            (node.inputs or { }).${n}
              or (throw "tack/inputs.nix: transitive '${n}' missing in flake.lock node '${nodeName}'");
          childName = resolveSpec upLock ref;
          childNode = upLock.nodes.${childName};
          childSrc = builtins.fetchTree childNode.locked;
        in
        if childNode.flake or true then evalTransitive upLock childName childSrc else childSrc
      ) (raw.inputs or { });
      outputs = raw.outputs (callerInputs // { self = result; });
      result = outputs // sourceInfo // {
        outPath = sourceInfo.outPath;
        inputs = callerInputs;
        inherit outputs;
        inherit sourceInfo;
        _type = "flake";
      };
    in
    result;

  evalTopFlake =
    sourceInfo: pin:
    let
      raw = import (sourceInfo.outPath + "/flake.nix");
      upLockPath = sourceInfo.outPath + "/flake.lock";
      upLock =
        if builtins.pathExists upLockPath then builtins.fromJSON (builtins.readFile upLockPath) else null;

      exclude_follow = pin.exclude_follow or [];
      explicit_follows = pin.follows or { };
      all_follow_rules = builtins.filterAttrs
        (name: _target: !(builtins.elem name exclude_follow))
        all_follow;
      combined_follows = explicit_follows // all_follow_rules;
      overrides = builtins.mapAttrs (_: target: self.${target}) combined_follows;

      callerInputs = builtins.mapAttrs (
        n: _decl:
        if overrides ? ${n} then
          overrides.${n}
        else if upLock != null then
          let
            ref =
              (upLock.nodes.${upLock.root}.inputs or { }).${n}
                or (throw "tack/inputs.nix: input '${n}' declared but not in flake.lock at ${toString sourceInfo.outPath}");
            childName = resolveSpec upLock ref;
            childNode = upLock.nodes.${childName};
            childSrc = builtins.fetchTree childNode.locked;
          in
          if childNode.flake or true then evalTransitive upLock childName childSrc else childSrc
        else
          throw "tack/inputs.nix: no flake.lock at ${toString sourceInfo.outPath}; cannot resolve '${n}'"
      ) (raw.inputs or { });

      outputs = raw.outputs (callerInputs // { self = result; });
      result = outputs // sourceInfo // {
        outPath = sourceInfo.outPath;
        inputs = callerInputs;
        inherit outputs;
        inherit sourceInfo;
        _type = "flake";
      };
    in
    result;

  loadPin =
    name: pin:
    let
      sourceInfo = fetchPin name;
    in
    if pin.flake or true then evalTopFlake sourceInfo pin else sourceInfo.outPath;

  self = builtins.mapAttrs loadPin pins.inputs;
in
self
