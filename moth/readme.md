# 🦋 moth library

## Purpose

Provides multiple C++ components to properly create HTML structured web-pages via *emscripten*.

## Content

### [moth]

Single *header* to ease *moth* inclusions, with a single *source* file to ease the *emcc* compilation.

### AnsiToWideMacros

Macros to convert ANSI string declarations to Wide chars (UCS-2), adding an "*L*" prefix.

### Base64

*Base64* encoding/decoding (*unused yet*).

### Buffer

Dealing with raw memory buffers (weak possession container), allowing *to/from* conversions.

### Ctti

*Compile-Time Type Information* helpers, typically to compute *class names* and *FNV1-a hashes* at compile time.

### Data

*moth* HTML components data object (the *Model* part of the *MVC*).

### Html

Provides C++ equivalences of the following HTML components:

- **Body**, a "*div*" element without any parent
- **Division**, all-purposes "*div*" element
- **Label**, a "*span*" element
- **Image**, an "*img*" element
- **PushButton**, a "*button*" element
- **ToggleButton**, a "*button*" element with toggling capabilities

### IndexedMap

An *unordered_map* respecting the emplacing order.

### Item

*moth* hierarchized (parent/child) item system.

### Js

*JavaScript* calls wrapper, covering:

- *Console* accesses
- *DOM* manipulation
- *initialization* (setting page title and icon)

### Json

Dealing with JSON simple and structured serialization/deserialization and *JSON path*, covering what's needed in the C++ standard library.

### Lambda

Allowing JavaScript to call properly C++ lambdas.

### Status

*Error* and *success* management in C++ (not relying over *exceptions*).

### StdIncludes

Single-header **standard library** inclusions.

### System

All purposes methods (output messages, *assertions*, *Mbcs/Wide conversions* etc.).

### VariadicExpanderMacros

Helper to deal with code generation based over variadic arguments.

### WebSocket

C++ abstraction of *emscripten* client *WebSocket*.

## TODO list

* rework serialization/deserialization mechanism
* centralized model+controller (data/empty shell component replication/action redirections)
* other Html elements: edit, drop down, slider, progress bar...
* canvas support
