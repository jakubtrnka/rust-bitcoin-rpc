{
  description = "bitcoind-rpc-client: JSON-RPC client for Bitcoin Core (v29 to v31)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
    }:
    let
      inherit (nixpkgs) lib;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = lib.genAttrs systems;

      mkCrate =
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          craneLib = crane.mkLib pkgs;

          # Rust sources plus the non-Rust files the crate reads at compile
          # time: README.md is the crate-level doc and tests/data holds the
          # PSBT fixtures pulled in with include_str!.
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              ./README.md
              ./tests/data
            ];
          };

          commonArgs = {
            inherit src;
            strictDeps = true;

            # The crate has no default features; build and check everything.
            cargoExtraArgs = "--all-features";

            # aws-lc-sys (pulled in through rustls under the tls feature)
            # compiles C code and may fall back to a CMake build.
            nativeBuildInputs = [
              pkgs.cmake
              pkgs.perl
            ];
            # The cmake setup hook must not hijack the cargo configure phase.
            dontUseCmakeConfigure = true;
          };

          # Build dependencies once so every check below reuses them.
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          crate = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              # Tests run in their own check derivation.
              doCheck = false;
            }
          );
        in
        {
          inherit
            pkgs
            craneLib
            src
            commonArgs
            cargoArtifacts
            crate
            ;
        };

      perSystem = forAllSystems mkCrate;
    in
    {
      packages = forAllSystems (system: {
        default = perSystem.${system}.crate;
      });

      checks = forAllSystems (
        system:
        let
          inherit (perSystem.${system})
            pkgs
            craneLib
            src
            commonArgs
            cargoArtifacts
            crate
            ;
        in
        {
          build = crate;

          # The crate must still compile with every feature switched off.
          build-no-features = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              pname = "bitcoind-rpc-client-no-features";
              cargoExtraArgs = "--no-default-features";
              doCheck = false;
            }
          );

          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          doc = craneLib.cargoDoc (
            commonArgs
            // {
              inherit cargoArtifacts;
              env.RUSTDOCFLAGS = "--deny warnings";
            }
          );

          fmt = craneLib.cargoFmt { inherit src; };

          # Unit, integration, and doc tests against the in-process mock
          # server. The live-node tests are #[ignore] and stay skipped here.
          test = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              # reqwest's rustls platform verifier loads the system root
              # store when the client is built. The Linux sandbox has none,
              # so point rustls-native-certs at the nixpkgs CA bundle.
              env.SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            }
          );
        }
      );

      devShells = forAllSystems (
        system:
        let
          inherit (perSystem.${system}) pkgs craneLib;
        in
        {
          default = craneLib.devShell {
            checks = self.checks.${system};
            packages = [
              pkgs.cargo-nextest
              pkgs.cargo-watch
            ];
          };
        }
      );

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt-rfc-style);
    };
}
