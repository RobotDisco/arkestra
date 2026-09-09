# Seed M1: Reader + Printer Implementation Plan

> **You are writing this yourself.** This plan gives you the module layout, the exact type definitions and public signatures (so M2 stays compatible), every test verbatim, and hints on the parts that are easy to get stuck on. The function *bodies* are yours to write. Work it test-first: write the test, watch it fail, make it pass, commit. Ping the mentor when a hint isn't enough or a design question surfaces.

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

**On private helpers:** the plan names helpers like `read_list`, `skip_trivia`, `write_list`, `parse_fixnum`. Those names are suggestions — they're private, so rename freely. The **public** items in each task's *Interfaces → Produces* block are a contract M2 depends on; keep those signatures exact.

---

## Task 1: Workspace, `Value` + `Heap`, atoms round-trip

**Files:**
- Create: `Cargo.toml`, `seed/Cargo.toml`
- Create: `seed/src/value.rs`, `seed/src/heap.rs`, `seed/src/reader.rs`, `seed/src/printer.rs`, `seed/src/lib.rs`, `seed/src/main.rs`
- Test: inline `#[cfg(test)]` module in `seed/src/lib.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces — keep these signatures exact:
  - `value::SymId(pub u32)`, `value::ObjRef(pub u32)` — both `#[derive(Copy, Clone, PartialEq, Eq, Debug)]`; `SymId` also `Hash`.
  - `value::Value` enum: `Nil`, `Fixnum(i64)`, `Sym(SymId)`, `Cons(ObjRef)`, `Str(ObjRef)` — `#[derive(Copy, Clone, PartialEq, Eq, Debug)]`.
  - `Value::nil() -> Value`, `Value::fixnum(i64) -> Value`, `Value::sym(SymId) -> Value` (all `const`); `Value::is_nil(self) -> bool`, `Value::as_fixnum(self) -> Option<i64>`, `Value::as_sym(self) -> Option<SymId>`.
  - `heap::Heap` with `Heap::new() -> Heap`, `intern(&mut self, &str) -> SymId`, `sym_name(&self, SymId) -> &str`, `cons(&mut self, Value, Value) -> Value`, `get_cons(&self, Value) -> Option<(Value, Value)>`, `string(&mut self, String) -> Value`, `get_str(&self, Value) -> Option<&str>`.
  - `reader::ReadError { pub msg: String, pub pos: usize }` implementing `Display` as `read error: {msg} at byte {pos}`; `reader::Reader<'a>` with `Reader::new(&'a str) -> Reader<'a>` and `read_one(&mut self, &mut Heap) -> Result<Option<Value>, ReadError>` (`Ok(None)` = end of input); `reader::read_all(&str, &mut Heap) -> Result<Vec<Value>, ReadError>`.
  - `printer::print_value(Value, &Heap) -> String`.
  - `seed::read_print(&str) -> String` — reads every datum, prints each on its own line joined by `\n`; on `ReadError`, returns the formatted error string.

- [ ] **Step 1: Write the failing test**

Create `seed/src/lib.rs`. Put the module declarations and a `read_print` stub with a `todo!()` body at the top so the crate compiles, then this test module at the bottom:

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

- [ ] **Step 3: Write `seed/src/value.rs` — the type definitions**

These definitions are the design decision from brainstorming — write them as given. The doc comment matters; it's the reminder of *why* this is a plain enum.

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
```

Then an `impl Value` block with these six methods — **you write the bodies**:

```
pub const fn nil() -> Value
pub const fn fixnum(n: i64) -> Value
pub const fn sym(id: SymId) -> Value
pub fn is_nil(self) -> bool
pub fn as_fixnum(self) -> Option<i64>
pub fn as_sym(self) -> Option<SymId>
```

*Hints:* all one-liners. The constructors just wrap (`Value::Fixnum(n)` etc.); `const fn` changes nothing about the body. `is_nil` — `matches!(self, Value::Nil)`. `as_fixnum` / `as_sym` — a `match` returning `Some(..)` for the one variant, `None` otherwise.

- [ ] **Step 4: Write `seed/src/heap.rs`**

Write the data definitions as given (this layout is the design):

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
```

Then `impl Heap` (and `impl Default for Heap`) with these — **you write the bodies**:

```
pub fn new() -> Heap
pub fn intern(&mut self, name: &str) -> SymId
pub fn sym_name(&self, id: SymId) -> &str
pub fn cons(&mut self, car: Value, cdr: Value) -> Value
pub fn get_cons(&self, v: Value) -> Option<(Value, Value)>
pub fn string(&mut self, s: String) -> Value
pub fn get_str(&self, v: Value) -> Option<&str>
```

*Hints:*
- `intern`: look in `sym_ids` first (`if let Some(&id) = ...`). On a miss, the new id is `SymId(self.sym_names.len() as u32)`; push the name into `sym_names` and insert into `sym_ids`. You'll `name.to_string()` twice — fine for the seed.
- `sym_name`: index `sym_names` by `id.0 as usize`.
- `cons` / `string`: the new `ObjRef` is `ObjRef(self.objs.len() as u32)`; `push` the `Obj`; return `Value::Cons(r)` / `Value::Str(r)`.
- `get_cons` / `get_str` (`&self`, no `&mut`): match `v` to pull out the `ObjRef`; index `self.objs`; match the `Obj` variant. Any mismatch (wrong `Value` variant, or `Cons` ref pointing at a `Str`) → `None`.

- [ ] **Step 5: Write `seed/src/reader.rs` — atom subset**

Write the error type and the reader struct as given (the `Display` string is part of the contract — tests assert on `"read error:"` and `"at byte "`):

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
```

Then implement — **bodies are yours**:

```
impl<'a> Reader<'a>:
    pub fn new(input: &'a str) -> Reader<'a>
    fn peek(&self) -> Option<char>
    fn bump(&mut self) -> Option<char>
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ReadError>
    fn skip_trivia(&mut self)
    pub fn read_one(&mut self, heap: &mut Heap) -> Result<Option<Value>, ReadError>
    fn read_string(&mut self, heap: &mut Heap) -> Result<Value, ReadError>
    fn read_atom(&mut self, heap: &mut Heap) -> Result<Value, ReadError>

free functions:
    fn parse_fixnum(tok: &str) -> Option<i64>
    pub fn read_all(input: &str, heap: &mut Heap) -> Result<Vec<Value>, ReadError>
```

*Hints:*
- `peek`: `self.input[self.pos..].chars().next()`. Do **not** index the string by byte — this is the only UTF-8 care you need.
- `bump`: `peek`, then `self.pos += c.len_utf8()`.
- `err`: build a `ReadError` with `pos: self.pos`; return `Err(..)`. Generic in `T` so you can `return self.err(...)` from any function.
- `skip_trivia`: loop. Whitespace (`char::is_whitespace`) → `bump`. `;` → `bump` until you consume a `\n` or hit EOF. Anything else → `break`.
- `read_one`: `skip_trivia`; `peek`; `None` → `Ok(None)`. Dispatch on the first char: `"` → `read_string`; `)` → error "unexpected \`)\`"; **for Task 1 only**, `(` and `'` → an error like "lists not implemented yet" / "quote not implemented yet" so the crate compiles and atom tests run; everything else → `read_atom`.
- `read_string`: save the opening `pos` for the "unterminated string" error. Consume the opening `"`. Accumulate `char`s into a `String`. On `\\`, read the next char and map `" \ n` to `" \ \n`; any other escape char → error; EOF mid-string → "unterminated string" at the saved pos.
- `read_atom`: consume until a delimiter — `char::is_whitespace` or one of `( ) " ; '`. Slice `&self.input[start..self.pos]`. Classify the token: exactly `"nil"` → `Value::Nil`; else `parse_fixnum` → `Value::Fixnum`; else `heap.intern(tok)` → `Value::Sym`.
- `parse_fixnum`: the rule that keeps `-` and `1+` as symbols. Strip one leading `+` (if any), then note a leading `-`; the remaining characters must be non-empty and all ASCII digits before you trust `str::parse::<i64>()`. Return `None` otherwise. (`+5` parsing to `5` and printing as `5` is acceptable — same value, surface not preserved.)
- `read_all`: loop `read_one`, pushing each `Some(v)` into a `Vec`, stop at `Ok(None)`, propagate `Err` with `?`.

- [ ] **Step 6: Write `seed/src/printer.rs`**

```rust
//! The printer: `Value` -> canonical source text. Re-reading this output must
//! reproduce the same structure (`tests/roundtrip.rs`).

use crate::heap::Heap;
use crate::value::Value;
```

Implement — **bodies are yours**:

```
pub fn print_value(v: Value, heap: &Heap) -> String
fn write_value(out: &mut String, v: Value, heap: &Heap)
fn write_list(out: &mut String, start: Value, heap: &Heap)
fn write_string(out: &mut String, s: &str)
```

*Hints:*
- `print_value`: make a `String`, hand `&mut` to `write_value`, return it.
- `write_value`: `match v` — `Nil` → `"nil"`; `Fixnum(n)` → `n.to_string()`; `Sym(id)` → `heap.sym_name(id)`; `Str(_)` → `write_string` with `heap.get_str(v).unwrap()`; `Cons(_)` → `write_list`.
- `write_list` is the subtle one. Push `(`. Walk: `get_cons` the current value; print the car (space-separate — track a `first` flag); then look at the cdr — `Nil` → stop; `Cons` → make it the current value and loop; **anything else** → push `" . "`, print that value, stop. Push `)`.
- `write_string`: inverse of the reader's escape set — `"` → `\"`, `\` → `\\`, `\n` → `\n`, everything else verbatim, wrapped in `"`.

- [ ] **Step 7: Finish `seed/src/lib.rs`**

Module declarations (write as given):

```rust
pub mod heap;
pub mod printer;
pub mod reader;
pub mod value;
```

Then `pub fn read_print(input: &str) -> String` — **body is yours**. *Hint:* make a `Heap`, call `reader::read_all`. On `Ok(vals)`, map each through `printer::print_value` and `join("\n")`. On `Err(e)`, return `e.to_string()` (that's the `Display` impl).

- [ ] **Step 8: Write `seed/src/main.rs`**

Boilerplate — write as given:

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

- [ ] **Step 9: Run the tests to verify they pass**

Run: `cargo test -p seed`
Expected: all 11 tests in `tests` PASS; `cargo build -p seed` succeeds.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml seed/
git commit -m "feat(seed): value representation, heap arena, atom reader/printer"
```

---

## Task 2: Lists

**Files:**
- Modify: `seed/src/reader.rs` (replace the `'('` error arm; add `read_list`)
- Modify: `seed/src/lib.rs` (add tests)

**Interfaces:**
- Consumes: everything Task 1 produced.
- Produces: `Reader::read_list` (private). Behaviour added: `(` … `)` reads a proper list; `()` reads as `Value::Nil`; lists nest.

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
Expected: all six FAIL — `read_print("(a b c)")` returns `read error: lists not implemented yet at byte 0`.

- [ ] **Step 3: Implement `read_list`**

Change the `(` arm of `read_one` to consume the paren and call `self.read_list(heap).map(Some)`. Add `fn read_list(&mut self, heap: &mut Heap) -> Result<Value, ReadError>`.

*Hints:*
- Loop: `skip_trivia`; `peek`. `None` → error "end of input inside list". `)` → `bump` and stop. Otherwise `read_one` and push the value into a `Vec<Value>` (a `None` from `read_one` here also means EOF → error).
- Build the chain **right-to-left**: `let mut chain = Value::Nil; for v in items.into_iter().rev() { chain = heap.cons(v, chain); }`. Building left-to-right would mean mutating the previous cell's cdr, which is awkward with index handles — don't.
- `()` needs no special case: empty `items` leaves `chain == Value::Nil`.

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

**Interfaces:**
- Consumes: everything Task 1–2 produced.
- Produces: no new public signatures. Behaviour: `(a . b)` / `(a b . c)` read as improper lists and print with ` . `; `(a . nil)` and `(1 . (2 . nil))` normalise to proper-list printing; `'x` reads as `(quote x)`.

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

Genuinely FAIL (these drive the implementation):
- `dotted_nil_normalises` — `(a . nil)` prints `(a . nil)`, want `(a)`.
- `nested_cons_normalises` — `(1 . (2 . nil))` prints `(1 . (2 . nil))`, want `(1 2)`.
- `dot_with_no_head_errors` — `( . x)` currently reads as the list `(. x)` (a symbol named `.`), no error.
- `quote_shorthand`, `quote_list` — FAIL with `read error: quote not implemented yet at byte 0`.

May already PASS accidentally, and that's fine:
- `simple_dotted`, `dotted_tail` — with `.` read as an ordinary symbol, `(a . b)` is the 3-element list `(a |.| b)`, which *prints* `(a . b)` — identical to the real dotted pair. The normalisation and error tests above are what force the correct parse.
- `foo_dot_bar_is_one_symbol`, `quote_is_structural` — already pass from Tasks 1–2.

> Before writing code, run the failing tests and read what `read_print` actually returns for each. That output is your starting point; make the minimal change to reach the asserted string.

- [ ] **Step 3: Implement the `'` branch in `read_one`**

*Hint:* consume the `'`, `read_one` for the next datum (`None` → error "end of input after \`'\`"), then build `(quote <datum>)` — `heap.cons(Value::Sym(heap.intern("quote")), heap.cons(datum, Value::Nil))`. Watch the borrow order: intern before you start nesting `cons` calls.

- [ ] **Step 4: Implement the dotted-pair branch in `read_list` + `dot_is_delimited`**

*Hints:*
- In the `read_list` loop, before the general case, match `Some('.')` **guarded by** `self.dot_is_delimited()`.
  - If `items.is_empty()` → error "nothing before \`.\`".
  - Consume the `.`; `read_one` for the tail value; `skip_trivia`; then require `)` — `Some(')')` stop, `None` → "end of input after dotted tail", `Some(_)` → "more than one datum after \`.\`".
- `fn dot_is_delimited(&self) -> bool`: look at the next two chars **without consuming** (`self.input[self.pos..].chars()`). First must be `.`. Second must be `None` (EOF) or whitespace or one of `( ) ;`. This is what stops `foo.bar` and `...` from being misread — inside `read_atom`, `.` is an ordinary constituent; only this lookahead in list context promotes it to a marker.
- The fold changes by one word: start `chain = tail` instead of `chain = Value::Nil`, then fold `items` in reverse as before.
- **No printer change.** `write_list` from Task 1 already prints improper tails. Sanity-check *why* `dotted_nil_normalises` now passes: `(a . nil)` folds to `cons(a, Nil)`, and `write_list` sees a `Nil` cdr and stops → `(a)`.

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
- Modify: `seed/src/reader.rs` only if a test reveals a gap (comment handling already exists from Task 1's `skip_trivia`; this task mostly locks behaviour in with tests)

**Interfaces:**
- Consumes: everything Task 1–3 produced.
- Produces: no new signatures. Guarantees: `;` comments ignored to end of line, including inside lists; every malformed input below yields a string starting `read error:` rather than a panic.

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
Expected: comment tests likely PASS already (Task 1's `skip_trivia` handles `;`); most error tests likely PASS too. **Any that fail** point to a real gap — fix it in Step 3. If all pass, this task is pure coverage: skip to Step 4. Note in the commit message which (if any) failed.

- [ ] **Step 3: Fix any gap a failing test reveals**

Only if a test failed. Likely candidates:
- `comment_only_input` not returning `""` → check `skip_trivia` consumes a trailing comment with no newline (the `bump`-until-`\n`-or-EOF loop should already end cleanly at EOF).
- A **panic** instead of a `read error:` string → find the `unwrap`/index that panicked and convert it to a `return self.err(...)`. Do not add error handling anywhere a test doesn't exercise (spec §2: no speculative error handling).

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
- Create: `seed/tests/golden/{atoms,list,dotted,quote,comment,grin}.{in,expected}`
- Create: `justfile`

**Interfaces:**
- Consumes: `seed::read_print(&str) -> String`.
- Produces: nothing consumed by later milestones; this packages M1 for use and regression.

- [ ] **Step 1: Write the golden fixture files**

`atoms.in`:
```
42 -7 nil foo "hi"
```
`atoms.expected`:
```
42
-7
nil
foo
"hi"
```
`list.in`:
```
(a b c)
()
(a (b) c)
```
`list.expected`:
```
(a b c)
nil
(a (b) c)
```
`dotted.in`:
```
(a . b)
(a b . c)
(1 . (2 . nil))
```
`dotted.expected`:
```
(a . b)
(a b . c)
(1 2)
```
`quote.in`:
```
'x
'(a b)
(quote z)
```
`quote.expected`:
```
(quote x)
(quote (a b))
(quote z)
```
`comment.in`:
```
; leading comment
42 ; trailing
(a ; mid-list
 b)
```
`comment.expected`:
```
42
(a b)
```
`grin.in`:
```
(a (b . c) "hi" 42)
```
`grin.expected`:
```
(a (b . c) "hi" 42)
```

- [ ] **Step 2: Write the failing test harness**

`seed/tests/roundtrip.rs` — test scaffolding, write as given:

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
Expected: both tests PASS. If `grin` fails, the message shows `read_print("(a (b . c) \"hi\" 42)")` vs expected — trace the mismatch to whichever of Tasks 1–4 owns it, fix, re-run.

- [ ] **Step 4: Write the `justfile`**

`justfile` at the repo root — write as given:

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

Run: `just rp '(a (b . c) "hi" 42)'` → expect stdout `(a (b . c) "hi" 42)`
Run: `printf '%s' '1 2 3' | cargo run -q -p seed` → expect three lines `1` / `2` / `3`

- [ ] **Step 6: Run the whole suite**

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
- Value repr — `value.rs`, plain enum + `u32` refs, architecture-neutral, encapsulated for a later tagged-pointer swap → Task 1. ✅
- Allocator — bump `Vec`, no GC, "it leaks" → `heap.rs`, Task 1. ✅
- Golden-file tests (spec §9.4) → Task 5. ✅
- Grin-test `(a (b . c) "hi" 42)` round-trips (spec §8) → Task 5, `grin` fixture. ✅
- `just` verbs (spec §9.3) → Task 5. ✅
- Correctly absent from M1: `eval`, primitives, macros, `boot.lisp`, thin-waist module, GC, packages. ✅

**2. Placeholder scan.** No "TBD"/"handle edge cases"/"add validation". Every test is verbatim; every function has a signature and a hint. Task 4 Step 3 is conditional but names concrete candidates. ✅

**3. Type consistency.** `Value` / `SymId` / `ObjRef` / `Heap` / `Reader` / `ReadError` / `read_all` / `read_print` / `print_value` appear with identical signatures throughout. Allocator methods are `cons` / `get_cons` / `string` / `get_str` / `intern` / `sym_name` everywhere. `read_one` is `Result<Option<Value>, ReadError>` throughout. The `read error:` prefix in `ReadError::Display` matches every `.starts_with("read error:")` assertion. ✅

**Reconciliation:** spec §6's reader row originally said "no dotted-pair syntax yet"; the §8 M1 grin-test requires `(b . c)`. The spec has been updated to include dotted-pair read/print in M1; this plan follows the grin-test.

---

## How to work this plan

- One task at a time, in order. Within a task: write the test(s) → `cargo test -p seed` and watch them fail for the *right* reason → write bodies → green → commit.
- If a hint leaves you stuck for more than ~15 minutes — borrow checker, a lifetime on `Reader<'a>`, `&self` vs `&mut self` on the heap accessors — that's a good moment to ask the mentor rather than grind.
- If while implementing you find yourself wanting to change a **public** signature from the *Interfaces → Produces* blocks, stop and raise it — M2's plan is written against those.
- Private helper names and structure are yours. So is going further than a task asks (a nicer error type, a `Display` for `Value`) — just note it's off-plan so the spec/plan stay in sync.
- After M1 is green end-to-end, come back and we plan M2 (`eval`, environments, the ~15 primitives, the REPL proper) with the real `Value` type in hand.
