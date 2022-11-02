# moth library

## Purpose

Provides multiple components to properly create HTML structured web-pages using C++.

## Content

### [moth]

Single header to ease inclusions, single source file to ease compilation.

### AnsiToWideMacros

Macros to convert ANSI declarations to Wide chars (UCS-2).

### Base64

Base64 encoding/decoding.

### Buffer

Dealing with weak-possessed memory spaces.

### Ctti

*Compile-Time Type Information* helpers, typically to compute class names or hashes at compile time.

### Data

moth Data object (Model part of the MVC).

### Html

Provides C++ wrappers of HTML components (element, body, division, label, button...).

### IndexedMap

Basically an *unordered_map* but keeping the insertion order.

### Item

moth hierarchized item system.

### Js

JavaScript calls wrapper.

### Json

Dealing with JSON serialization/deserialization.

### Lambda

Js to C++ lambda manager.

### Status

Error management in C++.

### StdIncludes

Single-header standard library inclusions.

### System

All purposes methods.

### VariadicExpanderMacros

Helper to deal with complex expanded calls.

### WebSocket

C++ abstraction of emscripten websocket.

## TODO list

* rework serialization/deserialization mechanism
* centralized model+controller (data/empty shell component replication/action redirections)
* other Html elements: edit, drop down, slider, progress bar...
* canvas support
