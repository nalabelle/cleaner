VERSION 0.8
FROM scratch
ARG --global REGISTRY="ghcr.io"
ARG --global PROJECT="nalabelle/cleaner"

# nix-flake pulls in the flake dependencies and build environment without including
# rust/cargo dependencies except the ones defined in flake.nix and devshell.toml
nix-flake:
  FROM nixos/nix:latest
  ENV NIX_CONFIG="experimental-features = nix-command flakes"

  COPY Cargo.toml devshell.toml flake.* /tmp/build/
  WORKDIR /tmp/build
  RUN nix build '.#devShell'
  SAVE IMAGE --cache-hint

# nix-deps imports the rust/Cargo.toml build dependencies
nix-deps:
  FROM +nix-flake
  COPY Cargo.lock .
  # Fake src because cargo won't work without a target
  RUN mkdir src && echo 'fn main() {}' > src/main.rs
  # Run an actual build on that trivial source file to compile the engine
  RUN nix develop --command cargo fetch
  RUN rm -rvf src
  SAVE IMAGE --cache-hint

# test runs the build tests
test:
  FROM +nix-deps
  COPY . ./
  RUN nix develop --command cargo-tarpaulin --engine llvm

build:
  FROM +nix-deps
  COPY . ./
  RUN nix develop --command build
