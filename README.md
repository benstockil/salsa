# Salsa

Salsa is a small declarative language for choreographing stage movement. You
describe a cast of dancers, where each one starts, and how they move across a
rectangular stage over musical time (bars and beats). Salsa parses the script
and plays it back in a 2‑D viewer that animates every dancer along their path.

```salsa
meta {
  title "Demo"
  bpm 120
  barlength 4
  stagesize (30, 20)
}

dancers {
  B is "Ben" in RED;
}

start B at (0, 0);

let hs = horseshoe {
  about (0, 0)
  closure 0.5
  width 10
  depth 10
}

at 0:0 move B to hs[1/8];
at 0:1 move B to hs[2/8];
at 1:3 move B to hs[8/8] until 4:0;
```

Writing out choreography by hand is fiddly and hard to visualise.
Salsa lets you write that intent directly and _watch it_, so you can
iterate on formations and timing without a studio full of people.

See [SPECIFICATION.md](SPECIFICATION.md) for the full grammar and semantics.

## Building and running

Salsa is implemented in Rust. The viewer uses [nannou](https://nannou.cc/), so
you'll need the usual Rust toolchain and a working graphics stack.

```sh
# build
cargo build --release

# run a script in the viewer
cargo run -- demo.salsa
```

A window opens and plays the dance from the start. If the script has errors,
Salsa prints rustc‑style diagnostics pointing at the offending line and column
instead of opening the viewer:

```text
error: expected metadata property, got Ident("wobble")
 --> demo.salsa:4:3
  |
4 |   wobble 5
  |   ^^^^^^
```

## File extension

Salsa scripts use the `.salsa` extension.

## Status

Salsa is an early prototype (v0.1). Expect rough edges:

- `rotate` is reserved in the grammar but **not implemented**.
- Some constraints are only checked at playback time rather than at parse time -
  notably, every moved dancer needs a start position, and a dancer's final
  movement needs an explicit `until` beat.

## Project layout

```
src/
  parser/      lexer, tokens, parser, and diagnostics
  dance/       the dance model (metadata, dancers, shapes, steps)
  iterator.rs  resolves scripts into per-dancer paths and samples them over time
  main.rs      CLI entry point + nannou viewer
  color.rs     the color palette
  coordinates.rs
```
