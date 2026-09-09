# Seed M1: Reader + Printer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `seed` Rust crate that reads Lisp source text into an in-memory value and prints it back, such that `(a (b . c) "hi" 42)` piped in comes back structurally identical.

**Architecture:** A plain Rust `enum Value` with heap objects (cons cells, strings) addressed by `u32` index into a bump-allocated `Vec<Obj>` — no pointer tagging, no `unsafe`, identical on every CPU architecture. Symbols are interned to `SymId(u32)` and never freed. A recursive-descent `Reader` turns `&str` into `Value`; a `printer` turns `Value` into canonical text. Nothing is ever deallocated (spec §6: "It leaks. It is a REPL."); a real allocator/GC replaces the bump arena at milestone M6.

**Tech Stack:** Rust (edition 2021), `std`, Cargo workspace, `cargo test` with inline `#[cfg(test)]` unit tests plus one `include_str!`-based golden-file integration test. Build/run verbs via `just`. No third-party crates.

**Spec:** `docs/superpowers/specs/2026-09-09-lispos-design.md` — this plan implements spine milestone **M1** only (section 8, Part I). The `Value`/arena design in Task 1–2 is the crux the spec calls out; M2 (`eval` + primitives) is planned separately once this runs.

## Global Constraints

Copied from the spec; every task's requirements implicitly include these.

- **Language:** Rust, edition 2021. No third-party crate dependencies in `seed/`.
- **Architecture-neutral:** `seed/` assumes nothing about the CPU. Use explicit-width integer types (`i64` for fixnums, `u32` for ids/indices) — never `usize` or `isize` in the `Value` representation or anything serialized. Code must compile and pass tests the same on x86-64 and AArch64 (spec §5).
- **Thin-waist rule (spec §5):** no `std::fs`, `std::net`, `libc::`, or `nix::` anywhere outside `substrate/`. `substrate/` does not exist yet in M1; `std::io` (stdin) is permitted. The golden-file test uses `include_str!` (compile-time), not `std::fs`, specifically to stay clean under this rule. When the CI grep for this rule is added (milestone M5 of Part I), it must exclude `**/tests/**` and `**/build.rs`.
- **Seed stays small (spec §6):** the whole seed targets ~900 lines. M1 is a fraction of that. Do not add features beyond this plan's scope (no `#…` reader syntax, no bignums, no floats, no `eval`).
- **TDD (spec §2, value 6):** no implementation code before a failing test. Triangulate every generalization with a second case before trusting it.
- **Fixnums are `i64`.** No other numeric types in M1.
- **Symbols are case-sensitive.** No case folding (decision for this plan: modern taste, simpler; spec is "CL-shaped, non-conformant").
- **`nil` prints as `nil`; `()` reads as `nil`** (spec §7.1 nil-punning).
- **Commit after every task** with a Conventional Commits message (spec §2, value 6: "frequent commits").

---

## File Structure

| File | Responsibility |
|---|---|
| `Cargo.toml` | workspace root; members = `["seed"]` |
| `seed/Cargo.toml` | the `seed` package: one lib (`seed`) + one bin (`seed`) |
| `seed/src/lib.rs` | module declarations + `read_print(&str) -> String`, the M1 deliverable surface used by both `main.rs` and tests |
| `seed/src/value.rs` | `Value`, `SymId`, `ObjRef` — the representation, behind constructors/accessors so a later tagged-pointer swap stays local to this file + `heap.rs` |
| `seed/src/heap.rs` | `Heap`: bump arena of `Obj` (cons/string) + symbol interner; all allocation and dereferencing |
| `seed/src/reader.rs` | `Reader`, `ReadError`, `read_all` — source text → `Value` |
| `seed/src/printer.rs` | `print_value` — `Value` → canonical source text |
| `seed/src/main.rs` | tiny binary: read all of stdin, `read_print`, write stdout |
| `seed/tests/roundtrip.rs` | golden-file integration test: `include_str!` fixtures, round-trip + idempotency assertions |
| `seed/tests/golden/*.in`, `*.expected` | golden fixtures (also serve as feature triangulation) |
| `justfile` | `test`, `run`, `rp` verbs |

---

## Task 1: Workspace, `Value` + `Heap`, atoms round-trip

**Files:**
- Create: `Cargo.toml`
- Create: `seed/Cargo.toml`
- Create: `seed/src/value.rs`
- Create: `seed/src/heap.rs`
- Create: `seed/src/reader.rs`
- Create: `seed/src/printer.rs`
- Create: `seed/src/lib.rs`
- Test: inline `#[cfg(test)]` module in `seed/src/lib.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces:
  - `value::SymId(pub u32)`, `value::ObjRef(pub u32)` — both `#[derive(Copy, Clone, PartialEq, Eq, Debug)]`; `SymId` also `Hash`.
  - `value::Value` enum: `Nil`, `Fixnum(i64)`, `Sym(SymId)`, `Cons(ObjRef)`, `Str(ObjRef)` — `#[derive(Copy, Clone, PartialEq, Eq, Debug)]`.
  - `Value::nil() -> Value`, `Value::fixnum(i64) -> Value`, `Value::sym(SymId) -> Value` (all `const`); `Value::is_nil(self) -> bool`, `Value::as_fixnum(self) -> Option<i64>`, `Value::as_sym(self) -> Option<SymId>`.
  - `heap::Heap` with `Heap::new() -> Heap`, `intern(&mut self, &str) -> SymId`, `sym_name(&self, SymId) -> &str`, `cons(&mut self, Value, Value) -> Value`, `get_cons(&self, Value) -> Option<(Value, Value)>`, `string(&mut self, String) -> Value`, `get_str(&self, Value) -> Option<&str>`.
  - `reader::ReadError { pub msg: String, pub pos: usize }` implementing `Display` as `read error: {msg} at byte {pos}`; `reader::Reader<'a>` with `Reader::new(&'a str) -> Reader<'a>` and `read_one(&mut self, &mut Heap) -> Result<Option<Value>, ReadError>` (`Ok(None)` = end of input); `reader::read_all(&str, &mut Heap) -> Result<Vec<Value>, ReadError>`.
  - `printer::print_value(Value, &Heap) -> String`.
  - `seed::read_print(&str) -> String` — reads every datum, prints each on its own line joined by `\n`; on `ReadError`, returns the formatted error string.

- [ ] **Step 1: Write the failing test**

Create `seed/src/lib.rs` with only the test module and `mod`/`pub fn` stubs that won't compile yet is *not* allowed — instead create the full module skeleton with `todo!()` bodies so it compiles, then the test fails at runtime. Put this at the bottom of `seed/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::read_print;

    #[test] fn fixnum() { assert_eq!(read_print("42"), "42"); }
    #[test] fn negative_fixnum() { assert_eq!(read_print("-7"), "-7"); }
    #[test] fn plus_prefixed_fixnum() { assert_eq!(read_print("+5"), "5"); }
    #[test] fn nil_atom() { assert_eq!(read_print("nil"), "nil"); }
    #[test] fn symbol() { assert_eq!(read_print("foo"), "foo"); }
    #[test] fn lone_dash_is_symbol() { assert_eq!(read_print("-"), "-"); }
    #[test] fn one_plus_is_symbol() { assert_eq!(read_print("1+"), "1+"); }
    #[test] fn string_atom() { assert_eq!(read_print("\"hi\""), "\"hi\""); }
    #[test] fn string_escapes() {
        assert_eq!(read_print(r#""a\"b\\c\nd""#), r#""a\"b\\c\nd""#);
    }
    #[test] fn multiple_atoms_one_per_line() { assert_eq!(read_print("1 2 3"), "1\n2\n3"); }
    #[test] fn empty_input() { assert_eq!(read_print(""), ""); }
}
```

- [ ] **Step 2: Create the workspace manifests**

`Cargo.toml`:

```toml
[workspace]
members = ["seed"]
resolver = "2"
```

`seed/Cargo.toml`:

```toml
[package]
name = "seed"
version = "0.1.0"
edition = "2021"

[lib]
name = "seed"
path = "src/lib.rs"

[[bin]]
name = "seed"
path = "src/main.rs"
```

- [ ] **Step 3: Write `seed/src/value.rs`**

```rust
//! The value representation for the seed.
//!
//! DELIBERATELY EXPEDIENT (spec §6, design value 4): a plain enum, with heap
//! objects addressed by `u32` index into `Heap`. No pointer tagging, no
//! `unsafe`, identical on every architecture. All construction and inspection
//! goes through the functions here and in `heap.rs`, so swapping in a
//! tagged-pointer representation later stays local to these two files.

/// Interned symbol id. Symbols live forever (never collected) — correct for Lisp.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SymId(pub u32);

/// Index of a heap object in `Heap`'s object arena.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ObjRef(pub u32);

/// An immediate value, or a reference to a heap object.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Nil,
    Fixnum(i64),
    Sym(SymId),
    Cons(ObjRef),
    Str(ObjRef),
}

impl Value {
    pub const fn nil() -> Value { Value::Nil }
    pub const fn fixnum(n: i64) -> Value { Value::Fixnum(n) }
    pub const fn sym(id: SymId) -> Value { Value::Sym(id) }

    pub fn is_nil(self) -> bool { matches!(self, Value::Nil) }

    pub fn as_fixnum(self) -> Option<i64> {
        match self { Value::Fixnum(n) => Some(n), _ => None }
    }

    pub fn as_sym(self) -> Option<SymId> {
        match self { Value::Sym(s) => Some(s), _ => None }
    }
}
```

- [ ] **Step 4: Write `seed/src/heap.rs`**

```rust
//! Storage for the seed: a bump arena of heap objects plus a symbol interner.
//!
//! "Bump" = `Vec::push`. Nothing is ever freed (spec §6: "It leaks. It is a
//! REPL."). A real allocator/GC replaces this at milestone M6.

use std::collections::HashMap;

use crate::value::{ObjRef, SymId, Value};

enum Obj {
    Cons(Value, Value),
    Str(String),
}

pub struct Heap {
    objs: Vec<Obj>,
    sym_names: Vec<String>,
    sym_ids: HashMap<String, SymId>,
}

impl Heap {
    pub fn new() -> Heap {
        Heap { objs: Vec::new(), sym_names: Vec::new(), sym_ids: HashMap::new() }
    }

    // --- symbols -------------------------------------------------------------

    pub fn intern(&mut self, name: &str) -> SymId {
        if let Some(&id) = self.sym_ids.get(name) {
            return id;
        }
        let id = SymId(self.sym_names.len() as u32);
        self.sym_names.push(name.to_string());
        self.sym_ids.insert(name.to_string(), id);
        id
    }

    pub fn sym_name(&self, id: SymId) -> &str {
        &self.sym_names[id.0 as usize]
    }

    // --- cons cells --------------------------------------------------------

    pub fn cons(&mut self, car: Value, cdr: Value) -> Value {
        let r = ObjRef(self.objs.len() as u32);
        self.objs.push(Obj::Cons(car, cdr));
        Value::Cons(r)
    }

    pub fn get_cons(&self, v: Value) -> Option<(Value, Value)> {
        match v {
            Value::Cons(ObjRef(i)) => match &self.objs[i as usize] {
                Obj::Cons(a, d) => Some((*a, *d)),
                Obj::Str(_) => None,
            },
            _ => None,
        }
    }

    // --- strings ----------------------------------------------------------

    pub fn string(&mut self, s: String) -> Value {
        let r = ObjRef(self.objs.len() as u32);
        self.objs.push(Obj::Str(s));
        Value::Str(r)
    }

    pub fn get_str(&self, v: Value) -> Option<&str> {
        match v {
            Value::Str(ObjRef(i)) => match &self.objs[i as usize] {
                Obj::Str(s) => Some(s),
                Obj::Cons(..) => None,
            },
            _ => None,
        }
    }
}

impl Default for Heap {
    fn default() -> Self { Heap::new() }
}
```

- [ ] **Step 5: Write `seed/src/reader.rs` (atom subset)**

Lists come in Task 2 — for now `(` is an error path so the crate compiles and atom tests pass.

```rust
//! The reader: source text -> `Value`.
//!
//! M1 scope: whitespace, `;` line comments, fixnums (`i64`), strings with
//! `\" \\ \n` escapes, symbols (case-sensitive), lists, dotted pairs, `'`
//! quote. No `#…` reader macros.

use crate::heap::Heap;
use crate::value::Value;

#[derive(Debug, PartialEq, Eq)]
pub struct ReadError {
    pub msg: String,
    pub pos: usize,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "read error: {} at byte {}", self.msg, self.pos)
    }
}

pub struct Reader<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Reader<'a> {
        Reader { input, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ReadError> {
        Err(ReadError { msg: msg.into(), pos: self.pos })
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => { self.bump(); }
                Some(';') => {
                    while let Some(c) = self.bump() {
                        if c == '\n' { break; }
                    }
                }
                _ => break,
            }
        }
    }

    /// Read one datum. `Ok(None)` means end of input.
    pub fn read_one(&mut self, heap: &mut Heap) -> Result<Option<Value>, ReadError> {
        self.skip_trivia();
        let c = match self.peek() {
            None => return Ok(None),
            Some(c) => c,
        };
        match c {
            '(' => self.err("lists not implemented yet"),
            ')' => self.err("unexpected `)`"),
            '"' => self.read_string(heap).map(Some),
            '\'' => self.err("quote not implemented yet"),
            _ => self.read_atom(heap).map(Some),
        }
    }

    fn read_string(&mut self, heap: &mut Heap) -> Result<Value, ReadError> {
        let start = self.pos;
        self.bump(); // opening quote
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(ReadError { msg: "unterminated string".into(), pos: start }),
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some(other) => return self.err(format!("unknown string escape `\\{other}`")),
                    None => return Err(ReadError { msg: "unterminated string".into(), pos: start }),
                },
                Some(c) => out.push(c),
            }
        }
        Ok(heap.string(out))
    }

    fn read_atom(&mut self, heap: &mut Heap) -> Result<Value, ReadError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_whitespace() || matches!(c, '(' | ')' | '"' | ';' | '\'') {
                break;
            }
            self.bump();
        }
        let tok = &self.input[start..self.pos];
        debug_assert!(!tok.is_empty());
        if tok == "nil" {
            return Ok(Value::Nil);
        }
        if let Some(n) = parse_fixnum(tok) {
            return Ok(Value::Fixnum(n));
        }
        Ok(Value::Sym(heap.intern(tok)))
    }
}

/// A token is a fixnum iff it is `[+-]?[0-9]+` and fits in `i64`.
/// Anything else (`-`, `1+`, `3.5`, `foo`) is a symbol.
fn parse_fixnum(tok: &str) -> Option<i64> {
    let body = tok.strip_prefix('+').unwrap_or(tok);
    let digits = body.strip_prefix('-').unwrap_or(body);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    body.parse::<i64>().ok()
}

pub fn read_all(input: &str, heap: &mut Heap) -> Result<Vec<Value>, ReadError> {
    let mut r = Reader::new(input);
    let mut out = Vec::new();
    while let Some(v) = r.read_one(heap)? {
        out.push(v);
    }
    Ok(out)
}
```

- [ ] **Step 6: Write `seed/src/printer.rs`**

```rust
//! The printer: `Value` -> canonical source text. Re-reading this output must
//! reproduce the same structure (`tests/roundtrip.rs`).

use crate::heap::Heap;
use crate::value::Value;

pub fn print_value(v: Value, heap: &Heap) -> String {
    let mut out = String::new();
    write_value(&mut out, v, heap);
    out
}

fn write_value(out: &mut String, v: Value, heap: &Heap) {
    match v {
        Value::Nil => out.push_str("nil"),
        Value::Fixnum(n) => out.push_str(&n.to_string()),
        Value::Sym(id) => out.push_str(heap.sym_name(id)),
        Value::Str(_) => write_string(out, heap.get_str(v).expect("Str obj")),
        Value::Cons(_) => write_list(out, v, heap),
    }
}

fn write_list(out: &mut String, start: Value, heap: &Heap) {
    out.push('(');
    let mut v = start;
    let mut first = true;
    loop {
        let (car, cdr) = heap.get_cons(v).expect("Cons obj");
        if !first { out.push(' '); }
        first = false;
        write_value(out, car, heap);
        match cdr {
            Value::Nil => break,
            Value::Cons(_) => v = cdr,
            other => {
                out.push_str(" . ");
                write_value(out, other, heap);
                break;
            }
        }
    }
    out.push(')');
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
}
```

- [ ] **Step 7: Write `seed/src/lib.rs` (glue above the test module)**

```rust
pub mod heap;
pub mod printer;
pub mod reader;
pub mod value;

use heap::Heap;

/// Read every datum in `input` and print each on its own line. On a read error,
/// return the formatted error string. The M1 deliverable surface — used by
/// `main.rs` and the tests.
pub fn read_print(input: &str) -> String {
    let mut heap = Heap::new();
    match reader::read_all(input, &mut heap) {
        Ok(vals) => vals
            .iter()
            .map(|v| printer::print_value(*v, &heap))
            .collect::<Vec<_>>()
            .join("\n"),
        Err(e) => e.to_string(),
    }
}
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p seed`
Expected: all 11 tests in `tests` PASS. (`cargo test` also compiles `main.rs` — Task 1 does not create it yet, so add a placeholder now: see Step 9.)

- [ ] **Step 9: Add a minimal `seed/src/main.rs` so the bin target compiles**

```rust
use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");
    let output = seed::read_print(&input);
    if !output.is_empty() {
        println!("{output}");
    }
}
```

Run: `cargo test -p seed` again.
Expected: PASS, and `cargo build -p seed` succeeds.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml seed/
git commit -m "feat(seed): value representation, heap arena, atom reader/printer"
```

---

## Task 2: Lists

**Files:**
- Modify: `seed/src/reader.rs` (replace the `'(' => ...` error arm; add `read_list`)
- Modify: `seed/src/lib.rs` (add tests to the `tests` module)
- Test: inline `#[cfg(test)]` module in `seed/src/lib.rs`

**Interfaces:**
- Consumes: everything Task 1 produced.
- Produces: `Reader::read_list` (private); no new public signatures. Behaviour added: `(` … `)` reads a proper list; `()` reads as `Value::Nil`; lists nest.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `seed/src/lib.rs`:

```rust
#[test] fn empty_list_is_nil() { assert_eq!(read_print("()"), "nil"); }
#[test] fn flat_list() { assert_eq!(read_print("(a b c)"), "(a b c)"); }
#[test] fn nested_list() { assert_eq!(read_print("(a (b) c)"), "(a (b) c)"); }
#[test] fn deeply_nested() {
    assert_eq!(read_print("((1 2) (3 (4)))"), "((1 2) (3 (4)))");
}
#[test] fn list_with_nil_element() { assert_eq!(read_print("(a nil b)"), "(a nil b)"); }
#[test] fn list_spanning_lines() { assert_eq!(read_print("(a\n b\n c)"), "(a b c)"); }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p seed`
Expected: the six new tests FAIL (`read_print("(a b c)")` returns the string `read error: lists not implemented yet at byte 0`).

- [ ] **Step 3: Implement `read_list` and wire it in**

In `seed/src/reader.rs`, change the `'('` arm of `read_one`:

```rust
            '(' => { self.bump(); self.read_list(heap).map(Some) }
```

Add the method to the `impl<'a> Reader<'a>` block:

```rust
    fn read_list(&mut self, heap: &mut Heap) -> Result<Value, ReadError> {
        let mut items: Vec<Value> = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return self.err("end of input inside list"),
                Some(')') => { self.bump(); break; }
                Some(_) => match self.read_one(heap)? {
                    Some(v) => items.push(v),
                    None => return self.err("end of input inside list"),
                },
            }
        }
        let mut chain = Value::Nil;
        for v in items.into_iter().rev() {
            chain = heap.cons(v, chain);
        }
        Ok(chain)
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p seed`
Expected: all tests PASS (Task 1's 11 + Task 2's 6).

- [ ] **Step 5: Commit**

```bash
git add seed/src/reader.rs seed/src/lib.rs
git commit -m "feat(seed): read and print proper lists"
```

---

## Task 3: Dotted pairs and quote

**Files:**
- Modify: `seed/src/reader.rs` (`read_list` gains a `.` branch; `read_one` gains the `'` branch; add `dot_is_delimited`)
- Modify: `seed/src/lib.rs` (add tests)
- Test: inline `#[cfg(test)]` module in `seed/src/lib.rs`

**Interfaces:**
- Consumes: everything Task 1–2 produced.
- Produces: no new public signatures. Behaviour added: `(a . b)` / `(a b . c)` read as improper lists and print with ` . `; `(a . nil)` and `(1 . (2 . nil))` normalise to proper-list printing; `'x` reads as `(quote x)`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `seed/src/lib.rs`:

```rust
#[test] fn simple_dotted() { assert_eq!(read_print("(a . b)"), "(a . b)"); }
#[test] fn dotted_tail() { assert_eq!(read_print("(a b . c)"), "(a b . c)"); }
#[test] fn dotted_nil_normalises() { assert_eq!(read_print("(a . nil)"), "(a)"); }
#[test] fn nested_cons_normalises() {
    assert_eq!(read_print("(1 . (2 . nil))"), "(1 2)");
}
#[test] fn dot_with_no_head_errors() {
    assert!(read_print("( . x)").starts_with("read error:"));
}
#[test] fn foo_dot_bar_is_one_symbol() { assert_eq!(read_print("foo.bar"), "foo.bar"); }
#[test] fn quote_shorthand() { assert_eq!(read_print("'x"), "(quote x)"); }
#[test] fn quote_list() { assert_eq!(read_print("'(a b)"), "(quote (a b))"); }
#[test] fn quote_is_structural() { assert_eq!(read_print("(quote x)"), "(quote x)"); }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p seed`

Expected — genuinely FAIL (these drive the implementation):
- `dotted_nil_normalises` — `(a . nil)` prints `(a . nil)`, want `(a)`.
- `nested_cons_normalises` — `(1 . (2 . nil))` prints `(1 . (2 . nil))`, want `(1 2)`.
- `dot_with_no_head_errors` — `( . x)` currently reads as the list `(. x)` (symbol named `.`), no error.
- `quote_shorthand`, `quote_list` — FAIL with `read error: quote not implemented yet at byte 0`.

Expected — may already PASS accidentally, and that is fine:
- `simple_dotted`, `dotted_tail` — with `.` read as an ordinary symbol, `(a . b)` is the 3-element list `(a |.| b)`, which prints `(a . b)` — identical to the real dotted pair. The normalisation and error tests above are what force the correct parse.
- `foo_dot_bar_is_one_symbol`, `quote_is_structural` — already pass from Tasks 1–2.

> Note for the implementer: before writing code, run the failing tests and read what `read_print` actually returns for each. That output is your starting point; make the minimal change to reach the asserted string.

- [ ] **Step 3: Implement the `'` branch in `read_one`**

In `seed/src/reader.rs`, replace the `'\''` arm of `read_one`:

```rust
            '\'' => {
                self.bump();
                let quoted = match self.read_one(heap)? {
                    Some(v) => v,
                    None => return self.err("end of input after `'`"),
                };
                let q = heap.intern("quote");
                let tail = heap.cons(quoted, Value::Nil);
                Ok(Some(heap.cons(Value::Sym(q), tail)))
            }
```

- [ ] **Step 4: Implement the dotted-pair branch in `read_list`**

Replace the whole `read_list` method with:

```rust
    fn read_list(&mut self, heap: &mut Heap) -> Result<Value, ReadError> {
        let mut items: Vec<Value> = Vec::new();
        let mut tail = Value::Nil;
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return self.err("end of input inside list"),
                Some(')') => { self.bump(); break; }
                Some('.') if self.dot_is_delimited() => {
                    if items.is_empty() {
                        return self.err("nothing before `.`");
                    }
                    self.bump(); // consume '.'
                    match self.read_one(heap)? {
                        Some(v) => tail = v,
                        None => return self.err("end of input after `.`"),
                    }
                    self.skip_trivia();
                    match self.peek() {
                        Some(')') => { self.bump(); break; }
                        None => return self.err("end of input after dotted tail"),
                        Some(_) => return self.err("more than one datum after `.`"),
                    }
                }
                Some(_) => match self.read_one(heap)? {
                    Some(v) => items.push(v),
                    None => return self.err("end of input inside list"),
                },
            }
        }
        let mut chain = tail;
        for v in items.into_iter().rev() {
            chain = heap.cons(v, chain);
        }
        Ok(chain)
    }

    /// True when the next char is `.` and the char after it ends a token
    /// (so `.` is the dotted-pair marker, not part of a symbol like `foo.bar`).
    fn dot_is_delimited(&self) -> bool {
        let mut chars = self.input[self.pos..].chars();
        if chars.next() != Some('.') {
            return false;
        }
        match chars.next() {
            None => true,
            Some(c) => c.is_whitespace() || matches!(c, '(' | ')' | ';'),
        }
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p seed`
Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add seed/src/reader.rs seed/src/lib.rs
git commit -m "feat(seed): dotted pairs and quote shorthand"
```

---

## Task 4: Comments and reader errors

**Files:**
- Modify: `seed/src/lib.rs` (add tests)
- Modify: `seed/src/reader.rs` only if a test reveals a gap (comment handling already exists from Task 1's `skip_trivia`; this task is mostly locking in behaviour with tests)
- Test: inline `#[cfg(test)]` module in `seed/src/lib.rs`

**Interfaces:**
- Consumes: everything Task 1–3 produced.
- Produces: no new signatures. Guarantees: `;` comments are ignored to end of line, including inside lists; every malformed input listed below yields a string starting `read error:` rather than a panic.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `seed/src/lib.rs`:

```rust
#[test] fn leading_comment() { assert_eq!(read_print("; hi\n42"), "42"); }
#[test] fn trailing_comment() { assert_eq!(read_print("42 ; hi"), "42"); }
#[test] fn comment_inside_list() { assert_eq!(read_print("(a ; x\n b)"), "(a b)"); }
#[test] fn comment_only_input() { assert_eq!(read_print("; nothing here"), ""); }

#[test] fn unterminated_string_errors() {
    assert!(read_print("\"abc").starts_with("read error:"));
}
#[test] fn unexpected_close_paren_errors() {
    assert!(read_print(")").starts_with("read error:"));
}
#[test] fn eof_in_list_errors() {
    assert!(read_print("(a b").starts_with("read error:"));
}
#[test] fn junk_after_dotted_tail_errors() {
    assert!(read_print("(a . b c)").starts_with("read error:"));
}
#[test] fn missing_dotted_tail_errors() {
    assert!(read_print("(a . )").starts_with("read error:"));
}
#[test] fn unknown_string_escape_errors() {
    assert!(read_print(r#""a\qb""#).starts_with("read error:"));
}
#[test] fn error_message_has_byte_position() {
    let msg = read_print(")");
    assert!(msg.contains("at byte "), "got: {msg}");
}
```

- [ ] **Step 2: Run tests to verify they fail (or pass)**

Run: `cargo test -p seed`
Expected: the comment tests likely PASS already (Task 1's `skip_trivia` handles `;`); the error tests should also mostly PASS. **Any that fail** point to a real gap — fix it in Step 3. If all pass, this task is pure test coverage: skip to Step 4. Record which tests failed, if any, in the commit message.

- [ ] **Step 3: Fix any gap a failing test reveals**

Only if a test failed. Likely candidates and their fixes:
- `comment_only_input` returning something other than `""` → `read_all` already returns `Ok(vec![])` for no data, so `read_print` returns `""`; if not, check `skip_trivia` consumes a trailing comment with no newline (the `while let Some(c) = self.bump()` loop ends at EOF — correct).
- A panic instead of a `read error:` string → find the `unwrap`/indexing that panicked and convert it to an `err(...)` return. Do not add error handling anywhere a test does not exercise.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p seed`
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add seed/src/lib.rs seed/src/reader.rs
git commit -m "test(seed): lock in comment handling and reader error paths"
```

---

## Task 5: Binary, golden-file harness, and the M1 grin-test

**Files:**
- Create: `seed/tests/roundtrip.rs`
- Create: `seed/tests/golden/atoms.in`, `seed/tests/golden/atoms.expected`
- Create: `seed/tests/golden/list.in`, `seed/tests/golden/list.expected`
- Create: `seed/tests/golden/dotted.in`, `seed/tests/golden/dotted.expected`
- Create: `seed/tests/golden/quote.in`, `seed/tests/golden/quote.expected`
- Create: `seed/tests/golden/comment.in`, `seed/tests/golden/comment.expected`
- Create: `seed/tests/golden/grin.in`, `seed/tests/golden/grin.expected`
- Create: `justfile`
- Test: `seed/tests/roundtrip.rs`

**Interfaces:**
- Consumes: `seed::read_print(&str) -> String`.
- Produces: nothing consumed by later milestones; this task packages M1 for use and regression.

- [ ] **Step 1: Write the golden fixture files**

`seed/tests/golden/atoms.in`:

```
42 -7 nil foo "hi"
```

`seed/tests/golden/atoms.expected`:

```
42
-7
nil
foo
"hi"
```

`seed/tests/golden/list.in`:

```
(a b c)
()
(a (b) c)
```

`seed/tests/golden/list.expected`:

```
(a b c)
nil
(a (b) c)
```

`seed/tests/golden/dotted.in`:

```
(a . b)
(a b . c)
(1 . (2 . nil))
```

`seed/tests/golden/dotted.expected`:

```
(a . b)
(a b . c)
(1 2)
```

`seed/tests/golden/quote.in`:

```
'x
'(a b)
(quote z)
```

`seed/tests/golden/quote.expected`:

```
(quote x)
(quote (a b))
(quote z)
```

`seed/tests/golden/comment.in`:

```
; leading comment
42 ; trailing
(a ; mid-list
 b)
```

`seed/tests/golden/comment.expected`:

```
42
(a b)
```

`seed/tests/golden/grin.in`:

```
(a (b . c) "hi" 42)
```

`seed/tests/golden/grin.expected`:

```
(a (b . c) "hi" 42)
```

- [ ] **Step 2: Write the failing test harness**

`seed/tests/roundtrip.rs`:

```rust
//! Golden-file tests. Fixtures are compiled in with `include_str!` (no runtime
//! file IO, so this stays clean under the thin-waist no-`std::fs` rule).

macro_rules! golden {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("golden/", $name, ".in")),
            include_str!(concat!("golden/", $name, ".expected")),
        )
    };
}

const GOLDENS: &[(&str, &str, &str)] = &[
    golden!("atoms"),
    golden!("list"),
    golden!("dotted"),
    golden!("quote"),
    golden!("comment"),
    golden!("grin"),
];

#[test]
fn golden_files_roundtrip() {
    for (name, input, expected) in GOLDENS {
        let got = seed::read_print(input);
        assert_eq!(got, expected.trim_end(), "golden `{name}` mismatch");
    }
}

#[test]
fn printing_is_idempotent() {
    for (name, input, _) in GOLDENS {
        let once = seed::read_print(input);
        let twice = seed::read_print(&once);
        assert_eq!(once, twice, "golden `{name}` printed form is not stable");
    }
}
```

- [ ] **Step 3: Run the harness to verify it passes**

Run: `cargo test -p seed --test roundtrip`
Expected: both tests PASS. If `grin` fails, the failure message shows `read_print("(a (b . c) \"hi\" 42)")` vs the expected — fix whichever of Tasks 1–4 the mismatch traces to, then re-run.

- [ ] **Step 4: Write the `justfile`**

`justfile` at the repo root:

```just
# run the full test suite
test:
    cargo test

# run the seed binary, reading a program on stdin
run:
    cargo run -q -p seed

# read-print a single string argument: `just rp '(a (b . c) 42)'`
rp arg:
    printf '%s' '{{arg}}' | cargo run -q -p seed
```

- [ ] **Step 5: Verify the binary by hand**

Run: `just rp '(a (b . c) "hi" 42)'`
Expected stdout: `(a (b . c) "hi" 42)`

Run: `printf '%s' '1 2 3' | cargo run -q -p seed`
Expected stdout:
```
1
2
3
```

- [ ] **Step 6: Run the whole suite once more**

Run: `cargo test`
Expected: every test across `lib` and `tests/roundtrip.rs` PASSES.

- [ ] **Step 7: Commit**

```bash
git add seed/tests/ justfile
git commit -m "test(seed): golden-file round-trip harness and M1 grin-test"
```

---

## Self-Review

**1. Spec coverage.** M1's scope is spec §8 row 1 ("Seed: reader + printer") plus the §6 pieces it necessarily pulls in:
- Reader — symbols, fixnums, lists, strings, dotted pairs, `'` quote, `;` comments, no `#…` → Tasks 1–4. ✅
- Printer — nil/fixnum/sym/str/cons canonical forms → Task 1 (atoms), Task 2 (lists), Task 3 (dotted). ✅
- Value repr + tagging — `value.rs`, plain enum + `u32` refs, architecture-neutral, encapsulated for a later tagged-pointer swap → Task 1. ✅
- Allocator — bump `Vec`, no GC, "it leaks" → `heap.rs`, Task 1. ✅
- Golden-file tests for reader/printer round-trips (spec §9.4) → Task 5. ✅
- Grin-test `(a (b . c) "hi" 42)` round-trips (spec §8) → Task 5, `grin` fixture. ✅
- `just` verbs (spec §9.3) → Task 5. ✅
- Not in M1, correctly absent: `eval`, primitives, macros, `boot.lisp`, the thin-waist module, GC, packages (all later milestones). ✅

**2. Placeholder scan.** No "TBD"/"handle edge cases"/"add validation"/"write tests for the above" — every test and every implementation body is spelled out. Task 4 Step 3 is conditional ("only if a test failed") but names the concrete candidates and fixes rather than gesturing. ✅

**3. Type consistency.** `Value` / `SymId` / `ObjRef` / `Heap` / `Reader` / `ReadError` / `read_all` / `read_print` / `print_value` are used with identical signatures everywhere they appear. Allocator methods are `Heap::cons` / `get_cons` / `string` / `get_str` / `intern` / `sym_name` throughout (not, e.g., `alloc_cons` in one place and `cons` in another). `read_one` returns `Result<Option<Value>, ReadError>` consistently. The error string prefix `read error:` in `ReadError::Display` matches every `.starts_with("read error:")` assertion. ✅

**Reconciliation noted:** spec §6's reader row originally said "no dotted-pair syntax yet"; the §8 M1 grin-test requires `(b . c)`. The spec has been updated to include dotted-pair read/print in M1; this plan follows the grin-test.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-09-seed-m1-reader-printer.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
