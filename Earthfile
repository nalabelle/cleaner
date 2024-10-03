VERSION 0.8
FROM scratch
ARG --global REGISTRY="ghcr.io"
ARG --global PROJECT="nalabelle/cleaner"

deps:
  FROM docker.io/jetpackio/devbox:latest@sha256:089092c0a2bddf9811fb1496b839ce36f83f84db4bdc59d4dae076f1d3923138
  # Installing your devbox project
  WORKDIR /code
  USER root:root
  RUN chown ${DEVBOX_USER}:${DEVBOX_USER} /code
  USER ${DEVBOX_USER}:${DEVBOX_USER}
  COPY --chown=${DEVBOX_USER}:${DEVBOX_USER} devbox.json devbox.json
  COPY --chown=${DEVBOX_USER}:${DEVBOX_USER} devbox.lock devbox.lock
  RUN --secret GITHUB_TOKEN devbox run -- echo "Installed Packages."

  COPY rust-toolchain.toml .
  # Installs multiple rust targets
  RUN devbox run rustup show
  COPY Cargo.* bin/configure-rust .

  # Fake src because cargo won't work without a target
  RUN mkdir src && echo 'fn main() {}' > src/main.rs
  # Run an actual build on that trivial source file to compile the engine
  RUN devbox run ./configure-rust
  RUN devbox run cargo fetch
  RUN rm -rvf src
  SAVE IMAGE --push $REGISTRY/$PROJECT/deps:latest

# test runs the build tests
test:
  FROM +deps
  COPY --dir src .
  RUN devbox run cargo test
#  RUN devbox run cargo tarpaulin --engine llvm

linux:
  FROM +deps
  ARG RUST_TARGET=x86_64-unknown-linux-gnu
  COPY --dir src .
  RUN devbox run cargo build -r --target=$RUST_TARGET
  RUN devbox run tar cJf cleaner-$RUST_TARGET.tar.xz target/$RUST_TARGET/release/cleaner
  SAVE ARTIFACT --keep-ts cleaner-$RUST_TARGET.tar.xz AS LOCAL dist/cleaner-$RUST_TARGET.tar.xz

win-deps:
  FROM +deps
  # I got a bunch of this from cross-rs/cross - I don't understand it fully yet
  USER root:root
  RUN dpkg --add-architecture i386
  RUN apt-get update && apt-get install --assume-yes --no-install-recommends libz-mingw-w64-dev g++-mingw-w64-x86-64
  ENV CROSS_TOOLCHAIN_PREFIX=x86_64-w64-mingw32-
  ENV CROSS_TOOLCHAIN_SUFFIX=-posix
  ENV CROSS_SYSROOT=/usr/x86_64-w64-mingw32
  ENV CROSS_TARGET_RUNNER="env -u CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER"
  ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$CROSS_TOOLCHAIN_PREFIX"gcc"$CROSS_TOOLCHAIN_SUFFIX"
  ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER="$CROSS_TARGET_RUNNER"
  ENV AR_x86_64_pc_windows_gnu="$CROSS_TOOLCHAIN_PREFIX"ar
  ENV CC_x86_64_pc_windows_gnu="$CROSS_TOOLCHAIN_PREFIX"gcc"$CROSS_TOOLCHAIN_SUFFIX"
  ENV CXX_x86_64_pc_windows_gnu="$CROSS_TOOLCHAIN_PREFIX"g++"$CROSS_TOOLCHAIN_SUFFIX"
  ENV BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_gnu="--sysroot=$CROSS_SYSROOT -idirafter/usr/include"
  ENV CROSS_CMAKE_SYSTEM_NAME=Windows
  ENV CROSS_CMAKE_SYSTEM_PROCESSOR=AMD64
  ENV CROSS_CMAKE_CRT=gnu
  ENV CROSS_CMAKE_OBJECT_FLAGS="-ffunction-sections -fdata-sections -m64"
  USER ${DEVBOX_USER}:${DEVBOX_USER}
  SAVE IMAGE --push $REGISTRY/$PROJECT/win-deps:latest

win:
  FROM +win-deps
  ARG RUST_TARGET=x86_64-pc-windows-gnu
  COPY --dir src .
  RUN devbox run cargo build -r --target=$RUST_TARGET
  RUN devbox run tar cJf cleaner-$RUST_TARGET.tar.xz target/$RUST_TARGET/release/cleaner.exe
  SAVE ARTIFACT --keep-ts cleaner-$RUST_TARGET.tar.xz AS LOCAL dist/cleaner-$RUST_TARGET.tar.xz

build:
  BUILD +test
  BUILD +linux
  BUILD +win
