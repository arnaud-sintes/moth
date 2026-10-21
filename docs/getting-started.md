# Getting started

This guide takes you from nothing to a page running in your browser, explains what each part of the starting project does, lists the dependencies you may need, and ends with the problems people meet first. After it: the [tutorial](tutorial.md) builds a real application step by step, [writing components](components.md) shows how to make your own, and [styling](styling.md) how to make it look the way you want.

moth2 is version 0.1.0 and not published: you depend on it by path, and its API may still change. It targets Chromium (Chrome, Edge) only.

## What you need

- **Rust** with the WebAssembly target. moth2 is built and tested with Rust 1.98; the real minimum has not been determined (edition 2024 needs at least 1.85).
- **trunk**, which builds the WebAssembly, generates the JavaScript glue and serves the page.
- **A Chromium-based browser.** Firefox and Safari are not supported, by decision: the code uses Chromium's platform freely.

```
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
```

## The starting project

[`templates/hello`](../templates/hello) is a page with one tree of components, one callback and a little CSS. Copy the folder anywhere, change the `moth` path in `Cargo.toml` to where moth2 lives, and run `trunk serve` in it: it builds, serves the page on <http://127.0.0.1:8080> and rebuilds when a file changes.

Its files, in full. `Cargo.toml`:

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2024"

[dependencies]
# where moth2's `moth` crate is, relative to this file (change it when you copy the template)
moth = { path = "../../moth" }

# Naming a browser type in your own code (an event type, an element type) needs the matching feature
# of web-sys switched on here, for example:
#
#   [dependencies.web-sys]
#   version = "0.3"
#   features = ["HtmlElement", "KeyboardEvent", "PointerEvent"]
#
# `moth` itself needs none of them from you: see docs/getting-started.md for the list by task.
```

`index.html` is what trunk builds from. The `rust` link says "build the crate next to me and load it"; the `css` link copies a stylesheet next to it:

```html
<!doctype html>
<html lang="en">
	<head>
		<meta charset="utf-8">
		<meta name="viewport" content="width=device-width, initial-scale=1">
		<title>Hello from moth</title>
		<link data-trunk rel="rust" href="Cargo.toml">
		<link data-trunk rel="css" href="style.css">
	</head>
	<body></body>
</html>
```

`src/main.rs`:

```rust
use std::{cell::Cell, rc::Rc};

use moth::{Body, Component, Label, Panel, PushButton};

/// Everything the page owns. Dropping it removes it all from the DOM.
struct Page {
	_body: Body,
	_card: Panel,
	_title: Label,
	_count: Label,
	_button: PushButton,
}

fn main() {
	// panics go to the browser console, with their file and line, instead of an opaque "unreachable"
	moth::init();

	let body = Body::new().unwrap();
	let card = Panel::new(&body).unwrap();
	card.element().set_class_name("card");
	let title = Label::new(&card, "Hello from moth").unwrap();
	let count = Label::new(&card, "No clicks yet").unwrap();

	// callbacks capture a non-owning handle to what they change, never the component itself
	let text = count.text_handle();
	let clicks = Rc::new(Cell::new(0));
	let button = PushButton::new(&card, "Click me", move || {
		clicks.set(clicks.get() + 1);
		text.set(format!("Clicked {} times", clicks.get()));
	})
	.unwrap();

	// keep the page alive until the browser tab goes away
	moth::mount(Page { _body: body, _card: card, _title: title, _count: count, _button: button })
		.unwrap();
}
```

`style.css`:

```css
/* The page's own CSS lays components out and sets the variables that style them (see docs/styling.md).
   The inside of a component is in its shadow root: this stylesheet does not reach it, the variables
   and ::part() do. */

:root {
	--moth-accent: #3b82f6;
	--moth-button-background: #3b82f6;
	--moth-button-color: white;
	--moth-button-border: #3b82f6;
	--moth-button-radius: 6px;
	--moth-panel-display: flex;
	--moth-panel-gap: 0.75rem;
}

body {
	font-family: system-ui, sans-serif;
	margin: 0;
	min-height: 100vh;
	display: grid;
	place-items: center;
}

.card {
	flex-direction: column;
	align-items: flex-start;
	padding: 1.5rem 2rem;
	border: 1px solid #ccd;
	border-radius: 10px;
}
```

## What the pieces do

- **`moth::init()`** comes first. WebAssembly cannot unwind, so a panic stops the program; with `init` the message (and the file and line) is written to the browser console, instead of an opaque `unreachable` error.
- **`Body::new()`** creates the root: a `<moth-body>` element inside the page's `<body>`. Every other component is created under a parent, `Label::new(&card, ...)`, and appears in the page at once.
- **Owning.** A component is a Rust value. It lives as long as its owner keeps it, and when it is dropped its element and listeners are removed. That is why the example has a `Page` struct that holds everything: if `main` simply ended, its variables would be dropped and the page would empty at once.
- **`moth::mount(page)`** hands the struct to the framework, which keeps it alive until the browser tab goes away (the one moment Rust cannot run destructors on its own) and then drops it.
- **Callbacks** are closures that live exactly as long as their component. They capture a *handle* to what they change (`count.text_handle()`), never the component, so a callback can never keep something alive or reach something that is gone.
- **`style.css`** is ordinary page CSS. It lays the components out (a component's element is a normal element in the page) and sets the CSS variables that style their insides; see [styling](styling.md).

## Dependencies you may need

`moth` is the only dependency the starting project has. You add others when your own code names browser or JavaScript types:

| You write | Add to `[dependencies]` | Feature to switch on |
|---|---|---|
| `wasm_bindgen::JsValue`, `JsCast` (`dyn_into`, `unchecked_into`), `Closure` | `wasm-bindgen = "0.2"` | |
| a `js_sys` type | `js-sys = "0.3"` | |
| a browser type, for example `web_sys::PointerEvent` | `web-sys = "0.3"` | the type's name: `"PointerEvent"` |
| `async` code that waits for a promise | `wasm-bindgen-futures = "0.4"` | |
| `moth::Link`, `Replica`, `Actions` (a model kept in step with a server) | `moth = { ..., features = ["replication"] }` | |

Features of `web-sys` are the names of the types you use; the ones the samples needed are `Event`, `MouseEvent`, `PointerEvent`, `WheelEvent`, `KeyboardEvent`, `HtmlElement`, `HtmlInputElement`, `HtmlCanvasElement`, `CanvasRenderingContext2d`, `Window`, `Location`, `UrlSearchParams`. The compiler tells you when one is missing: the method or type is "not found" until you add its feature. Calling a method on a type moth hands you works without any of this when moth itself uses that feature (`label.element().set_class_name(...)` does); for a method it does not use (`element().style()` needs `CssStyleDeclaration`) the compiler says the method is not found, and you add the feature.

## Growing the project

- Keep one struct that owns the whole page (`Page` above, or `App`) and give it everything the page creates. A component you forget to store disappears as soon as the function that made it returns.
- Put state in one place as a `Prop` and let components follow it with `bind`, instead of having components poke at each other: the [tutorial](tutorial.md) does this.
- Make a component of your own for each repeated piece (a row, a card, a form field): [writing components](components.md).
- A module per component, and a `mod.rs` or `main.rs` that builds the page, is plenty; there is no required structure.

## Building for release

`trunk build --release` makes an optimized `dist/` folder (WebAssembly optimized by binaryen's `wasm-opt`, which trunk downloads) that any static web server can serve. If the page is not served from the root of its server, say so in a `Trunk.toml` next to `index.html`:

```toml
[build]
public_url = "/my-app/"
```

## Talking to a server

`moth::WebSocket` is a WebSocket client whose callbacks stop when it is dropped. With the `replication` feature, `Link` and `Replica` keep a model that lives on a server in step with the page (the page sends actions, the server answers with the new model, and components follow it with `bind`, as above), and reconnect with backoff when the connection drops. [`samples/todo`](../samples/todo) is a shared todo list built that way, with its server in [`samples/back-end`](../samples/back-end) (Rust, axum); [`samples/viewer`](../samples/viewer) streams images the server renders.

## Testing your own application

moth's own tests run in a real browser with `wasm-bindgen-test`, which is also how to test an application written with it: components exist only in a browser. Put `wasm_bindgen_test_configure!(run_in_browser);` in an integration test, install `wasm-bindgen-cli` at the version of the `wasm-bindgen` crate in your `Cargo.lock` (`cargo install wasm-bindgen-cli --version <that> --locked`), have a WebDriver (`chromedriver` or `msedgedriver`) on your `PATH`, make `wasm-bindgen-test-runner` the runner for the wasm target in `.cargo/config.toml`, and run `cargo test --target wasm32-unknown-unknown`. The files in [`moth/tests`](../moth/tests) are examples; `moth/tests/guides.rs` tests the code in these guides, including how to type into an `Edit` and click a button.

## When something goes wrong

| What you see | Why | What to do |
|---|---|---|
| A blank page, nothing in the console | `main` never ran, or created nothing | Check that `index.html` has `<link data-trunk rel="rust" href="Cargo.toml">`, that you built with trunk (not by opening the file), and that `main` creates a `Body`. Open the browser's developer tools. |
| `RuntimeError: unreachable executed` | A panic, without a hook to say where | Call `moth::init()` first in `main`: the console then shows the panic message with file and line. |
| Components appear for a moment, or never | Their owner was dropped | Something must own every component for as long as you want it: store them in a struct and hand it to `moth::mount`. |
| `closure invoked recursively or after being dropped` | A handler of your own (a `wasm_bindgen::Closure`) is called after what owned it was dropped | Use component callbacks and `moth::Listener`, which are removed when their owner is dropped, instead of leaking or hand-managing closures. |
| Page CSS has no effect on the inside of a component | The inside is in a shadow root | Style the component's element, set its CSS variables, or use `::part()`: [styling](styling.md). |
| `... is already defined`, or a "tag" error | The custom element name of your own component is taken | Give your own elements a name with a hyphen and a prefix of your own (`my-card`, not `moth-...`). |
| `Keyed::sync` returns "duplicate key" | Two items have the same key | Keys must be unique among the items you pass. |
| trunk: `could not find the root package of the target crate` | trunk was not run in the folder with `Cargo.toml` and `index.html` | Run it there, or fix the path in the `rust` link. |
| `can't find crate for core` or a missing `wasm32` target | The WebAssembly target is not installed | `rustup target add wasm32-unknown-unknown` |
| A WebSocket is refused by the sample server | It only accepts pages of its own origin | Serve the page from the server, or run the server with `--dev` while using `trunk serve`. |
| It works in Chrome or Edge and not in Firefox or Safari | Only Chromium is supported | By decision; there is no workaround. |
