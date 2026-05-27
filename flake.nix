# SPDX-License-Identifier: EUPL-1.2

{
  description = "flake-like toml nix pins, lazily fetched and transformed";

  outputs =
    { self }:
    let
      nixpkgs = (import ./inputs.nix).nixpkgs;
      inherit (nixpkgs) lib;
      forAllSystems = lib.genAttrs lib.systems.flakeExposed;
      pkgsFor = system: nixpkgs.legacyPackages.${system};

      nativeDeps = pkgs: [ pkgs.pkg-config ];
      linkDeps = pkgs: [
        pkgs.openssl
        pkgs.libgit2
        pkgs.libssh2
        pkgs.zlib
      ];
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = self.packages.${system}.tack;

          tack = pkgs.rustPlatform.buildRustPackage {
            pname = "tack";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = nativeDeps pkgs;
            buildInputs = linkDeps pkgs;

            # link nixpkgs c libs, no vendored copies
            env = {
              LIBGIT2_NO_VENDOR = 1;
              OPENSSL_NO_VENDOR = 1;
            };

            meta = {
              description = "flake-like toml nix pins, lazily fetched and transformed";
              mainProgram = "tack";
            };
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.clippy
              pkgs.rustfmt
              pkgs.rust-analyzer
            ]
            ++ nativeDeps pkgs
            ++ linkDeps pkgs;

            env = {
              RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
              LIBGIT2_NO_VENDOR = 1;
              OPENSSL_NO_VENDOR = 1;
            };
          };
        }
      );
    };
}
