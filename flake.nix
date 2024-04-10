{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    devshell.url = "github:numtide/devshell";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, flake-utils, devshell, rust-overlay, ...}:
    flake-utils.lib.eachDefaultSystem (system:
      let
        rust_overlay = (import rust-overlay);
        manifest = (nixpkgs.lib.importTOML ./Cargo.toml).package;
        rust_bins = map (x: toString x) (builtins.attrNames (nixpkgs.lib.importTOML ./Cargo.toml).package.metadata.bin);
        dev_pkgs = import nixpkgs {
          inherit system rust_overlay;
          overlays = [ rust_overlay devshell.overlays.default ];
        };
        build_pkgs = import nixpkgs {
          inherit system rust_overlay;
          overlays = [ rust_overlay ];
        };
    in rec {
      packages.onlyDepsShell = dev_pkgs.devshell.mkShell {
        imports = [ (dev_pkgs.devshell.importTOML ./devshell.toml) ];
      };


      packages.devShell = dev_pkgs.devshell.mkShell {
        imports = [ (dev_pkgs.devshell.importTOML ./devshell.toml) ];
        # pull in config.toml bins
        devshell.packages = with dev_pkgs; rust_bins;
      };

      packages.build_metadata = build_pkgs.rustPlatform.buildRustPackage {
        pname = manifest.name;
        version = manifest.version;
        src = build_pkgs.lib.cleanSource ./.;

        nativeBuildInputs = [ build_pkgs.pkg-config ];
        buildInputs = with build_pkgs; [
          openssl
          cargo-binstall
          cargo-run-bin
        ];

        cargoLock.lockFile = ./Cargo.lock;
      };

      defaultPackage = packages.devShell;
    }
  );
}
