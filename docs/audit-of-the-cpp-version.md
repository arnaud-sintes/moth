# moth audit

**Purpose:** personal C++20/emscripten OOP web framework. Rejects JS/TS/React virtual-DOM approach; aims for a WPF/Qt-like model over HTML/CSS, direct DOM binding via wasm, plus a WebSocket client. Concept is coherent, not far-fetched.

## Architecture

`Item` (id + parent/child) → `DataPool` (observable properties, `onUpdate`) → `Html::Element` (DOM node). Coherent design, a genuine alternative to virtual-DOM (push property change straight to DOM instead of diffing a tree). Good idea in principle.

## Solid

- `Status.h`: strong exception-free error type, well suited to wasm (no EH available).
- Json API: stream-operator based serialize, JsonPath, reflection macro comparable to nlohmann in usability.

## Risky / fragile — needs fixing

- **Lifetime is not actually solved** (the framework's core selling point). `Item::m_parent` is a non-owning reference (`Item.h:51`). If the parent dies first → dangling deref (`Item.cpp:21-22`); `Element::~Element` calls `getElementById(id).remove()` on an already-gone node → JS `TypeError` (`Html.cpp:28`). All `Js::Dom::*` calls do unchecked `getElementById` lookups (`Js.cpp:65,75,105,134,153`). No real ownership model — string-id lookup, no dangling detection, and slower than the claimed perf win.
- **`WebSocket::Send` polarity inverted**: returns the emscripten result where 0 = success, treated as if it were a bool success flag (`WebSocket.cpp:62`). Silent failure bug.
- **`WebSocket` callbacks capture raw `this`**, destructor deletes without guaranteeing no in-flight callback (`WebSocket.cpp:41-55,70`). Use-after-free risk.
- **`Data<T>` template effectively only works for `wstring`** — the bool path calls `std::stoi` on the wrong type (`Data.cpp:20`, `Data.hpp:31/42`). Claims generality, isn't generic.
- **Serialization uses comma-join, no escaping** — a comma in text corrupts the stream (`Data.cpp:31`, `Item.cpp:35`).
- **`IndexedMap::erase` broken**: wrong iterator type passed to `list::erase`, wrong iterator type returned (`IndexedMap.h:66-71`).
- **`Ctti.h` is compiler-locked**: hardcoded `"[_Type = Moth::"` string match on Clang's `__PRETTY_FUNCTION__` (`Ctti.h:37`). Breaks on non-Clang, breaks on non-`Moth`-namespace types → UB (`npos`).
- Macro-heavy metaprogramming (`VariadicExpanderMacros.h`, 40-way manual unroll) — works but unreadable, silently mis-expands past the limit.
- Const-correctness hacks: `mutable m_children` + const `RegisterChild` (`Item.h:44-53`); `Status::operator=` returns `Status&&` — move-only despite the readme claiming copyable.

## Bottom line

Concept is sound and matches the stated purpose. The core promise ("proper destructors, no dangling") isn't actually delivered yet — the ownership model is still stringly-typed lookup, with no RAII guarantee that a parent outlives its children.

**Fix priority:**
1. `WebSocket::Send` polarity (silent bug)
2. `Item` parent/child ownership (make it real — e.g. `unique_ptr` children, or null-out on parent destruction)
3. Null-guard every `getElementById` call
4. `IndexedMap::erase`
5. `Data<T>` genericity
