# LispOS — Design

- **Date:** 2026-09-09
- **Status:** Approved design; ready for implementation planning (Part I)
- **Working name:** **Arkestra** (running default, non-binding — kept cheap to rename while the tree is small; package prefix `arkestra`). See *Naming & mindspace*.

---

## 1. Motivation & north star

A hand-built, Lisp-based operating system as a lifelong tinker playground. Explicitly a hobby/learning project, run as a planned spine of discrete milestones so it survives ADHD-style context loss — but with unlimited unplanned tinkering encouraged *off* that spine.

The payoff being chased is a blend of two things:

- **A ("it's alive"):** the REPL *is* the machine. Boot an emulator, get a Lisp prompt instead of a shell. Redefine a function that is part of the running system and watch the change take effect live. No wall between you and the system.
- **C ("I built the ladder"):** every rung underneath the running Lisp is yours — the reader, the evaluator, the GC, the way it talks to the screen. The self-made tower is the point.

Stated another way: **the best of Smalltalk and Emacs, if you ignore their C cores** — a persistent, self-editing, self-compiling world you author from the inside.

Two lower-priority flavours to grow toward later, never load-bearing:

- **B ("the world persists"):** image-based living environment; the whole world survives reboots; you return mid-thought.
- **D ("it's a place"):** presentation-based REPL where everything printed is a live, clickable object; the display is a graph of real objects.

---

## 2. Design values

These govern judgement calls throughout the project.

1. **The spine is the through-line.** At any moment you can name the next un-done spine milestone. If you cannot, re-read the spine before writing code.
2. **Tinkering hangs off spine milestones; it does not replace them.** Spending a week making the GC a generational compacting collector with a visualiser is great — that hangs off the GC milestone. Just do not start it *instead of* the GC milestone.
3. **Stepping off the spine is allowed and should be *noticed*.** "I am off-spine building X" is a fine thing to say to yourself. Silent drift for a month is the failure mode.
4. **Prefer the interesting path over the expedient one.** Where two implementations both work, take the one that teaches more or is stranger — *unless* it drags you into C-level bit-twiddling with no Lisp payoff. The spine milestones are the floor, not the ceiling.
5. **The POSIX build must never break.** It is the fast development loop and the behavioural oracle for the metal build. A change that only works on metal is not done.
6. **One milestone at a time.** TDD for the seed (Rust): no implementation code before a failing test. A failing `tests/*.lisp` assertion before any Lisp-layer feature. Earn generalization — a second case before trusting a general-looking implementation.
7. **No enforced grand theme.** Meaning (naming, mythology, aesthetics) accretes from use; it is not designed up front or back-filled for tidiness.

---

## 3. Trajectory: Stage C → Stage B → Stage A

The project moves through three substrate *stages* without rewriting the tower. It does this by binding everything above the substrate to a deliberately tiny interface (the *thin waist*, section 5). ("Stage" here is distinct from the numbered *milestones* of the spine in section 8.)

- **Stage C — POSIX process.** The "OS" runs as a normal Linux application. A Lisp environment with its own world, image save/load, its own shell and editor, that *pretends* it is the machine. Fastest path to a living system.
- **Stage B — freestanding on emulated metal.** The same tower runs on QEMU with no Linux underneath. A small Rust substrate (`no_std`) gets to flat memory, puts bytes on screen, delivers keyboard bytes and a timer interrupt. Everything a Lisp programmer would call "the system" is Lisp and is yours.
- **Stage A — the substrate earns more of the stack.** Optional, open-ended, pursued as far as motivation lasts: your own keyboard/disk drivers replacing BIOS/PIO shims, your own page tables, an AHCI driver, SMP. The waist the Lisp sees barely moves; it is renegotiated consciously, not forced.

The jump from Stage C to Stage B is "write a second implementation of a ten-function interface and fix what breaks," not a fresh start.

---

## 4. Architecture: the layer cake

```
┌─────────────────────────────────────────────────────────┐
│  lisp/system/  — written in your Lisp, loaded from src  │
│  editor · shell · inspector · scheduler · conditions ·  │
│  package system · the Lisp-side compiler · (later) GUI  │
├─────────────────────────────────────────────────────────┤
│  lisp/boot.lisp — written in your Lisp, loaded at start │
│  cond · let* · when · defstruct · destructuring · the   │
│  "real" reader · macro-expansion machinery             │
├─────────────────────────────────────────────────────────┤
│  seed/  — Rust, ~900 lines, disposable-but-kept         │
│  reader · tagged values · bump alloc · tree-walk eval   │
│  with macros · ~15 primitives · printer · REPL loop     │
├─────────────────────────────────────────────────────────┤
│  ═══════════════ THE THIN WAIST ═══════════════════════  │
│  10 ops. Nothing above this line may reach past it.     │
├─────────────────────────────────────────────────────────┤
│  substrate/ — Rust                                       │
│   posix/ (Stage C) → metal/ (Stage B) →                 │
│   metal/ grows toward Stage A · rpi/ (Part III stretch) │
└─────────────────────────────────────────────────────────┘
```

**Layer responsibilities:**

| Layer | Owns | Language | Lifecycle |
|---|---|---|---|
| `substrate/` | the metal boundary: memory, console, clock, timer, block I/O, idle, shutdown | Rust (`posix/` is `std`; `metal/` and `rpi/` are `no_std`) | one impl per target regime |
| `seed/` | cold-start: reader, value representation, bump allocation (later GC), tree-walking `eval` with macros, ~15 primitives, printer, REPL loop | Rust, target-agnostic | disposable-but-kept; never owes quality; exists to load `boot.lisp` |
| `lisp/boot.lisp` | the pleasant core language: `cond`, `let*`, `when`, `and`/`or`, `defstruct`, destructuring, the real reader, macro machinery | your Lisp | grows continuously |
| `lisp/system/` | everything recognisable as "the OS": packages, condition system, scheduler, editor, inspector, shell, the Lisp-side compiler, later a GUI | your Lisp | grows continuously; the real project |

**Endgame division of labour:** even when everything portable has moved into Lisp, Rust permanently owns exactly two things — **memory** (the GC and the value representation) and **the metal boundary** (the thin waist). Everything a Lisp programmer is proud of — the reader you actually use, macros, conditions, the object system, the optimizing compiler — is written in your Lisp, by you. This is the Mezzano split.

---

## 5. The thin waist

The entire contract between the tower and the metal. `substrate/waist.rs` defines it as a Rust trait. `posix/`, `metal/`, and later `rpi/` are implementations; the tower binds to whichever is linked.

| Op | Signature (sketch) | POSIX impl | Metal impl |
|---|---|---|---|
| memory | `mem_region() -> (*mut u8, usize)` | one large `mmap` | bootloader-reported free region minus reserved |
| console out | `console_write(&[u8])` | `write(1, …)` | poke UART + VGA text buffer |
| console in | `console_read() -> Option<Event>` | raw-mode `read(0, …)` | keyboard IRQ ring buffer |
| monotonic clock | `mono_ns() -> u64` | `clock_gettime(MONOTONIC)` | TSC or PIT/HPET |
| timer tick | `on_tick(hz, fn)` | `setitimer` + signal handler | program the timer IRQ |
| block read | `block_read(n, &mut [u8])` | `pread` on a backing file | ATA/AHCI PIO |
| block write | `block_write(n, &[u8])` | `pwrite` on a backing file | ATA/AHCI PIO |
| idle | `idle()` | `pause()` / `poll` | `hlt` |
| shutdown | `shutdown(code) -> !` | `exit(code)` | ACPI poweroff / QEMU `isa-debug-exit` |
| framebuffer *(deferred, for flavour D)* | `framebuffer() -> Option<Fb>` | SDL window or Linux fbdev | VBE/GOP linear framebuffer |

**Ten operations.** `block_read`/`block_write` are listed as a pair; the last (framebuffer) is deferred until flavour D is pursued, leaving nine implemented in Stage C. The count is deliberately small and roughly fixed — new capability is expected to arrive as Lisp on top of these, not as waist growth (until Stage A consciously renegotiates).

**The one enforcement rule:** no `std::fs`, `std::net`, `libc::`, or `nix::` usage outside `substrate/`. Enforced by a CI grep and a code-review habit. This single rule is the entire portability story — it is what makes Stage C → B → A a series of weekend milestones instead of a rewrite.

**Architecture neutrality:** the tower assumes nothing about the CPU architecture. Pointer tagging and alignment tricks in `seed/value.rs` must hold on AArch64 (Raspberry Pi) as well as x86-64.

---

## 6. The seed — cut line

The seed's entire job is: **load `boot.lisp` and get out of the way.** Anything a programmer would call "the language" is Lisp source read at startup.

**In the seed (first cut, ~900 lines Rust):**

| Piece | Rough size | First-cut scope |
|---|---|---|
| Reader | ~150 | symbols, fixnums, lists, strings, dotted pairs, `'` quote. No `#…` reader macros (the M1 grin-test needs dotted-pair read/print; `#…` syntax and reader macros are what's deferred) |
| Value repr + tagging | ~100 | tagged pointer: fixnum, cons, symbol, string, primitive-fn, closure. Must hold on x86-64 and AArch64 |
| Allocator | ~40 | bump-allocate, **no GC**. It leaks. It is a REPL. GC arrives at Milestone 6 |
| `eval` / `apply` | ~250 | ~10 special forms: `quote if fn def set! do let quasiquote` + **macro support** |
| Primitives | ~150 | arithmetic; `cons car cdr eq? null? cons? symbol?`; `print read apply eval`; the waist calls |
| Printer | ~80 | enough to see results |
| Thin-waist POSIX impl | ~100 | stdin/stdout bytes, monotonic clock, `block_*` against a backing file |
| REPL loop + `main` | ~50 | read, eval, print, loop; load `boot.lisp` first |

**Deliberately kept in the seed despite the minimalism:** macro support (~30 lines: a macro is a function in a "macro" table that `eval` calls at expansion time). Without it, `boot.lisp` must be written in a subset too primitive to bootstrap `cond`/`when`/`and` out of — the tool that builds those *is* macros.

**Explicitly NOT in the seed:** GC (Milestone 6), bytecode/compilation (Milestone 12, and in Lisp), bignums/ratios/floats, hygienic macros, packages (Milestone 7, in Lisp), the condition system (Milestone 9, in Lisp), objects.

The seed is disposable-by-design but **never actually deleted** — it remains the cold-load path forever, the way a compiler's bootstrap stage sticks around. It never has to become fast or clever.

---

## 7. Language shape

CL-*shaped*, with **zero ANSI conformance** — "the modern Common Lisp that never shipped." Take CL's semantic model; discard the portability cruft (`#+`/`#-` feature conditionals, the reader-macro zoo, the breadth of the numeric tower, platform variety).

### 7.1 Decided forks (baked into the seed and the later compiler)

| Fork | Decision | Rationale |
|---|---|---|
| Bootstrap | Rust seed interpreter, disposable-but-kept; everything else grows in Lisp | tight feedback loop the whole way; real self-hosting where it counts (the compiler is yours, in Lisp) |
| Flavour | CL-shaped, non-conformant | matches existing muscle memory; conditions/packages/`defmacro` are exactly what a living OS wants |
| Namespaces | **Lisp-2** (separate function/value namespaces; `funcall`, `#'foo`) | matches muscle memory; makes unhygienic `defmacro` capture bite far less often |
| `nil` | **`nil`-punning** — `nil` is false, `()`, and a symbol | terse early list code; known quantity. (Most defensible override: Scheme's `#f`/`()` separation catches bugs) |
| Continuations | **escape-only** — `block`/`return-from`/`catch`/`throw` + `unwind-protect`. No `call/cc` | full re-entrant continuations are a large permanent implementation tax; an OS has real threads and a condition system instead |
| Full `call/cc` | **deferred to a conscious checkpoint at Lisp-side-compiler design time** (Milestone 12) — not a silent assumption | cheap to choose a continuation-friendly codegen strategy (CPS / reifiable frames / stack-copying / segmented stack) *at* compiler design; expensive to retrofit onto a conventional compiler afterwards. Delimited continuations (`shift`/`reset`) are a lower-tax middle option if the toys (generators, coroutines, effect handlers) are wanted without full `call/cc` |
| Tail calls | proper TCO is a goal, delivered by the Lisp-side compiler; the disposable seed may trampoline or punt | cheap in the compiler; an OS writes servers/loops as recursive processes |
| Macros | `defmacro`, unhygienic, in the seed | ~30 lines; hygiene is a substantial subsystem and a Part III layer |
| Packages | early — Milestone 7, in Lisp | an OS needs namespaces; retrofitting them is miserable |
| Numbers | fixnums only in the seed | bignums/ratios/floats are Part III toys |
| Objects | deferred — `defstruct` in `boot.lisp`; single-dispatch generic functions when wanted (small in-spine add); full CLOS/MOP a Part III toy | |

### 7.2 Genuinely emergent (decide when you get there)

Macro hygiene (additive layer on top of `defmacro`), the condition system's exact shape (a *library* over dynamic binding + escape continuations), CLOS vs simple generic functions vs records, `car`/`cdr` vs `first`/`rest` naming, `format`, iteration constructs, reader-macro set, the numeric tower's breadth.

---

## 8. The spine

Ordered "**this now works**" milestones, each with a concrete grin-test. Size labels are vibes, not estimates. Part I gets its own implementation plan first; Part II gets a plan later; Part III is open-ended forever.

### Part I — a living Lisp on POSIX (Stage C)

| # | Milestone | Grin-test | Size |
|---|---|---|---|
| 1 | Seed: reader + printer | pipe `(a (b . c) "hi" 42)` in, get it echoed back structurally identical | small |
| 2 | Seed: `eval` + ~15 primitives + REPL | `(def (double x) (* x 2))` then `(double 21)` → `42` at a live prompt | medium |
| 3 | Seed: macros | `(defmacro unless (c . body) ...)` works; `unless` behaves | small |
| 4 | `boot.lisp` loads at startup | `cond`, `when`, `let*`, `and`, `or` defined *in Lisp*, used in the REPL | small |
| 5 | thin waist as a Rust module | REPL still works; all I/O now routes through `substrate/posix/` | small |
| 6 | mark-sweep GC | allocate in an infinite loop, RSS stays flat; `(room)` reports honestly | medium |
| 7 | packages | `(in-package :foo)`; symbol collisions across packages resolve | medium |
| 8 | image save/load via `block_*` | define stuff, `(save-world "w.img")`, restart, `(load-world "w.img")` — all still there | medium |
| 9 | condition system (in Lisp, on dynamic binding + `catch`/`throw`) | an error drops you into a REPL *in the failing context*; fix a binding, `(continue)` | medium |
| 10 | cooperative scheduler + `on_tick` preemption | two processes printing; one runs `(loop)` forever; the other keeps running | medium |
| 11 | in-world editor (structure- or line-based — choice deferred to build time) | edit a running function's source *inside the system*, save, see the change live — no external editor | large |
| 12 | self-hosting checkpoint: the Lisp-side compiler | a `boot.lisp` function, compiled by *your* compiler to bytecode/closures, runs faster than the tree-walker; the seed still only interprets the cold load. **`call/cc` checkpoint: decide the continuation strategy here.** | large |

**End of Part I:** a persistent, self-editing, self-compiling Lisp world that recovers from its own errors — running in an xterm.

Editor (#11) is intentionally sequenced *before* the compiler (#12): the visible shiny payoff first, the self-hosting payoff second.

### Part II — it's really the machine (Stage B)

| # | Milestone | Grin-test | Size |
|---|---|---|---|
| 13 | `metal/`: boot to a serial `"hello"` | QEMU, no Linux; bootloader stub → Rust `_start` prints over serial | medium |
| 14 | `metal/` waist: `mem_region` + `console_*` over serial | the *same* seed REPL, now on the metal build, over the serial port | medium |
| 15 | timer IRQ → `on_tick`; keyboard IRQ → `console_read` | the scheduler from #10 runs on real interrupts; you type at the metal REPL | medium |
| 16 | `block_*` on a real disk (ATA/AHCI PIO) | `save-world`/`load-world` from #8, now to a QEMU virtual disk | medium |
| 17 | the whole Part I world boots on metal | cold-load `boot.lisp` + `system/` on bare QEMU; land in the in-world editor | large |

**End of Part II:** an OS. No Unix underneath. Same tower.

### Part III — open-ended (toward Stage A / flavour B / flavour D, any order, forever)

Framebuffer + `framebuffer()` waist op → a graphical presentation REPL (flavour **D**) · your own keyboard/disk drivers replacing BIOS/PIO shims (toward **A**) · own page tables · AHCI · SMP · a network stack · hygienic macros as a layer · delimited continuations / full `call/cc` · CLOS / the MOP · a real filesystem vs. just the image · persistence as a first-class object store (flavour **B**, deepened) · **Raspberry Pi / AArch64 hardware bring-up** (`substrate/rpi/` — mini-UART, mailbox framebuffer, no BIOS/ATA; the thin waist proving itself a third time) · hand-rolled bootloader replacing Limine/`bootloader` crate.

---

## 9. Development environment & feedback loop

**Governing principle:** develop on the POSIX build ~95% of the time. Metal is a periodic "does it still boot" checkpoint, not where you iterate. Same tower, two substrates — so the POSIX build is also the *oracle*: any behaviour that differs between POSIX and metal is a substrate bug you can bisect.

### 9.1 Inner loop (Milestone C and forever after)

```
edit lisp/boot.lisp or lisp/system/*.lisp  →  cargo run  →  REPL in your terminal  →  repeat   (seconds)
```

Until the in-world editor (#11), "reload" = restart the POSIX binary. After #8 (image save) you restart *into your saved world*. After #11 you edit inside the running world and stop restarting.

### 9.2 Metal loop (Milestone B onward)

```
just run-metal    →  QEMU headless, serial on stdio  →  REPL over serial
just debug-metal  →  QEMU frozen, gdb attached on :1234
```

**Serial console is the primary metal I/O, not VGA.** Scriptable, logged, greppable, pipeable into tests. VGA text output is a nice-to-have added later; the framebuffer GUI is Part III.

### 9.3 Tooling choices

| Concern | Choice | Notes |
|---|---|---|
| Emulator | **QEMU** primary; Bochs when something is *deeply* wrong | QEMU: scriptable, gdb stub, virtio, `isa-debug-exit`. Bochs: better instruction-level introspection for rare gnarly bugs |
| Metal debugging | `qemu -s -S` + `gdb`/`lldb`, plus copious serial `println` | the gdb stub gives real breakpoints in Rust on bare metal |
| Bootloader | **kept open** — start with Limine *or* the `bootloader` crate; the other, and eventually a hand-rolled boot sequence, become toward-A toys | using a bootloader deletes ~a week of UEFI/multiboot/paging trivia with no Lisp payoff; hand-rolling is a legitimate rabbit hole if that specific romp appeals |
| `no_std` | only `substrate/metal/` and `substrate/rpi/`; `seed/` and everything above build for both host and target | keeps the thin-waist rule honest |
| Build orchestration | `just` (`run`, `run-metal`, `test`, `debug-metal`, `ci`) | a flat list of named incantations, which is what this is. `make` would be a `.PHONY` task runner (no transform-rule DAG exists: `cargo` owns Rust incremental builds; Lisp compiles at runtime inside the world). A ~30-line `dev.sh` `case` is an acceptable swap if avoiding a tool dependency |

### 9.4 Testing (TDD applies)

| Layer | How | Runs where |
|---|---|---|
| Seed (Rust) | `cargo test` — unit tests per component; **golden-file tests** for reader/printer round-trips | host |
| `boot.lisp` + `system/` (Lisp) | a `tests/*.lisp` suite run by the POSIX binary; assertions in Lisp | POSIX build, in CI |
| "does metal still boot?" | CI builds the image, boots it headless in QEMU, greps serial for a magic `READY` string, exits via `isa-debug-exit` | CI (once Part II exists) |
| thin-waist discipline | CI greps for `std::fs` / `std::net` / `libc::` / `nix::` outside `substrate/` — fails the build | CI |
| POSIX vs metal parity | the *same* `tests/*.lisp` suite run on both builds; divergence = substrate bug | CI (once the metal REPL exists) |

**CI, concretely:** one run (`just ci` locally / one GitHub Actions job): `cargo test` → waist-grep → build POSIX → run `tests/*.lisp` on it → (Part II onward) build metal → boot in QEMU → check for `READY`. Green means the romp is still standing.

---

## 10. Repo layout

```
some_os/
├── justfile                  # run · run-metal · test · debug-metal · ci
├── Cargo.toml                # workspace
├── seed/                     # Rust — the disposable-but-kept cold interpreter
│   ├── src/
│   │   ├── reader.rs
│   │   ├── value.rs          # tagged repr; must hold on x86-64 AND aarch64
│   │   ├── alloc.rs          # bump first; mark-sweep at Milestone 6
│   │   ├── eval.rs           # tree-walk + macro table
│   │   ├── prim.rs           # ~15 primitives
│   │   ├── printer.rs
│   │   └── main.rs           # REPL; loads lisp/boot.lisp
│   └── tests/                # cargo test — golden files for reader/printer
├── substrate/                # Rust — implementations of the 10-op thin waist
│   ├── waist.rs              # the trait. THE contract.
│   ├── posix/                # Milestone C  — mmap, read/write, clock_gettime
│   ├── metal/                # Milestone B  — no_std, x86-64, QEMU
│   └── rpi/                  # Part III     — no_std, aarch64, real hardware
├── lisp/
│   ├── boot.lisp             # cond, let*, when, defstruct, the real reader…
│   └── system/
│       ├── packages.lisp
│       ├── conditions.lisp
│       ├── scheduler.lisp
│       ├── editor.lisp
│       ├── compiler/         # the self-hosting payoff (Milestone 12)
│       └── shell.lisp
├── tests/                    # *.lisp — run by the POSIX build in CI; parity suite for metal
└── docs/
    └── superpowers/specs/    # this design doc
```

`seed/` and `substrate/` are **siblings**: the seed depends on the waist trait, substrates implement it. This keeps "the tower is separable from the metal" visually honest.

---

## 11. Scope boundaries

**Explicitly NOT in the spine** (Part III / someday-toys — written down so that building one is *noticed* as stepping off the spine, not prohibited):

- full `call/cc` / delimited continuations
- hygienic macros
- CLOS / the MOP (single-dispatch generic functions are a small in-spine add; the full protocol is a toy)
- bignums, ratios, floats, the numeric tower
- a real filesystem (the world-image is persistence until you want more)
- networking / a TCP stack
- SMP / multicore
- a graphical framebuffer UI and the presentation-REPL (flavour D)
- a first-class persistent object store (flavour B, deepened)
- your own drivers replacing bootloader/BIOS/PIO shims (toward A)
- Raspberry Pi / AArch64 hardware bring-up
- hand-rolled bootloader (vs. Limine / `bootloader` crate)
- any security model, users, permissions

**How the romp stays anchored:** design values 1–3 (section 2). The spine is the through-line; tinkering hangs off milestones; stepping off is fine if *noticed*.

---

## 12. Naming & mindspace (non-binding, emergent)

**Arkestra** is the running default name — chosen for feel, held loosely. No enforced theme. Meaning accretes from use; it is not designed up front, and not back-filled out of a wish for consistency. If a mythology emerges, keep it. The name stays cheap to change while the tree is small.

**Three directions kept warm** — playful, musical, adjacent to a construction-mindspace:

- **Arkestra** — *ark* (a vessel carrying and preserving the whole world — the image) + *orchestra* (many independent players tuning and improvising together — the processes). Sun Ra cues: Saturn, "Space is the Place," myth-science, joyful discipline, boarding and travelling as an ensemble, "it's after the end of the world" and that's fine.
- **Trancentral** — KLF cues: the last train to a central terminus of the mind, Mu Mu Land, 3 a.m. eternal, "Chill Out" as a sound-journey, stadium euphoria welded to gleeful art-prank sincerity. A hub you travel *to* and a state you are *in*.
- **Mu** — Zen's answer that un-asks the question (the gateless gate); the sunken mythic continent; the KLF's Mu Mu Land; µ as *micro* and as the *least-fixed-point operator* — self-reference, the Y-combinator's home. Smallness as fullness.

**Ideation ritual as a first-class practice.** In the lineage of Eno & Schmidt's *Oblique Strategies*, Cage's I Ching chance operations, Jodorowsky's tarot-as-creativity, Lynch's "ideas are fish": use ritual and randomness as an idea pump without believing the cosmology literally (the same fake-it-earnestly posture as Discordianism). Optional hooks:

- Draw a card (an Oblique Strategy, or a tarot card read generatively, not divinatorily) to *pick the name* when the time comes — let the choosing be part of the artifact's story.
- An `(oblique)` command in the in-world shell that deals a prompt when stuck — a `boot.lisp` one-liner over a card list, or a Part III toy with real tarot semantics.
- Name off-spine tinker-sessions after whatever card you drew when you wandered off. The detours get a folklore.

---

## 13. Next step

Hand this spec to the `writing-plans` skill to produce a step-by-step implementation plan for **Part I** (milestones 1–12). Part II gets its own plan when Part I lands. Part III is never planned as a whole.
