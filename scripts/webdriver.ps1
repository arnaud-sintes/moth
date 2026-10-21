# Minimal WebDriver client for the end-to-end scripts. Dot-source it: `. $PSScriptRoot\webdriver.ps1`.
# (No `$ErrorActionPreference = "Stop"` anywhere: Windows PowerShell 5.1 treats native stderr as an error.)

$script:wdBase = "http://127.0.0.1:9516"
$script:wdElementKey = "element-6066-11e4-a52e-4f735466cecf"
$script:wdShadowKey = "shadow-6066-11e4-a52e-4f735466cecf"
$script:failures = New-Object System.Collections.Generic.List[string]

$script:backEndPort = 2000

function Test-Port($port) {
	$client = New-Object System.Net.Sockets.TcpClient
	try { $client.Connect("127.0.0.1", $port); $true } catch { $false } finally { $client.Close() }
}

# Starts the sample back-end (already built) and waits until it listens. It refuses to start if the
# port is taken: tests would otherwise silently talk to whatever listens there (a stale build, another
# run) while the server just started exits. Honours CARGO_TARGET_DIR.
function Start-BackEnd($arguments, $logFile) {
	if (Test-Port $script:backEndPort) { Write-Error "port $script:backEndPort is already in use"; exit 3 }
	$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "target" }
	$start = @{
		FilePath = (Join-Path $targetDir "debug\moth-sample-back-end.exe")
		ArgumentList = $arguments
		PassThru = $true
		WindowStyle = "Hidden"
	}
	if ($logFile) { $start.RedirectStandardOutput = $logFile; $start.RedirectStandardError = "$logFile.err" }
	$process = Start-Process @start
	for ($i = 0; $i -lt 50; $i++) {
		if ($process.HasExited) { Write-Error "the back-end exited with code $($process.ExitCode)"; exit 3 }
		if (Test-Port $script:backEndPort) { return $process }
		Start-Sleep -Milliseconds 100
	}
	Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
	Write-Error "the back-end did not start listening on port $script:backEndPort"
	exit 3
}

# What the back-end printed: its stdout, then its stderr (warnings, refusals).
function Read-BackEndLog($logFile) {
	$parts = @($logFile, "$logFile.err") | Where-Object { Test-Path $_ } | ForEach-Object { Get-Content $_ -Raw }
	($parts -join "`n")
}

function Find-EdgeDriver {

	$path = $env:MSEDGEDRIVER
	if (-not $path) { $path = (Get-Command msedgedriver -ErrorAction SilentlyContinue).Source }
	if (-not $path) { Write-Error "msedgedriver not found (PATH or MSEDGEDRIVER)"; exit 2 }
	$path
}

function Start-EdgeDriver {
	$process = Start-Process -FilePath (Find-EdgeDriver) -ArgumentList "--port=9516" -PassThru -WindowStyle Hidden
	Start-Sleep -Seconds 2
	$process
}

function Wd($method, $path, $body) {
	$uri = "$script:wdBase$path"
	if ($body -ne $null) {
		(Invoke-RestMethod -Method $method -Uri $uri -ContentType "application/json" -Body ($body | ConvertTo-Json -Depth 10 -Compress)).value
	} else {
		(Invoke-RestMethod -Method $method -Uri $uri).value
	}
}

function New-Browser($windowSize = "1280,800") {
	$created = Wd POST "/session" @{
		capabilities = @{ alwaysMatch = @{ browserName = "MicrosoftEdge"; "ms:edgeOptions" = @{ args = @("--headless=new", "--disable-gpu", "--window-size=$windowSize") } } }
	}
	$created.sessionId
}

# A screenshot of the page, as a PNG file.
function Save-Screenshot($session, $path) {
	[IO.File]::WriteAllBytes($path, [Convert]::FromBase64String((Wd GET "/session/$session/screenshot" $null)))
}

function Close-Browser($session) {
	if ($session) { try { Wd DELETE "/session/$session" $null | Out-Null } catch {} }
}

function Open-Page($session, $url) { Wd POST "/session/$session/url" @{ url = $url } | Out-Null }

function Run-Script($session, $script) { Wd POST "/session/$session/execute/sync" @{ args = @(); script = $script } }

function Find-Element($session, $selector) {
	(Wd POST "/session/$session/element" @{ using = "css selector"; value = $selector }).$script:wdElementKey
}

# A real click at the element's center (so it goes through the shadow DOM like a user's).
function Click-Element($session, $selector) {
	Wd POST "/session/$session/element/$(Find-Element $session $selector)/click" @{} | Out-Null
}

# Real keystrokes into an element inside a custom element's shadow root.
function Type-Into-Shadow($session, $hostSelector, $innerSelector, $text) {
	$host_ = Find-Element $session $hostSelector
	$shadow = (Wd GET "/session/$session/element/$host_/shadow" $null).$script:wdShadowKey
	$inner = (Wd POST "/session/$session/shadow/$shadow/element" @{ using = "css selector"; value = $innerSelector }).$script:wdElementKey
	Wd POST "/session/$session/element/$inner/value" @{ text = $text } | Out-Null
}

# An element inside the shadow root of a custom element.
function Find-InShadow($session, $hostSelector, $innerSelector) {
	$host_ = Find-Element $session $hostSelector
	$shadow = (Wd GET "/session/$session/element/$host_/shadow" $null).$script:wdShadowKey
	(Wd POST "/session/$session/shadow/$shadow/element" @{ using = "css selector"; value = $innerSelector }).$script:wdElementKey
}

function Click-Id($session, $elementId) { Wd POST "/session/$session/element/$elementId/click" @{} | Out-Null }

# Real keystrokes: plain text, or WebDriver key codes (ArrowLeft "", ArrowRight "", Home "", End "").
function Send-Keys($session, $elementId, $keys) { Wd POST "/session/$session/element/$elementId/value" @{ text = $keys } | Out-Null }

$script:Keys = @{ Left = [string][char]0xE012; Right = [string][char]0xE014; Home = [string][char]0xE011; End = [string][char]0xE010 }

# Pointer steps, in one call; the pointer stays where the last step left it (and pressed, if it was
# not released) until the next call, so a drag can be watched while it is still going on.
function Invoke-Pointer($session, $steps) {
	$mouse = @{ type = "pointer"; id = "mouse"; parameters = @{ pointerType = "mouse" }; actions = @($steps) }
	Wd POST "/session/$session/actions" @{ actions = @($mouse) } | Out-Null
}
function Move-To($x, $y, $ms = 0) { @{ type = "pointerMove"; duration = $ms; x = [int]$x; y = [int]$y; origin = "viewport" } }
$script:Down = @{ type = "pointerDown"; button = 0 }
$script:Up = @{ type = "pointerUp"; button = 0 }

# A real pointer drag: press at (x, y) in the page, move by (dx, dy) over `ms`, release.
function Drag-Pointer($session, $x, $y, $dx, $dy, $ms = 300) {
	$mouse = @{ type = "pointer"; id = "mouse"; parameters = @{ pointerType = "mouse" }; actions = @(
		@{ type = "pointerMove"; duration = 0; x = [int]$x; y = [int]$y; origin = "viewport" },
		@{ type = "pointerDown"; button = 0 },
		@{ type = "pointerMove"; duration = $ms; x = [int]($x + $dx); y = [int]($y + $dy); origin = "viewport" },
		@{ type = "pointerUp"; button = 0 }
	) }
	Wd POST "/session/$session/actions" @{ actions = @($mouse) } | Out-Null
}

# A real wheel turn at (x, y) in the page.
function Scroll-Wheel($session, $x, $y, $deltaY) {
	$wheel = @{ type = "wheel"; id = "wheel"; actions = @(
		@{ type = "scroll"; x = [int]$x; y = [int]$y; deltaX = 0; deltaY = [int]$deltaY; duration = 0; origin = "viewport" }
	) }
	Wd POST "/session/$session/actions" @{ actions = @($wheel) } | Out-Null
}

# Re-evaluate `$condition` (a scriptblock) until it is true, 5 s at most (or `$seconds`).
function Wait-For($what, [scriptblock]$condition, $seconds = 5) {
	for ($i = 0; $i -lt ($seconds * 10); $i++) {
		# an error while the condition is looked at (a page that has not built its elements yet, right
		# after a fresh build) is "not yet", not the end of the run
		$met = $false
		try { $met = & $condition } catch { }
		if ($met) { return $true }
		Start-Sleep -Milliseconds 100
	}
	return $false
}

function Expect($condition, $message) {
	if ($condition) { Write-Host "  ok   $message" } else { Write-Host "  FAIL $message"; $script:failures.Add($message) }
}
