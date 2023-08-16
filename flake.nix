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
          packages = [ bin pkgs.postgresql_14 pkgs.gnutar pkgs.gzip pkgs.cacert ];
          devPackages = packages ++ [ pkgs.coreutils pkgs.bash ];
          dockerImage = pkgs.dockerTools.buildImage {
            name = "database-backup";
            tag = "latest";
            copyToRoot = packages;
            runAsRoot = ''
              mkdir /tmp
            '';
            config = {
              Env = [ "PATH=/bin" "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt" ];
              cmd = [ "${bin}/bin/database-backup" ];
            };
          };
          devImage = pkgs.dockerTools.buildImage {
            name = "database-backup";
            tag = "dev";
            copyToRoot = devPackages;
            runAsRoot = ''
              mkdir /tmp
            '';
            config = {
              Env = [ "PATH=/bin" "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt" ];
              cmd = [ "${bin}/bin/database-backup" ];
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

