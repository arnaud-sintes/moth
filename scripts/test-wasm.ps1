# Runs the wasm test suite in a real browser. The WebSocket and Link tests need the sample back-end, so
# this builds it, starts it on 127.0.0.1:2000 for the duration of the run, then stops it. `--dev` lets
# it accept the test runner's page (a loopback origin on a random port); without it only same-origin
# pages may open a WebSocket.
#
# Needs: wasm-bindgen-cli matching the wasm-bindgen crate, and a WebDriver matching your browser
# on PATH (msedgedriver / chromedriver / geckodriver) or pointed to by MSEDGEDRIVER & co.

# (no `$ErrorActionPreference = "Stop"`: Windows PowerShell 5.1 treats cargo's stderr as an error)
Set-Location (Split-Path $PSScriptRoot -Parent)
. "$PSScriptRoot\webdriver.ps1"

cargo build -p moth-sample-back-end
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$server = Start-BackEnd @("--port", "$script:backEndPort", "--dir", "samples\front-end\dist", "--dev") $null
try {
	cargo test -p moth --target wasm32-unknown-unknown --all-features
	$code = $LASTEXITCODE
}
finally {
	Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
}
exit $code
