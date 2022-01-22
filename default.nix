with import <nixpkgs> {};

stdenv.mkDerivation rec {
    name = "env";
    env = buildEnv { name = name; paths = buildInputs; };
    buildInputs = [
        asdf
	clang
	nim
        emacs
        # Elixir + Erlang
        erlang
        elixir
        # Haskell
        ghc
        haskellPackages.Cabal_3_4_0_0
        cabal-install
        # Idris 
        idris
        # JavaScript
        yarn
        # Lisp
        sbcl
        # Rust
        cargo
        rustc
        rustfmt
        # Scheme
        chicken
    ];

    # Load env vars in shell.
    shellHook = ''set -a; source .env; sh hack.sh;'';
}
