# The Salsa Language Specification

Version 0.1 - reference implementation in [`src/`](src/).

Salsa is a small declarative language for describing stage choreography: who is
dancing, where they begin, and how they move across a rectangular stage over
musical time. A Salsa program (`*.salsa`) is parsed into a dance script and
played back by a 2D viewer that animates each dancer along their resolved path.

This document specifies the lexical structure, grammar, and semantics of the
language as implemented by the reference parser.

---

## 1. Program structure

A program consists of four sections that **must appear in this order**:

```
meta { ... }        (1) metadata          - required
dancers { ... }     (2) cast              - required (may be empty)
start ...           (3) initial positions - required
<statements>        (4) choreography body - zero or more
```

Formally:

```ebnf
program   = meta dancers start { statement } ;
```

Whitespace (spaces, tabs, newlines) is insignificant except as a token
separator. There is no statement list separator between the four sections; each
section is delimited by its own braces or terminators.

---

## 2. Lexical structure

### 2.1 Comments

Only single‑line comments are supported. A `//` begins a comment that runs to
the end of the line. There are no block comments.

```salsa
// this is a comment
```

### 2.2 Identifiers

An identifier is a run of **alphabetic characters only** (`A`-`Z`, `a`-`z`). It
contains no digits, underscores, or other symbols.

Identifiers are used for dancer handles (e.g. `A`, `Bob`) and shape names
(e.g. `hs`). They are case‑sensitive.

### 2.3 Keywords

The following lowercase words are reserved and cannot be used as identifiers:

```
all  along  at  barlength  bars  beats  bpm  by  dancers  every
except  in  is  let  meta  move  rotate  stagesize  start  title  to  until
```

(`bars` is reserved but not currently used by any production.)

### 2.4 Colors

Colors are written in **uppercase** and are a fixed set of keywords:

```
RED  ORANGE  YELLOW  GREEN  DARKGREEN  BLUE  DARKBLUE  PURPLE  PINK  WHITE  GREY
```

A color is recognised lexically; any other uppercase word is just an identifier.

### 2.5 Numbers

| Form             | Example      | Token | Notes                                                                                                          |
| ---------------- | ------------ | ----- | -------------------------------------------------------------------------------------------------------------- |
| Unsigned integer | `120`        | uint  | A run of digits.                                                                                               |
| Float            | `0.5`        | float | Digits, `.`, digits.                                                                                           |
| Negative number  | `-7`, `-2.5` | float | A leading `-` is only valid immediately before digits; the result is always a float (so `-7` lexes as `-7.0`). |

Where a _float_ is expected, an unsigned integer is also accepted and widened.
There is no general arithmetic; `-` is only the sign of a numeric literal.

### 2.6 Beats (time literals)

A beat literal is written `bar:beat`, two digit runs separated by a colon:

```
0:0   1:3   8:1
```

Both components are integers. Bars and beats are **zero‑indexed**: with a
`barlength` of 4, the beats of bar 0 are `0:0 0:1 0:2 0:3`, and `1:0` is the
first beat of the second bar. See §5.1 for the mapping to seconds.

### 2.7 String literals

A string is any sequence of characters between double quotes: `"Hello World"`.
There are no escape sequences; a string runs to the next `"`.
An unterminated string is a lexing error.

### 2.8 Punctuation

```
{ }   ( )   [ ]   ,   :   ;   =   /
```

---

## 3. Sections

### 3.1 `meta`

```ebnf
meta      = "meta" "{" { meta_prop } "}" ;
meta_prop = "title"     string
          | "bpm"       number
          | "barlength" uint
          | "stagesize" "(" uint "," uint ")" ;
```

All four properties are **required** and each may be set **at most once**;
setting a property twice, or omitting any, is an error. Properties may appear in
any order and are not separated by punctuation.

- `title` - the dance's display name.
- `bpm` - tempo in beats per minute (drives the beat→time mapping).
- `barlength` - number of beats per bar.
- `stagesize` - stage `(width, depth)` in grid units.

```salsa
meta {
  title "Demo"
  bpm 120
  barlength 4
  stagesize (30, 20)
}
```

### 3.2 `dancers`

```ebnf
dancers     = "dancers" "{" { dancer_decl } "}" ;
dancer_decl = ident "is" string "in" color ";" ;
```

Each declaration binds an identifier (the handle used throughout the rest of the
program) to a display name and a color. Declaring the same handle twice is an
error.

```salsa
dancers {
  A is "Alice" in RED;
  Bob is "Bob" in ORANGE;
}
```

### 3.3 `start`

```ebnf
start      = "start" ( "{" { start_line } "}" | start_line ) ;
start_line = dancer_list "along" shape        (* distribute along a shape *)
           | dancer "at" position ";" ;       (* place a single dancer    *)
```

The `start` section gives every dancer an initial position. It is either a
single line or a `{ ... }` block of lines.

- `<dancers> along <shape>` evenly distributes the listed dancers across the
  shape's slots, in list order (first dancer at slot 0, last at slot 1; see
  5.3).
- `<dancer> at <position>` places one dancer at a specific position.

A dancer may be given a start position **only once**. Every dancer that is
later moved must have been started, or playback fails (see 5.5).

```salsa
start [A, B, C] along line from (0.0, 5.0) through (0.0, -7.0);
```

```salsa
start B at (0, 0);
```

---

## 4. Statements

The body is a sequence of `at` steps and `let` shape definitions, in any order.

```ebnf
statement = step | shape_def ;
```

### 4.1 `let` — named shapes

```ebnf
shape_def = "let" ident "=" shape_literal ;
```

Binds a name to a shape so it can be referenced by `start ... along`, `move ...
along`, or indexed positions like `hs[1/12]`. A `let` is a definition only; it
produces no movement.

```salsa
let hs = horseshoe {
  about (0, 10)
  closure 0.5
  width 10
  depth 10
}
```

### 4.2 `at` — steps

```ebnf
step        = "at" beat ( "{" { instruction } "}" | instruction ) ;
instruction = movement [ "until" beat ] ";" ;
```

A step attaches one or more instructions to a beat - the moment the movement
begins. A step is either a single instruction or a `{ ... }` block of
instructions that all start at the same beat.

```salsa
at 1:3 move B to hs[8/8] until 4:0;
```

```salsa
at 1:1 {
  move all except [B, Ji, E] by (0, 1) every 2 beats;
  move Ji to hs[1/12] until 8:1;
}
```

### 4.3 Movements

```ebnf
movement    = "move" dancer_spec "along" shape
            | "move" dancer_spec "to"    position
            | "move" dancer_spec "by"    coords [ "every" number "beats" ]
            | "rotate" ... ;                        (* reserved - see 4.6 *)

dancer_spec = "all" [ "except" dancer_list ]
            | dancer_list ;
dancer_list = ident | "[" [ ident { "," ident } ] "]" ;
```

- **`move <dancers> along <shape>`** - moves each listed dancer to its slot on
  the shape. The dancers must be an explicit list (not `all`).
- **`move <dancer> to <position>`** - moves a single dancer to a position. The
  specifier must name exactly one dancer.
- **`move <dancers> by (dx, dy)`** - translates the dancers by a delta. With the
  optional `every <n> beats` clause the delta is applied repeatedly, producing a
  continuous drift of `(dx, dy)` per `n` beats across the movement's duration
  (see 5.4). `all`/`all except ...` is permitted here.

### 4.4 Positions

```ebnf
position = ident "[" uint "/" uint "]"    (* slot on a named shape *)
         | coords ;                        (* literal coordinates   *)
coords   = "(" number "," number ")" ;
```

`shape[i/n]` selects slot _i_ of _n_ evenly spaced slots on a shape. **Slot
indices are 1‑based**: `hs[1/8]` is the first of eight slots and `hs[8/8]` the
last (see 5.3).

### 4.5 Shapes

```ebnf
shape         = ("line" | "horseshoe") shape_body   (* inline literal *)
              | ident ";" ;                         (* reference by name *)
shape_literal = ("line" | "horseshoe") shape_body ;
shape_body    = "{" { shape_prop } "}"              (* braced *)
              | { shape_prop } ";" ;                (* bare, semicolon-terminated *)
```

Two shape kinds are defined:

**`line`** - a straight segment.

| Property  | Type   | Meaning             |
| --------- | ------ | ------------------- |
| `from`    | coords | One endpoint.       |
| `through` | coords | The other endpoint. |

**`horseshoe`** - a parametric arc (see 5.3).

| Property  | Type   | Meaning                                         |
| --------- | ------ | ----------------------------------------------- |
| `about`   | coords | Center of the arc.                              |
| `closure` | float  | How closed the arc is (0 = full ellipse sweep). |
| `width`   | float  | Full width of the bounding ellipse.             |
| `depth`   | float  | Full depth of the bounding ellipse.             |

Every property of a shape is required and may be set at most once. A shape body
is either braced or a bare list terminated by `;`.

### 4.6 `rotate` (reserved)

The `rotate` keyword is reserved and recognised by the grammar but is **not
implemented**; using it is a parse-time error.

---

## 5. Semantics

### 5.1 Time

Beats map to seconds using the metadata tempo and bar length:

```
beat_length = 60 / bpm                       (seconds per beat)
time(bar:beat) = (barlength * bar + beat) * beat_length
```

So at `bpm 120`, `barlength 4`, the beat `1:2` is `(4 * 1 + 2) * 0.5 = 3.0`
seconds.

### 5.2 Coordinate system

Positions are `(x, y)` pairs in stage grid units. The stage is `stagesize
(width, depth)` units; coordinates are not clamped to it (dancers may be placed
or moved off‑stage). The viewer draws the stage centered, with `+y` rendered
toward the bottom of the window.

### 5.3 Shape slots

A shape maps a slot _i_ of _n_ slots to a parameter `amount = i / (n − 1)`,
ranging from `0` at the first slot to `1` at the last.

- **Line:** linear interpolation, `from * (1 - amount) + through * amount`.
- **Horseshoe:** with `a = amount * (1 - closure) + 0.5` and `t = 2pi * a`,
  the point is `about + (0.5 * width * cos(t), 0.5 * depth * sin(t))`.

Two slot conventions coexist:

- `start ... along <shape>` and `move ... along <shape>` distribute dancers with
  **0‑based** list indices (`i = 0 ... n-1` over `n` dancers).
- An indexed position `shape[i/n]` is **1‑based** (`i = 1 ... n`), internally
  mapped to slot `i - 1` of `n`.

### 5.4 Movement resolution

Each dancer's path is resolved independently from the steps that mention them (a
step "pertains to" a dancer if any of its instructions reference that dancer).
For each such instruction:

- **Start time** is the step's beat.
- **End time** is the instruction's `until` beat if present; otherwise the beat
  of that dancer's _next_ pertaining step.
- The dancer moves **linearly** from its current position to the instruction's
  target over `[start, end]`, with progress clamped to `[0, 1]`.

Movement targets:

- `to <position>` / `along <shape>`: an absolute destination.
- `by (dx, dy)` with no `every`: the destination is `origin + (dx, dy)`.
- `by (dx, dy) every <n> beats`: the destination is `origin + (dx, dy) ·
(duration_in_beats / n)`, i.e. a steady drift of the delta per `n` beats.

### 5.5 Requirements and runtime errors

The following are not caught by the parser but cause playback to fail:

- **Unstarted dancer:** every dancer referenced in the body must have a start
  position.
- **Open‑ended final movement:** a dancer's _last_ movement must have an
  `until` beat, since there is no following step to supply an end time.

### 5.6 Playback

The resolved paths are sampled over wall-clock time (seconds since start) to
produce, for every frame, the position of each dancer. Dancers are drawn as
colored discs on the stage grid.

---

## 6. Worked example

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
at 0:2 move B to hs[3/8];
at 0:3 move B to hs[4/8];
at 1:0 move B to hs[5/8];
at 1:1 move B to hs[6/8];
at 1:2 move B to hs[7/8];
at 1:3 move B to hs[8/8] until 4:0;
```

Ben starts at the origin and steps through the eight slots of a horseshoe, one
per beat, holding the final slot until the start of bar 4.

---

## 7. Grammar summary

```ebnf
program     = meta dancers start { statement } ;

meta        = "meta" "{" { meta_prop } "}" ;
meta_prop   = "title" string
            | "bpm" number
            | "barlength" uint
            | "stagesize" "(" uint "," uint ")" ;

dancers     = "dancers" "{" { dancer_decl } "}" ;
dancer_decl = ident "is" string "in" color ";" ;

start       = "start" ( "{" { start_line } "}" | start_line ) ;
start_line  = dancer_list "along" shape
            | dancer "at" position ";" ;

statement   = step | shape_def ;
shape_def   = "let" ident "=" shape_literal ;
step        = "at" beat ( "{" { instruction } "}" | instruction ) ;
instruction = movement [ "until" beat ] ";" ;

movement    = "move" dancer_spec "along" shape
            | "move" dancer_spec "to"    position
            | "move" dancer_spec "by"    coords [ "every" number "beats" ] ;

dancer_spec = "all" [ "except" dancer_list ] | dancer_list ;
dancer_list = ident | "[" [ ident { "," ident } ] "]" ;
dancer      = ident ;

position    = ident "[" uint "/" uint "]" | coords ;
coords      = "(" number "," number ")" ;

shape         = ("line" | "horseshoe") shape_body | ident ";" ;
shape_literal = ("line" | "horseshoe") shape_body ;
shape_body    = "{" { shape_prop } "}" | { shape_prop } ";" ;
shape_prop    = ident ( coords | number ) ;

beat        = uint ":" uint ;
number      = [ "-" ] ( uint | float ) ;‑‑
color       = "RED" | "ORANGE" | "YELLOW" | "GREEN" | "DARKGREEN"
            | "BLUE" | "DARKBLUE" | "PURPLE" | "PINK" | "WHITE" | "GREY" ;
ident       = alpha { alpha } ;
string      = '"' { any-char-except-'"' } '"' ;
```
