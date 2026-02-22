{
  description = "zscdoc Flake";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        webStuff = pkgs.buildNpmPackage {
          pname = "web_stuff";
          version = "1.0.0";
          src = ./web_stuff;
          npmDepsHash = "sha256-Ozv8V84y47D1GYQIwxkKbc/3khjE7ZOCH8WcqkLgnJw=";
          makeCacheWritable = true;
          forceGitDeps = true;
          installPhase = ''
            mkdir $out
            cp -R dist/* $out
          '';
        };
        zscdoc = pkgs.rustPlatform.buildRustPackage {
          pname = "zscdoc";
          version = "0.1.0";
          nativeBuildInputs = with pkgs; [
            pkg-config
            nodejs
          ];
          env = {
            WEB_STUFF_DIST_FOLDER = "${webStuff}";
          };
          buildInputs = with pkgs; [
            openssl
          ];
          src = ./.;
          cargoHash = "sha256-83KesHoke4Oml3XGiX0IFftkiy2zXVTQtFsH/j+WBAM=";
        };
      in
      {
        devShells.default =
          with pkgs;
          mkShell {
            packages = [
              rust-bin.stable.latest.default
              pkg-config
              openssl.dev
              nodejs
              python3
            ];
          };
        packages.zscdoc = zscdoc;
        packages.default = zscdoc;
      }
    );
}
