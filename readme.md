# 🦋 *moth*

## Purpose

### Problems statement

*JavaScript* is a mess and *TypeScript* just tries to cover the mess, with extra performance costs.

*JavaScript* doesn't have any mortal-accessible notion of destructor, which is a pain when trying to do proper OOP with dynamic instances management.

*JavaScript* DOM manipulation is way too low-level and is lacking of basic structured object management.

All of these led to "workaround libraries" additions like *React* that may seem acceptable by some (weird) people but that are clearly not: the "*virtual DOM*" concept is a bad way to assume that we're unable to deal properly with DOM elements individually - introducing again some extra performance loss because of a systematic tree comparison - and trying to apply basics MVC or MVVM patterns on top of these stack to do things properly requires more additional libraries (like *Redux*) and lot of useless headaches because of multiple fancy new concepts that are revolutionary only to people with no computer science background at all.

Any (*so-called nowadays*) "back-end" developers that got some experience with *WPF* or *Qt* frameworks barely understood all of this and sadly, yes, I'm part of it, even when trying hard to figure it out.

On the other side, client/server based architecture are clever by nature, *HTML* got the huge advantage of being extra simple (structured *markup*) and quite fast to use when creating web-pages, and most of all: *CSS* is really a no-brainer when it goes to style an UI and focus on UX alone (even *Qt* tries to mimic it then).

### A step forward

Recently, the W3C introduces [WebAssembly](https://en.wikipedia.org/wiki/WebAssembly), that really help to feed the gap between the two worlds by proposing to rely on proper OOP languages (*C++, Rust, C#*...) to build web-browser compatible programs: thanks to the *LLVM* intermediate language and a subsequent *wasm* compilation, we can now switch back to C# (*Blazor*) or C++ (*Qt*) to build our web-pages without (or with very few) *JavaScript* (at least required for DOM manipulation), which is a very clever move.

Sadly, the two aforementioned frameworks suffers to be very complex (big) things and lost their interest when we just want to achieve really simple things (but with proper OOP).

### Proposed solution

*moth* proposes then a lightweight OOP framework to create dynamic *HTML* structured web-pages (including also a *WebSocket* client) using the **C++20 language**, thanks to the usage of the ***emscripten*** [toolchain](https://emscripten.org/) (*C++/Clang/LLVM/wasm* compiler) and libraries.

> Note *moth* is currently built on the *Windows OS*, but as long as everything is *Clang* & *NodeJs* based it must be really easy to port it on other systems.

## Hello world example

To understand how it works, let's consider the following classic "*hello world*" example:

```c++
#include "../../moth/[moth].h"

namespace Moth {

class Page
{
private:
	Page()
		: m_label{ m_body, L"Hello world" }
	{}
	
public:
	static Page & GetInstance()
	{
		static Page instance;
		return instance;
	}
	
private:
	Html::Body m_body;
	Html::Label m_label;
};

} // Moth

int main()
{
	Moth::Page::GetInstance();
	return 0;
}
```

As we can see, we retrieve our classical "*main*" entry point that just creates a single instance of our custom web-page, defining on its own a *body* root element and a *label* child element.

After compiling this using the *emscripten* toolchain you will get a "*main.wasm*" compiled *WebAssembly* file and a related "*main.js*" file (which is a generated helper use to load the *wasm*).

You just need then to create a default (very) minimal HTML page that will be used as the host for the *WebAssembly* (and optionally a *CSS* file to style your page):

```html
<!doctype html>
<html lang="en">
	<head>
		<meta charset="utf-8">
		<link rel="stylesheet" href="style.css">
	</head>
	<body>
		<script async src="main.js"></script>
	</body>
</html>
```

The page content itself will be created dynamically thanks to *moth*.

## *moth* repository folder structure

```
moth/
samples/
	back-end/
	front-end/
emsdk/
```

* *moth* is the folder containing the [C++ *moth* library data](./moth/readme.md) (built on top of the *emscripten* C++ libraries)
* *samples* is the folder container [*moth* samples](./samples/readme.md)

  > Note *back-end* samples relies on *NodeJs*, typically to ease the creation of *HTTP* and *WebSocket* servers
* *emsdk* is the folder where we need to install [emscripten](https://emscripten.org/) to build things properly

## *emscripten* installation

We just need to execute the "***install.bat***" batch at the repository root, that will automatically clone the *emscripten* SDK and install it in the "*emsdk/*" folder.

## Test

We can then test if everything's ok by:

- [running the back-end sample](./samples/back-end/readme.md)
- [compile and running the front-end sample](./samples/front-end/readme.md)

## TODO list

My personal *moth* TODO list:

* rework serialization/deserialization mechanism (use JSON)
* centralized model+controller (data/empty shell component replication/action redirections)
* add other Html elements: edit, drop down, slider, progress bar...
* smart canvas support
