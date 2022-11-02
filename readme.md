# 🦋 moth

## Purpose

Create HTML structured web-pages using C++ via *emscripten* ([webassembly](https://en.wikipedia.org/wiki/WebAssembly)).

## Folder structure

```
moth/
samples/
emsdk/
```

* *moth* is the folder containing the [C++ *moth* library data](./moth/readme.md)
* *samples* is the folder container [*moth* samples](./samples/readme.md)
* *emsdk* is the folder where you need to install [emscripten](https://emscripten.org/)

## Installation

You just need to execute "***install.bat***" that will automatically clone the *emscripten* SDK and install it in the "*emsdk/*" folder.

## Test

You can test if everything's ok by:

- [running the back-end sample](./samples/back-end/readme.md)
- [compile and running the front-end sample](./samples/front-end/readme.md)
