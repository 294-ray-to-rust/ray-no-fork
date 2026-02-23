{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
          isLinux = pkgs.stdenv.isLinux;

          pythonEnv = pkgs.python312.withPackages (ps: [
            ps.cython
            ps.setuptools
            ps.pip
            ps.wheel
            ps.setuptools-scm
            ps.pybind11
            ps.numpy
          ]);
        in
        {
          default = pkgs.mkShell {
            packages = [
              # Orchestrator tools
              pkgs.opencode
              pkgs.git
              pkgs.gh
              pkgs.codex
              pkgs.claude-code

              # Python
              pythonEnv

              # Java
              pkgs.jdk11
              pkgs.maven

              # C/C++ build toolchain
              pkgs.cmake
              pkgs.ninja
              pkgs.gnumake
              pkgs.gcc
              pkgs.clang

              # Bazel
              pkgs.bazel_7

              # Node.js (dashboard)
              pkgs.nodejs

              # System libraries
              pkgs.zlib
              pkgs.openssl
              pkgs.curl
              pkgs.protobuf

              # Utilities
              pkgs.wget
              pkgs.unzip
              pkgs.rsync
              pkgs.tmux
              pkgs.gnupg
              pkgs.patchelf
            ] ++ pkgs.lib.optionals isLinux [
              pkgs.jemalloc
              pkgs.libunwind
            ];
          };
        }
      );
    };
}
