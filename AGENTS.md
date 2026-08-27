# AGENTS.md — GhostDesk

How this project is laid out and named. Read it before adding a file: the
conventions below cannot be inferred from the tree, and drifting from them is
what turns a slice into a folder nobody can navigate.

**The doctrine is the framework's, not this file's.** The reference is the
official architecture documentation — <https://nestrs.dev/architecture/>
(fundamentals, all options, configuration). This file is the project-local
delta: what the tree adds on top, never what it overrides. When this file and
the framework documentation disagree, **the documentation wins**, and the
disagreement is a bug here to fix, not a choice to defend.

## Layout — three crates, three jobs

```
apps/ghostdesk/src/   main.rs, module.rs, and the endpoint's own edge — pure composition
crates/features/      the product's domains, each with its own MCP edge
crates/platform/      the OS substrate — knows nothing about the framework
```

**`crates/platform/` is framework-free by decision.** Five OS-neutral contracts
(`InputBackend`, `ScreenBackend`, `WindowManager`, `Clipboard`, `AppCatalog`);
`host` picks the implementation for the compile target. Nothing outside that
crate names an OS, and nothing inside it names the framework. A port to a third
OS is a new sibling module, never a branch in a service.

**`crates/features/` holds the domains.** Each wraps a platform seam in an
`#[injectable]` service. A domain never talks to the OS directly.

A module is a **port** plus one **adapter per transport**. The port sits at the
module root; each adapter gets a sub-folder with its own `module.rs`, and an app
imports only the edges it serves.

```
crates/features/src/<module>/
  module.rs service.rs config.rs error.rs   the port
  mcp/      module.rs tool.rs
  schedule/ module.rs tasks.rs
```

**The shape is invariant.** One transport, one adapter sub-folder, one
`<Module><Edge>Module`, every time. Never invert it into a single top-level edge
folder that injects every domain service: that trades the module gate — an app
importing exactly the edges it serves — for an adapter no app can subset.
Several modules may join the **same** MCP endpoint: the framework merges them
onto one, so a single client URL is no reason to fold domains together. That is
how `/mcp` is served here — see *How the endpoint is composed*.

## Names — four levels, and none overflows into the next

| Level | Named for | Appears as |
|---|---|---|
| **Project** | the product | the repository and the workspace — **nowhere else** |
| **Crate** | what it holds | `crates/<crate>/`, and the root of every span target it emits |
| **App** | what it **serves** (`api`, `worker`, `auth`) | `apps/<app>/`, the binary, `<App>Module` |
| **Module** | its **domain** (`users`, `billing`) | `<module>/`, `<Module>Module` |

```
<App>Module              apps/<app>/src/module.rs     composition root
<Module>Module           <module>/module.rs            the port
<Module><Edge>Module     <module>/<edge>/module.rs     one adapter
<WhatItBinds>Module      <name>/module.rs              a substrate
```

**No module or provider below the root ever carries the project's or the app's
name.** The project name stops at the workspace; the app name stops at
`<App>Module`. An app may share the project's name only while it is the only
app — and even then, nothing beneath it may.

**A module name is plural when the domain is a collection of enumerable things
(`users`, `orders`), singular when it is a capability (`auth`, `search`).** Not
cosmetic: the generator singularizes the folder name to derive the entity, so a
wrongly pluralized module produces a wrongly named entity, silently.

## Crates — a type, and a direction

Every crate has a type, and the type decides what it may depend on. An arrow
that points back up is a defect, not a trade-off. Cargo enforces most of this
for you: a crate that does not list another as a dependency simply cannot reach
it.

| Crate | Type | May depend on |
|---|---|---|
| `apps/<app>` | composition | everything |
| `crates/features` | feature | the framework, substrates |
| a substrate (`crates/<name>`) | util | third parties only — **never** the framework, never features |
| `crates/migrations`, `crates/seed` | tooling | binaries, outside the graph |

## Modules — two files, two jobs, never merged

| File | Job | Answers to | Holds |
|---|---|---|---|
| `mod.rs` | which **files** exist, and **what leaves the module** | the compiler, and readers | `//!`, `mod`, `pub use` |
| `module.rs` | which **providers** exist, what is imported | the framework | exactly one `#[module]` |

`module.rs` is the DI module. `mod.rs` is both the folder index *and* the
export contract: **its `pub use` list is what the rest of the workspace may
reach.** `pub` means exported; everything else is `pub(crate)` or private. A
`mod.rs` that re-exports everything cancels the encapsulation — that list is a
decision, not plumbing.

**No `*_module.rs`, ever.** One `#[module]` per file, one `module.rs` per
folder; two modules in a feature means two folders.

**The composition root lists imports only.** A provider on the root module is
composition doing a feature's job — the documentation's shape for a root is
"compose features and framework modules; let adapters own the handlers".
App-local glue gets its own `module.rs` beside what it binds and joins the
import list like everything else (see *How the endpoint is composed*, point 4).

## Providers — three questions, in order

`#[module]` takes only `imports` and `providers`. There is no `controllers`
list, so the *mechanism* cannot say what a thing is for. **The name has to.**
Answer these before naming anything.

**Q1 — is it listed in `providers`?** No ⇒ it is not a provider. It is either
something the framework *consumes* without injecting (an entity, a `#[config]`,
a DTO the validator reads) or plain vocabulary (an enum, a type alias, a set of
constants). Both are named by the tables below; neither needs a module of its
own. **Only something injected by type needs a module.**

**Q2 — who calls it?** The framework, *because of what it is* ⇒ **primitive**:
the vocabulary is closed, you pick from it rather than invent. Your own code
⇒ **custom** ⇒ Q3.

**Q3 — does it own domain logic?** Yes ⇒ it is a **`Service`**, and that is the
residue by design. No ⇒ name it for **what it is** — a factory, a client, a
store, a bridge, a registry, a transport seam — and never `Service`.

## Naming tables

File name = role, folder = module. Snake_case, no dotted variants, **one role
→ one file per folder**.

**Mounted or injected primitives.** The framework dispatches to these; the file
is named for the role, never for the type.

| Role | File |
|---|---|
| DI module (exactly one `#[module]` struct per file) | `module.rs` |
| Folder index (`pub use` / `mod` only) | `mod.rs` |
| Service | `service.rs` / `services/` |
| Controller (REST) / Resolver (GraphQL) / Gateway (WS) | `http/controller.rs` / `graphql/resolver.rs` / `ws/gateway.rs` |
| Processor (queue) / Scheduled tasks / Tool (MCP) | `queue/processor.rs` / `schedule/tasks.rs` / `mcp/tool.rs` |
| Event listener host | `events/listener.rs` |
| Entity (ORM + `#[expose]`) | `entity.rs` / `entities/` |
| Guard / Strategy / Pipe | `guard.rs` / `strategy.rs` / `pipe.rs` |
| Module config (`#[config]`) | `config.rs` |
| Domain error / Static constants | `error.rs` / `constants.rs` |

An adapter role carries its folder: `schedule/tasks.rs`, never `tasks.rs` at
the module root. A transport-specific guard belongs to its adapter too
(`mcp/guard.rs`).

**Custom providers.** Injectable, but nothing is dispatched *to* them. Named
for what they are, file named the same, and **never folded into `service.rs`**.
A recognised word beats an invented one: `Factory`, `Client`, `Store`,
`Registry`, `Source`, `Bridge`.

**Vocabulary.** Not registered anywhere: an enum, a struct, a type alias, a set
of constants. Named for *what it declares* — a role suffix on vocabulary is
noise. Shared test doubles are the one crate-root file: `testing.rs`, behind
`#[cfg(test)]`, doubles only.

## Precedence — when a type carries a primitive role *and* logic

A primitive role wins **only when the framework is the sole caller and the file
holds no domain logic**. `tasks.rs` earns its name when the clock is the only
caller and the work it drives lives in a service; otherwise it is a service
that happens to have a trigger. Same test for `#[hooks]`, `#[listeners]` and a
health indicator: a lifecycle hook or a scheduled tick never renames a service.

## Several of the same role

Pluralized sub-folder; the singular trait file stays at the parent.

| Folder | File | Type |
|---|---|---|
| Providers — `services/`, `strategies/`, `pipes/` | bare: `input.rs` | `InputService` |
| Entities — `entities/` | bare: `user.rs` | `User` |
| Transfer objects — `dtos/`, `commands/`, `events/` | suffixed: `login_dto.rs` | `LoginDto` |

A provider's role is spelled by its folder *and* its type, so the file does not
spell it a third time. A transfer object is read far from its folder — in a
handler signature — so it keeps the suffix at both sites.

**Two services in one module is a last resort.** Extracting a factory, a client
or an enum leaves the count at one, and that is the common case. Reach for
`services/` only when the module owns two bodies of domain logic.

## Folders

- A module that is not a feature still gets a folder — cross-cutting wiring
  imported once by the root is `<name>/module.rs`, never a top-level
  `<name>.rs`. A hand-written `impl Module` is still a DI module.
- **A module's sub-folders are a closed set of two kinds**: transport adapters
  and pluralized role folders (`services/`, `entities/`, `dtos/`, …). **There
  is no third kind.** A folder invented to group "things that go together" —
  `contract/`, `types/`, `core/`, `shared/`, `common/` — is a defect: every
  file it would hold is already named by a table above, so it sits flat beside
  its siblings. A folder that feels too full means the module is too big; split
  the module, never the vocabulary.
- **The edge vocabulary is closed**: `http`, `graphql`, `ws`, `queue`,
  `schedule`, `mcp`, `events`. The *form* is open — a new edge follows
  `<edge>/module.rs` + `<Module><Edge>Module` — but adding one is a framework
  change, not a local improvisation.
- `mod.rs` / `lib.rs` carry `//!`, `mod` and `pub use` — no logic.
- Injected service field is `svc` when there is one, `<name>_svc` when there
  are several. Non-service dependencies keep descriptive names (`db`, `queue`,
  `config`).

## Comments

**A comment carries the business, never the framework.** The reader is
assumed to know NestRS: its mechanics are documented at nestrs.dev, and a
comment restating them — what `as dyn` binds, when a scheduled method ticks,
what the access graph checks — is that documentation duplicated: wrong the
day the framework moves, and tiring every day before. The same goes for a
header that restates its own path — `//! The screen domain's MCP adapter.`
above `screen/mcp/` says nothing twice — and for the same sentence repeated
across sibling modules: it teaches nothing the second time.

A comment earns its place when it states what is *ours* and the code cannot
show: a product rule, a security boundary, a deliberate trade-off, the why
behind a shape that would otherwise read as a mistake. One test decides every
case — **if the sentence would be true in any NestRS project, it belongs to
the framework's docs, not to this repo.** The `//!` slot on an index exists
for a header that has something to say; an index whose path already says
everything keeps none.

## Reserved vocabulary

**A module may not take a name from the structural vocabulary.** These words
already mean something to the layout, and reusing one makes every path
ambiguous. Pick the domain word instead — a module about desktop applications
is `programs`, not `apps`.

```
structure   apps  crates  features  src  tests
roles       mod  module  service  controller  resolver  gateway  tool
            processor  tasks  listener  guard  strategy  pipe  config
            entity  error  constants  testing
plurals     services  entities  dtos  commands  events  strategies  pipes
edges       http  graphql  ws  queue  schedule  mcp  events
```

## Transfer objects — named for the boundary they cross

| Kind | Suffix |
|---|---|
| REST body, in or out | `Dto` — `LoginDto` |
| Queue payload, imperative ("do X", verb-led) | `Command` — `TranscodeCommand` |
| Queue payload, published fact (past tense) | `Event` — `OrderPlacedEvent` |
| WS message payload | `Dto` — `SendMessageDto` |
| GraphQL input, hand-written | `Input` |

A queue payload is a producer↔worker contract, so it lives at the port and the
processor imports it. The entity is the exception: it stays `Model` in
`entity.rs`, its `#[expose]`d wire struct keeps the bare entity name, and the
generated `Create<E>` / `Update<E>` are bare too.

## Errors

**`thiserror` in a library, `anyhow` at the binary's entry point.** A service
returning `anyhow::Result` hands its caller a string: the transport can only
stringify it, and no caller can tell "not found" from "the backend is down".
Domain errors are an enum in `error.rs` — never scattered through
`service.rs` — and they propagate as `Result` all the way to the transport
boundary, which maps them to a status code.

**The enum also says whose mistake each variant is** (`blames_the_caller`),
because that is the one thing the adapter cannot read off a message. A refusal
the caller can act on reaches the model verbatim, so it can correct itself and
call again; everything else leaves through the framework's `opaque`, which
keeps the real error for the operator and hands the model a constant. A tool
that renders a server failure itself is re-implementing that posture — the
adapter's `Answered` trait is where the two answers are chosen, and nowhere
else.

## Configuration

Every module's config is settable **both** ways: from the environment and
pinned in code. A field that only exists in one of the two is incomplete.

**Never spell a variable name as a literal** — not in a message, not in a
check, not in a doc comment. `NESTRS_ENV_PREFIX` is set on the process and
renames every framework variable at once, so a name typed by hand points at
nothing the day it changes, and the compiler never notices. Build it
(`nest_rs_config::var_name`, `EnvPrefix::var`) or name the setting in words.

## Dependencies

**One framework line.** `cargo add nest-rs --features <capability>` — never a
`nest-rs-*` sub-crate. The manifest names only what your own source names.

## Observability

A constant event-name message plus structured fields, never interpolation —
the output is JSON. `tracing::info!(target: "features::users", user_id = %id,
"created user")`, not a formatted sentence. **Every event carries at least one
field**; a bare log is a defect, and the events queried under an incident are
exactly the ones people emit bare. Controllers log `info` on success, services
`debug`, denials and security events `warn` or above.

**The target is rooted at the crate that emits, never at the product.** One
target per concern per crate: `features::users` here. A crate whose name is
not the product's keeps its own root anyway — the target's one job is to say
where the event came from.

**An act performed on the desktop is `info`, in the service that performs
it** — a key pressed, a pointer moved, a screen captured, the clipboard read
or written, a program launched. This is the one exception to services logging
`debug`, and it is not about diagnostics: the server acts on someone's real
machine, so the trail of what it did is a security record, and a record that
exists only when an operator thought to raise the log level is not one. It
belongs in the service because that is where the act happens — an adapter can
only report what it passed on, and a second transport would have to remember
to report it again.

**Such a line spells the act in fields, never in a sentence.** `action="click"
button="left" x=612 y=335`, not `action="Clicked left at (612, 335)"`. The
questions asked of this trail are "every click below this line" and "how much
text was typed this session", and a phrase answers neither. A field that does
not apply to the act is absent rather than empty — `x` on a keypress would be a
coordinate the act never had. The agent-facing prose is a separate concern and
belongs to the wire type, phrased from the same value so the two cannot drift.

**Such a line counts secrets, never quotes them.** Typed text, clipboard
contents, anything a human put there: the field is a character count. An audit
trail that leaks what it audits is worse than none, and this is the rule that
makes the level above affordable.

## Testing

A test target is always a directory — `tests/<suite>/main.rs`, even for one
file. Exactly two suite names: **`integration`** (in process, no live infra)
and **`e2e`** (needs infrastructure, selected by the nextest binary filter,
never `#[ignore]`). Inside a suite the module tree mirrors `src/`, and
`main.rs` holds the `mod` list and shared fixtures — no test function.
Unit tests stay in `#[cfg(test)] mod tests` in the file under test.

## Commands

`nestrs run` is the single front door: `dev`, `start`, `build`, `lint`,
`check`, `test <unit|e2e|cov|doc>`. This project's framework variables carry
the `GHOSTDESK_` prefix.

## Known deviations

Listed so neither a human nor an agent propagates them — or "fixes" one without
knowing why it is there. Do not copy these shapes into new code.

| Where | Rule broken | Status |
|---|---|---|
| `programs/service.rs` — `SCRUBBED_PREFIX` spells the env prefix | never a variable name as a literal | **kept, deliberately.** The container's own knobs are set straight on the process without passing through the framework, so following `EnvPrefix::current()` would stop sweeping them the day the two diverged. The scrub is a security boundary; the rule loses to it. |
| `apps/ghostdesk/tests/` — `e2e/` is empty and `integration/` boots the real `GhostdeskModule` | integration = no app boot; wiring proof lives in e2e | **kept, and now true of the whole suite.** The wiring assertions need a boot but no desktop — `InputService::warm_up` reports an unbound compositor rather than aborting, so `TestApp` boots on a runner with no display. What still belongs in `e2e/` is anything that drives the desk. |
| `apps/ghostdesk/src/mcp/guard.rs` — `CallTrail` is a guard that only observes | a guard decides access | **kept until the framework offers an observation seam.** The endpoint now files its own line per operation, but it names the JSON-RPC method — one word for all fourteen tools. The per-operation chain stays the one place `host`/`kind`/`name` reach an application; the alternative was the same line hand-written in every tool body. It never refuses, and every host is written with `#[tools]`, so it covers all four. |
| `crates/platform/` contracts return `anyhow::Result` | thiserror in a library | **kept, deliberately.** The backends are opaque OS seams; the typed classification each caller needs happens once, at the feature boundary, in each domain's `error.rs`. Typing the substrate would duplicate that vocabulary one layer down with nothing new to say. |

Everything else that used to be listed here is done: the MCP edge is one
`<module>/mcp/` adapter per domain merged onto one `/mcp`, the endpoint's
identity is declared by the app, `apps/` is `programs/`, `host.rs` is
`host/module.rs`, `idle/tasks.rs` is `idle/schedule/tasks.rs`, `auth/guard.rs`
is a `Strategy` behind the framework's `AuthnGuard`, every service returns a
domain enum from `error.rs`,
and span targets are rooted at `features::`. The framework carries the request
span across rmcp's spawn and opens the operation span itself, so nothing here
propagates one by hand; every line carries its own trace ids and every edge
files its own access line, with no observability stack mounted.

## How the endpoint is composed

Read this before adding a domain or a tool — it is the shape the rest of the
tree now assumes.

1. **One `mcp/` adapter per module.** `<module>/mcp/tool.rs` + `<Module>Tool` +
   `<Module>McpModule`, each a bare `#[mcp]` — the decorator's default path is
   `/mcp`, so writing it would restate a framework constant, and a host that
   ever needed to stand apart is the only one that spells a path.
   Each keeps its own `#[inject]` dependencies; none of them learns that it
   shares the endpoint. `tools/list` is the union, `tools/call` is routed by
   name, and **a tool name claimed by two hosts fails boot naming both** — the
   god-host could hide that collision, the endpoint cannot.
2. **`#[mcp]` on the struct, `#[tools]` on the impl, and that is the whole
   file.** No `rmcp`, no `ServerHandler`, no router, no `get_info` — the
   framework writes all of it, and capabilities are derived from the operations
   present so nothing can advertise a surface no method serves. Every `#[tool]`
   declares a posture: `#[public]` here, because the authentication guard gates the
   whole endpoint and GhostDesk has no per-caller ability model. **Arguments
   that carry `#[validate]` are wrapped in `Valid<T>` and destructured in the
   signature** — `Parameters(Valid(params))`: the framework makes the field
   public for exactly that, so an `into_inner()` in the body restates what the
   pattern already did, and a hand-written `params.validate()` is the inline
   edge conversion the layer rules call drift.
   **No host drops out of the sugar.** rmcp's `#[tool_router]` /
   `#[tool_handler]` are the way out, and the one thing that used to earn them
   here — a hand-written `ServerHandler` serving `list_resources` /
   `read_resource`, which `#[tools]` cannot generate a second of — is gone:
   `ghostdesk://apps` and `ghostdesk://clipboard` duplicated `app_list` and
   `clipboard_get` under a second addressing scheme, and no client read them.
   Four hosts, four `#[tools]`, one shape.
3. **The app declares who it is**, in `apps/ghostdesk/src/module.rs`:
   `McpModule::for_root(McpOptions { server: Some(McpIdentity::new("ghostdesk", …).instructions(…).icons(…)), .. })`.
   The identity carries no path — the endpoint is named by the hosts that join
   it — and `version` is the app's alone, since no shared host knows the
   deployment's. Identity is declared, capabilities stay *observed* from the
   hosts, so nothing can advertise a tool no host serves. The session brief and
   the product's icons live in `apps/ghostdesk/src/mcp/` for the same reason:
   they describe the whole surface, which no single host can see.
4. **The per-call binding is app-local, and the root stays pure imports.**
   `DesktopContext` (`dyn McpToolContext`) is resolved once per path and is
   glue over idle, the coordinate space and the caller's identity — so it sits
   in `apps/ghostdesk/src/mcp/context.rs`, not in a feature. It is provided by
   `DesktopContextModule` (`mcp/module.rs`), which imports the port it
   injects; `features` exports that port for exactly this consumer, and the
   root module composes imports and declares nothing.
5. **Who called is the transport's answer, not a feature's.** The HTTP edge
   resolves the caller once — the peer, or a forwarding header only from a
   proxy named in `GHOSTDESK_HTTP__TRUSTED_PROXIES` — and files it on the
   request's own line with the ids everything else is grouped under.
   `DesktopContext` deliberately does not read it a second time: two
   resolutions are two chances to disagree, and a trail whose address does not
   match the one the throttler bucketed is unauditable. It also opens no span
   of its own: a span is a unit of work an operator asks about, and neither
   arming the watchdog nor reading the screen's geometry is one. What `around`
   keeps is what no layer above can know: the idle watchdog, and the agent's
   coordinate space.

   **Four lines make one record, joined by `trace_id`.** What the desktop did
   (`features::*`, from the service that did it — see *Observability*), which
   tool was addressed (`ghostdesk::mcp`), what the endpoint served and whether
   it succeeded, and what the request cost (both `nest_rs::operation`). The
   endpoint is stateless, so one request is one operation and they line up
   exactly. Each line carries the ids itself: none of them is read by being
   nested under another, and a line quoted out of a console keeps its trace.

   **Which tool was addressed comes from a guard, and that is a trade.** The
   endpoint files the operation's own line, but it names the JSON-RPC method a
   client called — `operation="tools/call"`, the same word for all fourteen
   tools. `host`/`kind`/`name` reach an application in exactly one place — the
   per-operation guard chain `#[tools]` emits — so `CallTrail` overrides
   `check_mcp`, logs, and always returns `Ok`. Everything else on this path is
   blind by construction: `around` is handed an `OperationValue` a wrapper may
   only test for `Ok`/`Err`, and `#[tools]` rejects a per-operation interceptor
   with a named compile error. **It covers all four hosts**, since every one of
   them is written with `#[tools]` and so runs the chain.
6. **The endpoint serves tools and nothing else.** No host publishes a
   resource or a prompt, so the capabilities the framework observes are tools
   alone. A host that ever needs a resource would have to hand-write
   `ServerHandler` and leave `#[tools]` behind — and with it the per-operation
   guard chain that names the tool in the trail. That is the trade to weigh
   before adding one, not a formality.
