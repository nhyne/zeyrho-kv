{
  description = "Shell for grocery";

  inputs = {
    nixpkgs.url = "nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, flake-compat }:

    flake-utils.lib.eachDefaultSystem (system:
      let 
        pkgs = nixpkgs.legacyPackages.${system};
      in rec {
        devShell = pkgs.mkShell {
#          inherit (pkgs.stdenv.hostPlatform) isDarwin;
          buildInputs = with pkgs; [
            cargo
            gnumake
            gcc
            ghz
            gnuplot
            graphviz
            grpcurl
            protobuf
            rustup
          ];

          shellHook = if pkgs.stdenv.hostPlatform.isDarwin then ''
            export LIBRARY_PATH=$LIBRARY_PATH:$(brew --prefix)/lib:$(brew --prefix)/opt/libiconv/lib
          '' else "";
        };
      }
  
    );

}
