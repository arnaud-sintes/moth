# moth samples/front-end

## Purpose

This is a very simple C++ sample based on moth.

## Content

- *main.cpp* C++ sample file
- *html/index.html* HTML sample front-end page root
- *html/style.css* CSS sample content

## How to compile

```
"../../emsdk/emsdk_env.bat"
emcc -Wall -O3 ../../moth/[moth].cpp main.cpp -std=c++20 -sEXPORTED_FUNCTIONS=_CallCppLambda,_main -lwebsocket.js -sLLD_REPORT_UNDEFINED -sASSERTIONS -sNO_DISABLE_EXCEPTION_CATCHING -o html/main.js
```

This will activate first the *emscripten* compilation environment, then build both the *moth* library and the current sample (*main.cpp*).

The compilation output is stored in the "*html/*" folder:

* *html/main.js* emscripten javascript entry point
* *html/main.wasm* generated webassembly content

## Other helpful flags

Note some compilation flags may helps in the future (around thread management):

```
-sPTHREAD_POOL_SIZE=2 -sUSE_PTHREADS -sPROXY_TO_PTHREAD
```

## Execution

Execute the *back-end runner*.

Connect to "http://localhost:1000".
