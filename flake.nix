{
  description = "Starshell, a Wayland desktop shell built with GPUI";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          inherit (pkgs) lib;
          runtimeLibraries = with pkgs; [
            wayland
            libxkbcommon
            vulkan-loader
            libGL
            fontconfig
            freetype
            libxcb
          ];
          starshell = pkgs.rustPlatform.buildRustPackage {
            pname = "starshell";
            version = (builtins.fromTOML (builtins.readFile ./crates/app/Cargo.toml)).package.version;
            src = lib.cleanSource self;

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "gpui-ce-0.2.2" = "sha256-Ogtb6cug4JnIoKkYkhY7Cl8MRoghXIvJQ3DVvAftuB0=";
                "wgsl-rs-0.1.0" = "sha256-M+YP+viw8PADmxr92qg/6C3WweXBRvOTDuS1RKvzoBA=";
              };
            };

            cargoBuildFlags = [ "--package" "starshell" "--bin" "starshell" ];
            cargoTestFlags = [ "--package" "starshell" ];

            nativeBuildInputs = with pkgs; [
              pkg-config
              makeWrapper
              rustPlatform.bindgenHook
            ];
            buildInputs = runtimeLibraries ++ (with pkgs; [
              openssl
              dbus
              alsa-lib
              pam
            ]);

            postFixup = ''
              wrapProgram "$out/bin/starshell" \
                --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibraries}
            '';

            meta = {
              description = "A Wayland desktop shell built with GPUI";
              homepage = "https://github.com/moiCR/Capsule";
              license = lib.licenses.mit;
              mainProgram = "starshell";
              platforms = systems;
            };
          };
        in
        {
          inherit starshell;
          default = starshell;
        });
    };
}
