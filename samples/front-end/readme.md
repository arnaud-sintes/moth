# moth samples/front-end

## Purpose

This is a very simple C++ sample based on moth.

## Content

- *main.cpp* C++ sample file
- *html/index.html* HTML sample front-end page root
- *html/style.css* CSS sample content

## How to compile

The first time, you need to activate *emscripten* SDK:

```
"../../emsdk/emsdk_env.bat"
```

Then you can safely compile the "*main.cpp*" sample file:

```
emcc -Wall -O3 ../../moth/[moth].cpp main.cpp -std=c++20 -sEXPORTED_FUNCTIONS=_CallCppLambda,_main -lwebsocket.js -sLLD_REPORT_UNDEFINED -sASSERTIONS -sNO_DISABLE_EXCEPTION_CATCHING -o html/main.js
```

The compilation output is stored in the "*html/*" folder:

* *html/main.js* emscripten javascript entry point
* *html/main.wasm* generated webassembly content

## Execution

Execute the [*back-end runner*](../back-end/readme.md) first.

Connect to "http://localhost:1000": you must see the sample webpage and be able to exchange data with the back-end.

## Other helpful flags

Note some compilation flags may helps in the future (around thread management):

```
-sPTHREAD_POOL_SIZE=2 -sUSE_PTHREADS -sPROXY_TO_PTHREAD
```
