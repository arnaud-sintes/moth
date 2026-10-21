# End-to-end check of the streaming sample: builds the front-end, starts the axum back-end serving it,
# drives the page in headless Edge through WebDriver (real clicks), and checks both the DOM and the
# server log.
#
# Needs: trunk, and msedgedriver matching your Edge (on PATH or in $env:MSEDGEDRIVER).
# Usage: .\scripts\e2e.ps1 [-NoBuild]

param([switch]$NoBuild)

$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
. "$PSScriptRoot\webdriver.ps1"
Find-EdgeDriver | Out-Null

if (-not $NoBuild) {
	Push-Location samples\front-end
	trunk build
	$code = $LASTEXITCODE
	Pop-Location
	if ($code -ne 0) { exit $code }
}
cargo build -p moth-sample-back-end
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$serverLog = Join-Path $env:TEMP "moth-e2e-server.log"
$server = Start-BackEnd @("--port", "$script:backEndPort", "--dir", "samples\front-end\dist") $serverLog
$driver = Start-EdgeDriver
$session = $null

$snapshot = @'
const text = id => document.getElementById(id).textContent;
const toggle = document.getElementById("streaming-toggle");
const img = document.getElementById("stream-image").shadowRoot.querySelector("img");
return {
	state: text("stream-state"), fps: text("stream-fps"),
	toggled: toggle.hasAttribute("toggled"),
	pressed: toggle.shadowRoot.querySelector("button").getAttribute("aria-pressed"),
	src: (img.getAttribute("src") || "").slice(0, 23)
};
'@
$declared = @'
const toggle = document.getElementById("declared-toggle");
return {
	label: document.getElementById("declared-label").textContent,
	pressed: toggle.shadowRoot.querySelector("button").getAttribute("aria-pressed"),
	toggled: toggle.hasAttribute("toggled"),
	progress: document.getElementById("declared-progress").shadowRoot.querySelector("progress").getAttribute("value")
};
'@

try {
	$session = New-Browser
	Open-Page $session "http://127.0.0.1:2000/"
	Start-Sleep -Seconds 1

	Write-Host "initial page"
	$s = Run-Script $session $snapshot
	Expect ($s.state -eq "Image streaming: inactive") "state label starts inactive ('$($s.state)')"
	Expect ($s.fps -eq "fps: -") "fps label starts at '-' ('$($s.fps)')"
	Expect (-not $s.toggled -and $s.pressed -eq "false") "toggle starts off, aria-pressed=false"
	Expect ($s.src -eq "") "no image yet"

	Write-Host "elements declared in markup"
	$d = Run-Script $session $declared
	Expect ($d.label -eq "declared in markup:") "declared label kept its text ('$($d.label)')"
	Expect ($d.pressed -eq "false") "declared toggle was upgraded into a component (aria-pressed='$($d.pressed)')"
	Expect ($d.progress -eq "0.4") "declared progress bar got its value ('$($d.progress)')"
	Click-Element $session "#declared-toggle"
	$d = Run-Script $session $declared
	Expect ($d.toggled -and $d.pressed -eq "true") "real click flips the declared toggle (attribute and aria-pressed)"

	Write-Host "stream on"
	Click-Element $session "#streaming-toggle"
	Start-Sleep -Seconds 3
	$s = Run-Script $session $snapshot
	Expect ($s.state -eq "Image streaming: active") "state label follows the toggle ('$($s.state)')"
	Expect ($s.toggled -and $s.pressed -eq "true") "toggle reflected on host and aria-pressed"
	Expect ($s.fps -match '^fps: \d+(\.\d)?$' -and [double]($s.fps -replace 'fps: ', '') -gt 0) "fps label shows a positive rate ('$($s.fps)')"
	Expect ($s.src -eq "data:image/jpeg;base64,") "image received over the socket ('$($s.src)')"

	Write-Host "send button"
	Click-Element $session "#send-button"

	Write-Host "stream off"
	Click-Element $session "#streaming-toggle"
	Start-Sleep -Seconds 1
	$before = (Run-Script $session $snapshot).fps
	Start-Sleep -Milliseconds 1800
	$s = Run-Script $session $snapshot
	Expect ($s.state -eq "Image streaming: inactive") "state label back to inactive"
	Expect ($s.fps -eq $before) "no frames after the stream was stopped ('$before' -> '$($s.fps)')"
}
catch {
	# an error inside the run would otherwise stop it silently and let the script go on to say "passed"
	Write-Host ("  FAIL the run stopped early: " + ($_ | Out-String))
	$script:failures.Add("the run stopped early: $_")
}
finally {
	Close-Browser $session
	Stop-Process -Id $driver.Id -Force -ErrorAction SilentlyContinue
	Start-Sleep -Milliseconds 500
	Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
}

Write-Host "server log"
$log = Read-BackEndLog $serverLog
Write-Host ($log -replace "(?m)^", "  | ")
Expect ($log -match "WebSocket client connection") "server saw the client connect"
Expect ($log -match 'WebSocket receive "\(Front-End\) WebSocket is opened and ready"') "server received the open message"
Expect ($log -match 'WebSocket receive "\(Front-End\) clicking send button"') "server received the send-button message"
Expect ($log -match "Image streaming: active") "server received stream on"
Expect ($log -match "Image streaming: inactive") "server received stream off"
Expect ($log -match "WebSocket client disconnection \((clean|connection lost)\)") "server saw the client go away"

if ($script:failures.Count -gt 0) { Write-Host "`n$($script:failures.Count) check(s) failed"; exit 1 }
Write-Host "`nall checks passed"
