{
  # Heavily referenced https://fasterthanli.me/series/building-a-rust-service-with-nix/part-11
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
        flake-utils.follows = "flake-utils";
      };
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem
      (system:
      let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          # Tell crane to use the toolchain we created above
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          src = craneLib.cleanCargoSource ./.;
          nativeBuildInputs = with pkgs; [ rustToolchain rust-analyzer pkg-config ];
          buildInputs = with pkgs; [ ];
          commonArgs = {
            inherit src buildInputs nativeBuildInputs;
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          bin = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });
          packages = with pkgs; [ bin postgresql_16 gnutar gzip dockerTools.caCertificates bash ];
          devPackages = with pkgs; packages ++ [ coreutils ];
          dockerImage = pkgs.dockerTools.buildLayeredImage {
            name = "s3-postgres-backup";
            tag = "latest";
            contents = packages;
            extraCommands = ''
              mkdir tmp
            '';
            config = {
              Env = [ "PATH=/bin" ];
              Cmd = [ "s3-postgres-backup" ];
            };
          };
          # Might be able to use pkgs.dockerTools.mergeImages [] to avoid repeating myself here
          devImage = pkgs.dockerTools.buildLayeredImage {
            name = "s3-postgres-backup";
            tag = "dev";
            contents = devPackages;
            extraCommands = ''
              mkdir tmp
            '';
            config = {
              Env = [ "PATH=/bin" ];
              Cmd = [ "s3-postgres-backup" ];
            };
          };
        in
        with pkgs;
        {
          devShells.default = mkShell {
            inputsFrom = [ bin ];
          };
          packages = {
            inherit bin dockerImage devImage;
            default = devImage;
          };
        }
      );
}

