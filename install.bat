REM get the emsdk repo:
git clone https://github.com/emscripten-core/emsdk.git
cd emsdk

REM download and install the latest SDK tools:
emsdk install latest

REM make the "latest" SDK "active" for the current user:
REM TODO not working in the batch?
emsdk activate latest
