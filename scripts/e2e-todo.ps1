# End-to-end check of the todo sample with TWO browsers on one server: real typing and clicking in
# one, checking that the other follows (model replication), that rows come and go with the model,
# that the canvas shows the progress, and that a reloaded page catches up.
#
# Needs: trunk, and msedgedriver matching your Edge (on PATH or in $env:MSEDGEDRIVER).
# Usage: .\scripts\e2e-todo.ps1 [-NoBuild]

param([switch]$NoBuild)

$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
. "$PSScriptRoot\webdriver.ps1"
Find-EdgeDriver | Out-Null

if (-not $NoBuild) {
	Push-Location samples\todo
	trunk build
	$code = $LASTEXITCODE
	Pop-Location
	if ($code -ne 0) { exit $code }
}
cargo build -p moth-sample-back-end
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$serverLog = Join-Path $env:TEMP "moth-e2e-todo-server.log"
$server = Start-BackEnd @("--port", "$script:backEndPort", "--dir", "samples\front-end\dist", "--todo-dir", "samples\todo\dist") $serverLog
$driver = Start-EdgeDriver
$alice = $null
$bob = $null

$snapshot = @'
const rows = [...document.querySelectorAll("#todo-list > moth-panel")].map(panel => ({
	text: panel.querySelector("moth-label").textContent,
	done: panel.querySelector("moth-toggle-button").hasAttribute("toggled")
}));
return {
	status: document.getElementById("todo-status").textContent,
	summary: document.getElementById("todo-summary").textContent,
	input: document.getElementById("todo-input").shadowRoot.querySelector("input").value,
	rows: rows
};
'@
$pixels = @'
const canvas = document.querySelector("moth-canvas").shadowRoot.querySelector("canvas");
const context = canvas.getContext("2d");
const y = Math.floor(canvas.height / 2);
const at = fraction => [...context.getImageData(Math.floor(canvas.width * fraction), y, 1, 1).data];
return { width: canvas.width, left: at(0.25), right: at(0.75) };
'@

# A raw WebSocket client (no browser): what a page of another origin would do with the server.
function Open-RawSocket($origin) {
	$socket = New-Object System.Net.WebSockets.ClientWebSocket
	if ($origin) { $socket.Options.SetRequestHeader("Origin", $origin) }
	try {
		$socket.ConnectAsync([Uri]"ws://127.0.0.1:$script:backEndPort/todo/ws", [Threading.CancellationToken]::None).Wait(4000) | Out-Null
	} catch { }
	$socket
}

# A .NET client only learns that the server closed by reading: read until a close frame, an error, or 4 s.
function Wait-ForClose($socket) {
	$buffer = New-Object byte[] 65536
	$deadline = (Get-Date).AddSeconds(4)
	while ((Get-Date) -lt $deadline) {
		$cts = New-Object Threading.CancellationTokenSource 1000
		try {
			$received = $socket.ReceiveAsync([ArraySegment[byte]]$buffer, $cts.Token).GetAwaiter().GetResult()
			if ($received.MessageType -eq "Close") { return "the server closed it ($($socket.CloseStatus))" }
		} catch [OperationCanceledException] {
			continue   # nothing to read for a second: still open
		} catch {
			return "the connection was dropped by the server"
		}
	}
	"still open"
}

function Rows($session) { @((Run-Script $session $snapshot).rows) }
function Texts($session) { (Rows $session | ForEach-Object { $_.text }) -join "|" }
function Add-Item($session, $text) {
	Type-Into-Shadow $session "#todo-input" "input" $text
	Click-Element $session "#todo-add"
}
function Row-Selector($position, $part) { "#todo-list > moth-panel:nth-child($position) $part" }

try {
	Write-Host "the server only lets its own pages (or none, for non-browsers) open a socket"
	$raw = Open-RawSocket $null
	Expect ($raw.State -eq "Open") "a client with no Origin header is accepted ($($raw.State))"
	$raw.Dispose()
	$raw = Open-RawSocket "http://127.0.0.1:$script:backEndPort"
	Expect ($raw.State -eq "Open") "the server's own origin is accepted ($($raw.State))"
	$raw.Dispose()
	$raw = Open-RawSocket "https://evil.example"
	Expect ($raw.State -ne "Open") "a foreign origin is refused ($($raw.State))"
	$raw.Dispose()
	$raw = Open-RawSocket "http://127.0.0.1:8080"
	Expect ($raw.State -ne "Open") "another loopback page is refused without --dev ($($raw.State))"
	$raw.Dispose()

	$raw = Open-RawSocket $null
	$huge = [Text.Encoding]::UTF8.GetBytes('{"type":"add","text":"' + ("x" * 100000) + '"}')
	try { $raw.SendAsync([ArraySegment[byte]]$huge, "Text", $true, [Threading.CancellationToken]::None).Wait(5000) | Out-Null } catch { }
	$outcome = Wait-ForClose $raw
	Expect ($outcome -ne "still open") "a 100 KB message ends the connection instead of being buffered ($outcome)"
	$raw.Dispose()

	$alice = New-Browser
	$bob = New-Browser
	Open-Page $alice "http://127.0.0.1:2000/todo/"
	Open-Page $bob "http://127.0.0.1:2000/todo/"
	Wait-For "both connected" { (Run-Script $alice $snapshot).status -eq "connected" -and (Run-Script $bob $snapshot).status -eq "connected" } | Out-Null

	Write-Host "start"
	foreach ($name in "alice", "bob") {
		$session = if ($name -eq "alice") { $alice } else { $bob }
		$s = Run-Script $session $snapshot
		Expect ($s.status -eq "connected") "$name is connected ('$($s.status)')"
		Expect ($s.summary -eq "0 of 0 done" -and (Rows $session).Count -eq 0) "$name starts with an empty list ('$($s.summary)')"
	}

	Write-Host "alice adds an item by typing and clicking"
	Add-Item $alice "buy milk"
	Expect (Wait-For "alice row" { (Texts $alice) -eq "buy milk" }) "alice sees her row"
	Expect (Wait-For "bob row" { (Texts $bob) -eq "buy milk" }) "bob's page grew the same row without any action of his own"
	Expect ((Run-Script $alice $snapshot).input -eq "") "alice's input was cleared after adding"

	Write-Host "bob adds one: both lists keep the same order"
	Add-Item $bob "walk dog"
	Expect (Wait-For "two rows" { (Texts $alice) -eq "buy milk|walk dog" -and (Texts $bob) -eq "buy milk|walk dog" }) "both show 'buy milk|walk dog' (alice: '$(Texts $alice)', bob: '$(Texts $bob)')"

	Write-Host "bob toggles the first row"
	Click-Element $bob (Row-Selector 1 "moth-toggle-button")
	Expect (Wait-For "alice sees done" { (Rows $alice)[0].done -and -not (Rows $alice)[1].done }) "alice's first row became done, the second did not"
	Expect (Wait-For "summary" { (Run-Script $alice $snapshot).summary -eq "1 of 2 done" -and (Run-Script $bob $snapshot).summary -eq "1 of 2 done" }) "both summaries read '1 of 2 done'"

	Write-Host "the canvas shows half the bar filled"
	Start-Sleep -Milliseconds 500
	foreach ($name in "alice", "bob") {
		$session = if ($name -eq "alice") { $alice } else { $bob }
		$p = Run-Script $session $pixels
		Expect ($p.width -gt 0) "$name's canvas has a backing store ($($p.width) px wide)"
		Expect ($p.left[0] -eq 46 -and $p.left[1] -eq 160 -and $p.left[2] -eq 67) "$name's canvas is green at 25% (rgb $($p.left[0..2] -join ','))"
		Expect ($p.right[3] -lt 255 -and $p.right[3] -gt 0) "$name's canvas is the translucent track at 75% (alpha $($p.right[3]))"
	}

	Write-Host "alice removes the first row"
	Click-Element $alice (Row-Selector 1 "moth-push-button")
	Expect (Wait-For "removed everywhere" { (Texts $alice) -eq "walk dog" -and (Texts $bob) -eq "walk dog" }) "both lists are down to 'walk dog'"
	Expect (Wait-For "summary 0 of 1" { (Run-Script $bob $snapshot).summary -eq "0 of 1 done" }) "bob's summary follows ('$((Run-Script $bob $snapshot).summary)')"

	Write-Host "the server goes away and comes back"
	$restartLog = "$serverLog.restart"
	Stop-Process -Id $server.Id -Force
	Expect (Wait-For "alice notices" { (Run-Script $alice $snapshot).status -like "disconnected*" }) "alice's page says the server is gone ('$((Run-Script $alice $snapshot).status)')"
	Expect (Wait-For "bob notices" { (Run-Script $bob $snapshot).status -like "disconnected*" }) "so does bob's"
	Expect ((Texts $bob) -eq "walk dog") "a page keeps showing its last model while the server is away ('$(Texts $bob)')"
	Expect ((Run-Script $bob $snapshot).status -match "retrying in \d\.\d s") "and says when it will try again ('$((Run-Script $bob $snapshot).status)')"
	$server = Start-BackEnd @("--port", "$script:backEndPort", "--dir", "samples\front-end\dist", "--todo-dir", "samples\todo\dist") $restartLog
	Expect (Wait-For "reconnected" { (Run-Script $alice $snapshot).status -eq "connected" -and (Run-Script $bob $snapshot).status -eq "connected" } 30) "both pages reconnected by themselves ('$((Run-Script $alice $snapshot).status)', '$((Run-Script $bob $snapshot).status)')"
	# the restarted server starts from an empty model: the first snapshot repairs the stale pages
	Expect (Wait-For "repaired" { (Rows $alice).Count -eq 0 -and (Rows $bob).Count -eq 0 }) "the new server's snapshot replaced the stale list on both pages ('$(Texts $alice)' / '$(Texts $bob)')"
	Add-Item $alice "walk dog"
	Expect (Wait-For "after the restart" { (Texts $alice) -eq "walk dog" -and (Texts $bob) -eq "walk dog" }) "actions work again, and reach the other page, after the restart"

	Write-Host "clear done"
	Click-Element $alice (Row-Selector 1 "moth-toggle-button")
	Expect (Wait-For "done" { (Rows $bob)[0].done }) "bob sees the remaining row done"
	Click-Element $bob "#todo-clear"
	Expect (Wait-For "cleared" { (Rows $alice).Count -eq 0 -and (Rows $bob).Count -eq 0 }) "clearing done items empties both lists"

	Write-Host "a reloaded page catches up"
	Add-Item $alice "survives reload"
	Wait-For "bob row" { (Texts $bob) -eq "survives reload" } | Out-Null
	Open-Page $bob "http://127.0.0.1:2000/todo/"
	Expect (Wait-For "reloaded" { (Texts $bob) -eq "survives reload" }) "bob's reloaded page shows the current model at once ('$(Texts $bob)')"

	Write-Host "alice leaves"
	Close-Browser $alice
	$alice = $null
	Add-Item $bob "still alive"
	Expect (Wait-For "bob continues" { (Texts $bob) -eq "survives reload|still alive" }) "bob keeps working after alice's page went away"
}
catch {
	# an error inside the run would otherwise stop it silently and let the script go on to say "passed"
	Write-Host ("  FAIL the run stopped early: " + ($_ | Out-String))
	$script:failures.Add("the run stopped early: $_")
}
finally {
	Close-Browser $alice
	Close-Browser $bob
	Stop-Process -Id $driver.Id -Force -ErrorAction SilentlyContinue
	Start-Sleep -Milliseconds 500
	Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
}

Write-Host "server log"
$log = (Read-BackEndLog $serverLog) + "`n" + (Read-BackEndLog $restartLog)
Write-Host ($log -replace "(?m)^", "  | ")
Expect (([regex]::Matches($log, "todo client connected")).Count -ge 6) "server saw every accepted connection (3 raw, alice, bob, bob reloaded)"
Expect ($log -match 'WebSocket refused, origin Some\("https://evil.example"\)') "server logged the refused foreign origin, escaped"
# bob's first page is torn down by `pagehide` when he reloads: that close is the page's own, so clean
Expect ($log -match "todo client disconnected \(clean\)") "a page torn down by pagehide closed its socket cleanly (the framework's own teardown path)"
Expect ($log -notmatch "ill-formed") "server received no ill-formed action"

if ($script:failures.Count -gt 0) { Write-Host "`n$($script:failures.Count) check(s) failed"; exit 1 }
Write-Host "`nall checks passed"
