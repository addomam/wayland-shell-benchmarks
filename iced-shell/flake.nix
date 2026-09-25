{
  description = "Native iced Wayland shell comparison";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/567a49d1913ce81ac6e9582e3553dd90a95587";
  outputs = { self, nixpkgs }: let
    systems = [ "x86_64-linux" "aarch64-linux" ];
  in { devShells = nixpkgs.lib.genAttrs systems (system: let pkgs = import nixpkgs { inherit system; }; in {
    default = pkgs.mkShell {
      packages = with pkgs; [ cargo rustc rustfmt gcc pkg-config wayland libxkbcommon vulkan-loader ];
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [ wayland libxkbcommon vulkan-loader ]) + ":/run/opengl-driver/lib";
    };
  }); };
}
