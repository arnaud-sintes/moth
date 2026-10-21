# End-to-end check of the 3D viewer: the server renders and streams, the page shows it.
#
#  1. the stream protocol with a raw client: previews then a final frame, the newest state wins, frames
#     in flight are limited by acknowledgements, hostile input is bounded, other origins and oversized
#     messages are refused, the viewer count follows connections;
#  2. one scene for everybody: changes made through one connection reach the others, changes to
#     different things merge, the server turns the camera for everyone, a late viewer finds the scene
#     as it stands;
#  3. the page, in headless Edge, driven with real pointer drags, wheel turns and clicks: what the user
#     does changes the picture, the HUD reports the stream, and a second browser shares the scene (its
#     controls and its picture follow the first's, and the other way round) while keeping its own
#     resolution; a killed and restarted server is recovered from, the scene back at its defaults.
#
# Needs: trunk, and msedgedriver matching your Edge (on PATH or in $env:MSEDGEDRIVER).
# Usage: .\scripts\e2e-viewer.ps1 [-NoBuild] [-Quick] [-Screenshots <folder>]
#   -Quick  leaves out three parts that stand apart from the rest (viewers with alike screens, the
#           second and third browsers and windows of other shapes, the killed and restarted server):
#           the protocol, the shared scene and everything one browser can do. About half the time.

param([switch]$NoBuild, [switch]$Quick, [string]$Screenshots)

$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
. "$PSScriptRoot\webdriver.ps1"
. "$PSScriptRoot\viewer-client.ps1"
Find-EdgeDriver | Out-Null

if (-not $NoBuild) {
	Push-Location samples\viewer
	trunk build
	$code = $LASTEXITCODE
	Pop-Location
	if ($code -ne 0) { exit $code }
}
cargo build -p moth-sample-back-end
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if ($Screenshots) { New-Item -ItemType Directory -Force $Screenshots | Out-Null }

$serverLog = Join-Path $env:TEMP "moth-e2e-viewer-server.log"
$restartLog = "$serverLog.restart"
$serverArguments = @("--port", "$script:backEndPort", "--viewer-dir", "samples\viewer\dist")
$server = Start-BackEnd $serverArguments $serverLog
$driver = Start-EdgeDriver
$page = $null
$other = $null
$twin = $null

$script:runStart = Get-Date
# A section heading, with how long into the run it starts: where the time goes.
function Heading($text) { Write-Host ("{0}   [{1:N0} s]" -f $text, ((Get-Date) - $script:runStart).TotalSeconds) }

# ---- what the script reads from the page

$hudJs = 'return Object.fromEntries([...document.querySelectorAll("#hud .stat")].map(s => [s.querySelector(".stat-name").textContent, s.querySelector(".stat-value").textContent]));'
# the picture on the canvas, sampled every 4th pixel: how much of it is object, a checksum to tell two
# pictures apart, how many pixels are the wireframe's cyan, the object's average colour
$pictureJs = @'
const canvas = document.getElementById("viewport").shadowRoot.querySelector("canvas");
const { width: w, height: h } = canvas;
const data = canvas.getContext("2d").getImageData(0, 0, w, h).data;
let n = 0, cover = 0, cyan = 0, checksum = 0, red = 0, green = 0, blue = 0;
for (let i = 0; i < data.length; i += 16) {
	const r = data[i], g = data[i + 1], b = data[i + 2];
	n++;
	checksum = (checksum * 31 + r + g * 3 + b * 7) % 1000000007;
	if (r + g + b > 220) { cover++; red += r; green += g; blue += b; }
	if (g > 200 && b > 200 && r < g - 30) cyan++;
}
return { w, h, cover: cover / n, checksum, cyan, red: cover ? red / cover : 0, green: cover ? green / cover : 0, blue: cover ? blue / cover : 0 };
'@
$statusJs = 'const s = document.getElementById("status"); return { text: s.textContent, state: s.getAttribute("data-state"), viewers: document.getElementById("viewers").textContent };'
$valueJs = 'return document.getElementById(arguments[0]).parentElement.querySelector(".row-value").textContent;'

# The canvas, and the box around what is not dark backdrop on it (the backdrop is a dark gradient, the
# object is lit): a sphere seen in a picture of the right shape is as wide as it is tall.
$shapeJs = @'
const canvas = document.getElementById("viewport").shadowRoot.querySelector("canvas");
const { width: w, height: h } = canvas;
const data = canvas.getContext("2d").getImageData(0, 0, w, h).data;
let minX = w, maxX = -1, minY = h, maxY = -1;
for (let y = 0; y < h; y += 2) for (let x = 0; x < w; x += 2) {
	const i = (y * w + x) * 4;
	if (data[i] + data[i + 1] + data[i + 2] > 200) {
		if (x < minX) minX = x; if (x > maxX) maxX = x; if (y < minY) minY = y; if (y > maxY) maxY = y;
	}
}
return { w, h, boxWidth: maxX - minX + 1, boxHeight: maxY - minY + 1 };
'@

function Hud($session) { Run-Script $session $hudJs }
function Picture($session) { Run-Script $session $pictureJs }
function Status($session) { Run-Script $session $statusJs }
function Value($session, $id) { Wd POST "/session/$session/execute/sync" @{ args = @($id); script = $valueJs } }
function Shot($session, $name) { if ($Screenshots) { Save-Screenshot $session (Join-Path $Screenshots "$name.png") } }

# The picture has stopped changing and the refined frame is the one on screen.
function Wait-Settled($session) {
	for ($i = 0; $i -lt 20; $i++) {
		$before = Picture $session
		Start-Sleep -Milliseconds 350
		if ((Picture $session).checksum -eq $before.checksum -and (Hud $session).frame -eq "refined") { return $true }
	}
	$false
}

# A real click at `$fraction` (0 to 1) along a slider's track. (Keys are not used: headless Chromium driven by
# WebDriver does not apply Home or the arrow keys to range inputs at all, which was checked with a plain
# <input type="range"> that has nothing of moth in it.)
function Click-Slider($session, $sliderId, $fraction) {
	$box = Run-Script $session ('const r = document.getElementById("' + $sliderId + '").shadowRoot.querySelector("input").getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height };')
	Invoke-Pointer $session @((Move-To ($box.x + $box.w * $fraction) ($box.y + $box.h / 2)), $script:Down, $script:Up)
}

# A real click at the position where a slider (`$min` to `$max`) has `$value`: the browser keeps the
# thumb's centre half a thumb (8 px) inside the track at either end, and a click on the thumb itself
# moves nothing.
function Click-SliderTo($session, $sliderId, $value, $min, $max) {
	$box = Run-Script $session ('const r = document.getElementById("' + $sliderId + '").shadowRoot.querySelector("input").getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height };')
	$x = $box.x + 8 + ($value - $min) / ($max - $min) * ($box.w - 16)
	Invoke-Pointer $session @((Move-To $x ($box.y + $box.h / 2)), $script:Down, $script:Up)
}

function Number-In($text) { [int]($text -replace '[^\d-]', '') }

function Choose-Option($session, $menuId, $position) {
	Click-Id $session (Find-InShadow $session "#$menuId" "option:nth-child($position)")
}

# What a menu shows as chosen, and whether a toggle shows as pressed.
function Chosen($session, $menuId) {
	Wd POST "/session/$session/execute/sync" @{ args = @($menuId); script = 'const s = document.getElementById(arguments[0]).shadowRoot.querySelector("select"); return s.options[s.selectedIndex] ? s.options[s.selectedIndex].textContent : null;' }
}
function Pressed($session, $toggleId) {
	Wd POST "/session/$session/execute/sync" @{ args = @($toggleId); script = 'return document.getElementById(arguments[0]).shadowRoot.querySelector("button").getAttribute("aria-pressed");' }
}

try {
	# =====================================================================================
	Heading "the stream protocol, with a raw client"
	$v = Open-Viewer
	$hub = Receive-Message $v 3000
	$snapshot = [Text.Encoding]::UTF8.GetString($hub.Bytes) | ConvertFrom-Json
	Expect ($hub.Type -eq "Text" -and $snapshot.models.Count -eq 4 -and $snapshot.viewers -ge 1) "a new viewer is sent the hub: the models on offer and how many are watching ($($snapshot.viewers))"
	Expect ($snapshot.scene.model -eq "torus_knot" -and $snapshot.scene.material -eq "bronze" -and $snapshot.scene.spin -eq $false -and $snapshot.scene.author -eq 0) "with the scene as it stands (the defaults, on a fresh server)"

	Send-Display $v 640 480
	$frames = Receive-Frames $v 1500
	Expect ($frames.Count -eq 2 -and $frames[0].Kind -eq "preview" -and $frames[1].Kind -eq "final") "telling the server the frame size gets a preview, then one final frame (got $(($frames | ForEach-Object { $_.Kind }) -join ', '))"
	Expect ($frames[0].Magic -eq "MOTH" -and $frames[0].Jpeg -and $frames[1].Jpeg) "each is the header plus a JPEG"
	Expect ($frames[0].Width -eq 320 -and $frames[0].Height -eq 240 -and $frames[1].Width -eq 640 -and $frames[1].Height -eq 480) "the preview is half size, the final frame full size ($($frames[0].Width)x$($frames[0].Height), $($frames[1].Width)x$($frames[1].Height))"
	Expect ($frames[1].Length -gt $frames[0].Length) "the final frame carries more detail than the preview ($($frames[0].Length) -> $($frames[1].Length) bytes)"
	Expect (($frames | Where-Object { $_.ViewId -ne 0 }).Count -eq 0) "nobody here changed the scene, so the frames name no change of ours (view 0)"

	$coarse = Open-Viewer
	Receive-Message $coarse 3000 | Out-Null
	Send-Display $coarse 640 480 1.0 0.25
	$coarseFrames = Receive-Frames $coarse 1500
	Expect ($coarseFrames.Count -eq 2 -and $coarseFrames[0].Width -eq 160 -and $coarseFrames[0].Height -eq 120 -and $coarseFrames[1].Width -eq 640) "a viewer chooses how big its previews are (a quarter of the full frame: $($coarseFrames[0].Width)x$($coarseFrames[0].Height), then $($coarseFrames[1].Width)x$($coarseFrames[1].Height))"
	Send-Display $coarse 640 480 1.0 1.0
	$sharpFrames = Receive-Frames $coarse 1500
	Expect ($sharpFrames.Count -eq 2 -and $sharpFrames[0].Width -eq 640 -and $sharpFrames[0].Kind -eq "preview") "up to a preview as big as the full frame ($($sharpFrames[0].Width)x$($sharpFrames[0].Height))"
	Send-Display $coarse 640 480 1.0 1e30
	Send-Display $coarse 640 480 1.0 0
	$boundedFrames = Receive-Frames $coarse 1500
	Expect (($boundedFrames | Where-Object { $_.Kind -eq "preview" } | Select-Object -Last 1).Width -eq 64) "and a hostile preview setting is bounded (0 gives the smallest, a tenth: $((($boundedFrames | Where-Object { $_.Kind -eq 'preview' } | Select-Object -Last 1).Width)) px wide)"
	$coarse.Socket.Dispose()

	Send-Set $v 7 1 @{ yaw = 1.0 }
	$own = Receive-Frames $v 1500
	Expect ($own.Count -eq 2 -and ($own | Where-Object { $_.ViewId -ne 1 }).Count -eq 0) "a change of ours gets frames that name it, to time the round trip (view $(($own | ForEach-Object { $_.ViewId }) -join ', '))"

	1..60 | ForEach-Object { Send-Set $v 7 (100 + $_) @{ yaw = (1.0 + $_ * 0.05) } }
	$burst = Receive-Frames $v 1500
	Expect ($burst.Count -le 8) "60 changes sent at once are not all rendered: the newest wins ($($burst.Count) frames)"
	Expect (($burst | Select-Object -Last 1).ViewId -eq 160 -and ($burst | Select-Object -Last 1).Kind -eq "final") "and the last frame is the refined newest state (view $(($burst | Select-Object -Last 1).ViewId), $(($burst | Select-Object -Last 1).Kind))"

	Send-Display $v 320 240
	Receive-Frames $v 1000 | Out-Null
	Send-Set $v 7 500 @{ yaw = 1.0 }
	$unacknowledged = Receive-Frames $v 1000 $false
	Send-Set $v 7 501 @{ yaw = 2.0 }
	$stuck = Receive-Frames $v 1000 $false
	Expect ($unacknowledged.Count -eq 2 -and $stuck.Count -eq 0) "without acknowledgements the server stops after 2 frames in flight ($($unacknowledged.Count), then $($stuck.Count))"
	Send-Json $v @{ type = "ack"; seq = 0 }
	Send-Json $v @{ type = "ack"; seq = 1 }
	$resumed = @(Receive-Frames $v 1500)
	Expect ($resumed.Count -eq 1 -and $resumed[0].Kind -eq "final" -and $resumed[0].ViewId -eq 501) "acknowledging resumes the stream with the newest state, and as it has rested, as one full frame (view $(($resumed | Select-Object -Last 1).ViewId), $(($resumed | ForEach-Object { $_.Kind }) -join ', '))"
	1..200 | ForEach-Object { Send-Json $v @{ type = "ack"; seq = $_ } }
	Send-Set $v 7 600 @{ yaw = 3.0 }
	$first = Receive-Frames $v 1000 $false
	Send-Set $v 7 601 @{ yaw = 4.0 }
	$second = Receive-Frames $v 1000 $false
	Expect ($first.Count -eq 2 -and $second.Count -eq 0) "200 acknowledgements nobody earned do not raise the limit ($($first.Count), then $($second.Count))"
	$v.Socket.Dispose()

	$v = Open-Viewer
	Receive-Message $v 3000 | Out-Null
	Send-Display $v 4294967295 4294967295 1e39
	Send-Set $v 8 1 @{ yaw = 1e30; pitch = 1e30; distance = -1e30; shininess = 1e39; light = -1e30 }
	$hostile = Receive-Frames $v 3000
	Expect ($hostile.Count -ge 1 -and ($hostile | Where-Object { $_.Width -gt 1280 -or $_.Height -gt 720 }).Count -eq 0) "absurd sizes and scales are bounded before rendering (largest frame $(($hostile | Measure-Object Width -Maximum).Maximum)x$(($hostile | Measure-Object Height -Maximum).Maximum))"
	$bounded = Latest-Scene $v
	Expect ($bounded.pitch -le 1.5 -and $bounded.distance -ge 2.6 -and $bounded.distance -le 14 -and $bounded.light -ge -3.15 -and $bounded.yaw -ge 0 -and $bounded.yaw -le 6.3 -and $bounded.shininess -le 200) "and so is the shared scene itself (pitch $($bounded.pitch), distance $($bounded.distance), light $([math]::Round($bounded.light, 2)))"
	Send-Display $v 320 240
	Receive-Frames $v 1500 | Out-Null
	Send-Raw $v ([Text.Encoding]::UTF8.GetBytes("{nope")) "Text"
	Send-Json $v @{ type = "teleport" }
	Send-Raw $v ([byte[]](0xFF, 0xFE, 0xFD)) "Binary"
	Send-Set $v 8 10 @{ yaw = 2.5 }
	$after = Receive-Frames $v 1500
	Expect ($after.Count -eq 2 -and ($after | Select-Object -Last 1).ViewId -eq 10) "malformed messages are ignored and the same connection keeps working"
	Reset-Scene $v
	Receive-Frames $v 1000 | Out-Null
	$v.Socket.Dispose()

	$a = Open-Viewer
	Receive-Message $a 3000 | Out-Null
	$count = { param($viewer, $expected) $deadline = (Get-Date).AddSeconds(4); while ((Get-Date) -lt $deadline) { $m = Receive-Message $viewer 500; if ($m -and $m.Type -eq "Text") { $hubNow = [Text.Encoding]::UTF8.GetString($m.Bytes) | ConvertFrom-Json; if ($hubNow.viewers -eq $expected) { return $true } } }; $false }
	$b = Open-Viewer
	Expect (& $count $a 2) "a second viewer joining is announced to the first (2 viewers)"
	$b.Socket.Dispose()
	Expect (& $count $a 1) "and leaving is announced too (1 viewer)"
	$a.Socket.Dispose()

	$foreign = Open-Viewer 2000 "https://evil.example"
	Expect ($foreign.Socket.State -ne "Open") "a page of another origin cannot open the viewer ($($foreign.Socket.State))"
	$foreign.Socket.Dispose()
	$big = Open-Viewer
	# the server cuts the connection while it is still being sent: the send may fail, which is the point
	try { Send-Raw $big ([Text.Encoding]::UTF8.GetBytes('{"type":"set","client":1,"pad":"' + ("x" * 100000) + '"}')) "Text" } catch { }
	for ($i = 0; $i -lt 20 -and (Receive-Message $big 300); $i++) { }
	Expect (Wait-For "closed" { $big.Socket.State -ne "Open" } 3) "a 100 KB message ends the connection ($($big.Socket.State))"
	$big.Socket.Dispose()

	# =====================================================================================
	Heading "everyone shares one scene"
	$a = Open-Viewer
	Receive-Message $a 3000 | Out-Null
	Send-Display $a 320 240
	Receive-Frames $a 1200 | Out-Null
	$b = Open-Viewer
	Receive-Message $b 3000 | Out-Null
	Send-Display $b 400 300
	Receive-Frames $b 1200 | Out-Null
	Latest-Scene $a | Out-Null

	Send-Set $a 1 1 @{ material = "ruby" }
	$seenByB = Watch-Viewer $b 1.5
	$seenByA = Watch-Viewer $a 0.8
	$sceneB = $seenByB.Scenes | Select-Object -Last 1
	Expect ($sceneB.material -eq "ruby" -and $sceneB.author -eq 1 -and $sceneB.seq -eq 1) "a change made through one connection is announced to the other, with who made it ($($sceneB.material), author $($sceneB.author))"
	Expect ($seenByB.Frames.Count -ge 2 -and ($seenByB.Frames | Where-Object { $_.ViewId -ne 0 }).Count -eq 0) "the other viewer is streamed the new picture, at its own size ($($seenByB.Frames[0].Width)x$($seenByB.Frames[0].Height) here), naming no change of its own"
	Expect ($seenByA.Frames.Count -eq 0 -or ($seenByA.Frames | Where-Object { $_.ViewId -ne 1 }).Count -eq 0) "and the one who asked is streamed it too, its own change named"

	Send-Set $b 2 1 @{ light = 1.5 }
	Send-Set $a 1 2 @{ pitch = 0.1 }
	$sa = Latest-Scene $a 600
	$sb = Latest-Scene $b 600
	Expect ($sa.material -eq "ruby" -and [math]::Abs($sa.light - 1.5) -lt 0.001 -and [math]::Abs($sa.pitch - 0.1) -lt 0.001) "changes to different things made at once are both kept (material $($sa.material), light $($sa.light), pitch $($sa.pitch))"
	Expect (($sa | ConvertTo-Json -Compress) -eq ($sb | ConvertTo-Json -Compress)) "and both viewers hold exactly the same scene"

	Send-Set $a 1 3 @{ yaw = 1.0 }
	Send-Set $b 2 2 @{ yaw = 2.0 }
	$sa = Latest-Scene $a 600
	$sb = Latest-Scene $b 600
	Expect ([math]::Abs($sa.yaw - 2.0) -lt 0.001 -and [math]::Abs($sb.yaw - 2.0) -lt 0.001 -and $sa.author -eq 2) "for the same thing the later change wins, for everyone (yaw $($sa.yaw) and $($sb.yaw), author $($sa.author))"

	Send-Set $b 2 3 @{ yaw = 2.0; material = "ruby" }
	$quiet = Watch-Viewer $a 0.8
	Expect ($quiet.Scenes.Count -eq 0 -and $quiet.Frames.Count -eq 0) "a change that changes nothing is neither announced nor rendered again ($($quiet.Scenes.Count) announcements, $($quiet.Frames.Count) frames)"

	$c = Open-Viewer
	$late = [Text.Encoding]::UTF8.GetString((Receive-Message $c 3000).Bytes) | ConvertFrom-Json
	Expect ($late.scene.material -eq "ruby" -and [math]::Abs($late.scene.yaw - 2.0) -lt 0.001 -and [math]::Abs($late.scene.light - 1.5) -lt 0.001) "a viewer that arrives later is sent the scene as it stands, in its first message"
	$c.Socket.Dispose()

	Send-Set $a 1 4 @{ spin = $true }
	$turning = Watch-Viewer $b 1.5
	$yaws = $turning.Scenes | ForEach-Object { $_.yaw } | Sort-Object -Unique
	Expect ($yaws.Count -ge 8 -and ($turning.Scenes | Select-Object -Last 1).author -eq 0) "with the turntable on the server turns the camera for everyone, and says it did ($($yaws.Count) positions in 1.5 s)"
	Expect ($turning.Frames.Count -ge 8) "and streams each viewer the turning object ($($turning.Frames.Count) frames in 1.5 s)"
	Send-Set $b 2 4 @{ spin = $false }
	Latest-Scene $a 600 | Out-Null
	$still = Watch-Viewer $a 0.8
	Expect ($still.Scenes.Count -eq 0 -and (Latest-Scene $a 200).spin -eq $false) "and the other viewer can switch it off, and it stops"
	$a.Socket.Dispose()
	$b.Socket.Dispose()

	$d = Open-Viewer
	$left = [Text.Encoding]::UTF8.GetString((Receive-Message $d 3000).Bytes) | ConvertFrom-Json
	Expect ($left.scene.material -eq "ruby" -and $left.viewers -eq 1) "the scene outlives its viewers: the next one finds it as it was left"
	Reset-Scene $d
	Expect ((Latest-Scene $d 600).material -eq "bronze") "(put back to the defaults for the page)"
	$d.Socket.Dispose()

	if (-not $Quick) {
		# =====================================================================================
		Heading "viewers with alike screens share one rendering"
		$same1 = Open-Viewer
		$same2 = Open-Viewer
		$half = Open-Viewer
		$tall = Open-Viewer
		$lowQuality = Open-Viewer
		$highQuality = Open-Viewer
		foreach ($client in @($same1, $same2, $half, $tall, $lowQuality, $highQuality)) { Receive-Message $client 3000 | Out-Null }
		Send-Display $same1 640 400
		Send-Display $same2 640 400
		Send-Display $half 320 200
		Send-Display $tall 300 500
		Send-Display $lowQuality 640 400 1.0 0.5 20 20
		Send-Display $highQuality 640 400 1.0 0.5 100 100
		# (their first frames are rendered as each viewer arrives: it is the next change of the scene, which
		# all of them want a frame of at the same moment, that one rendering can serve)
		foreach ($client in @($same1, $same2, $half, $tall, $lowQuality, $highQuality)) { Receive-Frames $client 1200 | Out-Null }
		Send-Set $same1 77 1 @{ yaw = 2.0 }
		$framesSame1 = Receive-Frames $same1 1200
		$framesSame2 = Receive-Frames $same2 1200
		$framesHalf = Receive-Frames $half 1200
		$framesTall = Receive-Frames $tall 1200
		$framesLow = Receive-Frames $lowQuality 1200
		$framesHigh = Receive-Frames $highQuality 1200
		$final1 = $framesSame1 | Select-Object -Last 1
		$final2 = $framesSame2 | Select-Object -Last 1
		$finalHalf = $framesHalf | Select-Object -Last 1
		$finalTall = $framesTall | Select-Object -Last 1
		Expect ($framesSame1.Count -eq 2 -and $framesSame2.Count -eq 2 -and $framesHalf.Count -eq 2 -and $framesTall.Count -eq 2) "each of four viewers is sent a preview and a full frame"
		Expect ($final1.Width -eq 640 -and $final1.Height -eq 400 -and $finalHalf.Width -eq 320 -and $finalHalf.Height -eq 200 -and $finalTall.Width -eq 300 -and $finalTall.Height -eq 500) "each at the size it asked for ($($final1.Width)x$($final1.Height), $($finalHalf.Width)x$($finalHalf.Height), $($finalTall.Width)x$($finalTall.Height))"
		Expect ($final1.RenderMs -eq $final2.RenderMs -and $final1.EncodeMs -eq $final2.EncodeMs -and $final1.Length -eq $final2.Length -and $framesSame1[0].Length -eq $framesSame2[0].Length -and $framesSame1[0].RenderMs -eq $framesSame2[0].RenderMs) "two viewers with the same screen are sent the very same frames (rendered in $($final1.RenderMs) ms, $($final1.Length) bytes)"
		Expect ($finalHalf.RenderMs -eq $final1.RenderMs -and $framesHalf[0].RenderMs -eq $framesSame1[0].RenderMs) "a smaller viewer of the same shape is served from that same rendering, in the previews too ($($finalHalf.RenderMs) ms)"
		Expect ($finalHalf.EncodeMs -ne $final1.EncodeMs -and $finalHalf.Length -lt $final1.Length * 0.8) "reduced on the server before it was compressed: its own file, $($finalHalf.Length) bytes against $($final1.Length)"
		Expect ($finalTall.RenderMs -ne $final1.RenderMs -and $framesTall[0].RenderMs -ne $framesSame1[0].RenderMs) "a viewer of another shape gets a rendering of its own ($($finalTall.RenderMs) ms against $($final1.RenderMs))"
		$finalLow = $framesLow | Select-Object -Last 1
		$finalHigh = $framesHigh | Select-Object -Last 1
		Expect ($framesLow.Count -eq 2 -and $framesHigh.Count -eq 2 -and $finalLow.RenderMs -eq $final1.RenderMs -and $finalHigh.RenderMs -eq $final1.RenderMs -and $framesLow[0].RenderMs -eq $framesSame1[0].RenderMs) "viewers that want another JPEG quality share the same rendering ($($final1.RenderMs) ms)..."
		Expect ($finalLow.Length -lt $final1.Length -and $final1.Length -lt $finalHigh.Length -and $framesLow[0].Length -lt $framesSame1[0].Length -and $framesSame1[0].Length -lt $framesHigh[0].Length) "...and are compressed on their own: quality 20, 90 and 100 give $($finalLow.Length), $($final1.Length) and $($finalHigh.Length) bytes (previews $($framesLow[0].Length), $($framesSame1[0].Length), $($framesHigh[0].Length))"
		Expect ($finalLow.Width -eq 640 -and $finalHigh.Width -eq 640) "all at the size asked for"
		Reset-Scene $same1 901 1
		Latest-Scene $same1 600 | Out-Null
		foreach ($client in @($same1, $same2, $half, $tall, $lowQuality, $highQuality)) { $client.Socket.Dispose() }

	}

	# =====================================================================================
	Heading "the page"
	$page = New-Browser "1440,900"
	Open-Page $page "http://127.0.0.1:2000/viewer/"
	Expect (Wait-For "live" { (Status $page).state -eq "live" } 10) "the page connects ($((Status $page).text))"
	Expect (Wait-For "first frame" { (Hud $page).frame -eq "refined" } 10) "and shows the first frame"
	Expect (Wait-Settled $page) "the picture settles"
	$home_ = Picture $page
	Shot $page "01-first-frame"
	Expect ($home_.cover -gt 0.05 -and $home_.cover -lt 0.6) "an object is on screen, in the middle of the stage ($([math]::Round($home_.cover * 100)) % of it)"
	Expect ($home_.w -gt 600 -and $home_.h -gt 400) "the canvas has the size of the stage ($($home_.w)x$($home_.h))"
	$hud = Hud $page
	Expect ($hud.size -match "^\d+\D+\d+" -and $hud."server render" -match "\d+\.\d ms" -and $hud."server encode" -match "\d+\.\d ms") "the HUD reports the frame and what the server spent on it (size '$($hud.size)', render $($hud.'server render'), encode $($hud.'server encode'))"
	Expect ((Status $page).viewers -eq "1 viewer") "the top bar counts the viewers ($((Status $page).viewers))"
	Expect ((Value $page "light-slider") -eq "40$([char]0xb0)" -and (Value $page "shininess-slider") -eq "48" -and (Value $page "resolution-slider") -eq "100%") "the controls start at the server's defaults"

	Heading "a real drag turns the object"
	Invoke-Pointer $page @((Move-To 500 330), $script:Down)
	$sawPreview = $false
	$best = 0
	for ($i = 1; $i -le 16; $i++) {
		Invoke-Pointer $page @((Move-To (500 + $i * 22) (330 - $i * 4) 50))
		$now = Hud $page
		if ($now.frame -eq "preview") { $sawPreview = $true }
		$best = [math]::Max($best, [int]$now."frames/s")
	}
	Shot $page "02-dragging"
	Invoke-Pointer $page @($script:Up)
	Expect $sawPreview "while the camera moves the frames are previews"
	Expect ($best -ge 4) "and they arrive at an interactive rate ($best frames/s)"
	Expect ((Hud $page).latency -match "\d+ ms") "and the HUD times the round trip of the camera moves ($((Hud $page).latency))"
	Expect (Wait-Settled $page) "when the pointer is released a refined frame follows"
	$turned = Picture $page
	Shot $page "03-turned"
	Expect ($turned.checksum -ne $home_.checksum) "the picture is the object seen from elsewhere"

	Click-Element $page "#reset-button"
	Expect (Wait-Settled $page) "reset camera: settles"
	Expect ((Picture $page).checksum -eq $home_.checksum) "and the picture is exactly the first one again"

	Invoke-Pointer $page @((Move-To 500 330), $script:Down, (Move-To 650 300 150), $script:Up)
	Wait-Settled $page | Out-Null
	Expect ((Picture $page).checksum -ne $home_.checksum) "dragged away again"
	Invoke-Pointer $page @((Move-To 700 400), $script:Down, $script:Up, $script:Down, $script:Up)
	Expect (Wait-Settled $page) "a double click: settles"
	Expect ((Picture $page).checksum -eq $home_.checksum) "and brings the camera home"

	Heading "the wheel zooms"
	Scroll-Wheel $page 500 330 -600
	Wait-Settled $page | Out-Null
	$closer = Picture $page
	Shot $page "04-zoomed-in"
	Expect ($closer.cover -gt $home_.cover * 1.3) "turning the wheel towards you brings the object closer ($([math]::Round($home_.cover * 100)) % -> $([math]::Round($closer.cover * 100)) % of the stage)"
	Scroll-Wheel $page 500 330 1400
	Wait-Settled $page | Out-Null
	Expect ((Picture $page).cover -lt $home_.cover * 0.75) "and away from you, farther ($([math]::Round((Picture $page).cover * 100)) %)"
	Click-Element $page "#reset-button"
	Wait-Settled $page | Out-Null

	Heading "the controls"
	Choose-Option $page "model-menu" 2
	Expect (Wait-For "torus" { (Picture $page).checksum -ne $home_.checksum } 5) "choosing another model in the menu changes the picture"
	Wait-Settled $page | Out-Null
	Shot $page "05-torus"
	$torus = Picture $page
	Choose-Option $page "material-menu" 2
	Wait-Settled $page | Out-Null
	$jade = Picture $page
	Expect ($jade.green -gt $jade.red -and $torus.red -ge $torus.green) "choosing Jade makes the object green (red $([math]::Round($torus.red)) -> $([math]::Round($jade.red)), green $([math]::Round($torus.green)) -> $([math]::Round($jade.green)))"
	Shot $page "06-jade"

	Click-Slider $page "light-slider" 0.04
	$angle = Number-In (Value $page "light-slider")
	Expect ($angle -le -140) "clicking the light slider near its left end moves it, and its readout follows ($(Value $page 'light-slider'))"
	Wait-Settled $page | Out-Null
	Expect ((Picture $page).checksum -ne $jade.checksum) "and the lighting on the object changes"
	Click-Slider $page "shininess-slider" 0.92
	Expect ((Number-In (Value $page "shininess-slider")) -ge 150) "the shininess slider, clicked near its right end: 48 -> $(Value $page 'shininess-slider')"

	Wait-Settled $page | Out-Null
	Click-Slider $page "resolution-slider" 0.02
	Expect ((Number-In (Value $page "resolution-slider")) -le 30) "the resolution slider, clicked near its left end, goes down ($(Value $page 'resolution-slider'))"
	Wait-Settled $page | Out-Null
	$small = (Hud $page).size
	Expect ($small -match "^(\d+)" -and [int]$Matches[1] -lt 400) "and the frames shrink with it (frame size '$small')"
	Shot $page "07-low-resolution"
	Click-Slider $page "resolution-slider" 0.99
	Wait-Settled $page | Out-Null
	Expect ((Hud $page).size -match "^(\d+)" -and [int]$Matches[1] -ge 950) "back to nearly full size ('$((Hud $page).size)')"

	# the moving resolution: how big the previews are while the picture is being manipulated
	Expect ((Value $page "preview-slider") -eq "50%") "the moving resolution starts at half the full frame ($(Value $page 'preview-slider'))"
	function Preview-Width($session) {
		Invoke-Pointer $session @((Move-To 500 330), $script:Down)
		$widest = 0
		$narrowest = 99999
		$kilobytes = @()
		for ($i = 1; $i -le 10; $i++) {
			Invoke-Pointer $session @((Move-To (500 + $i * 20) (330 - $i * 3) 50))
			$now = Hud $session
			if ($now.frame -eq "preview" -and $now.size -match "^(\d+)\D+\d+\s+(\d+) kB") { $widest = [math]::Max($widest, [int]$Matches[1]); $narrowest = [math]::Min($narrowest, [int]$Matches[1]); $kilobytes += [int]$Matches[2] }
		}
		Invoke-Pointer $session @($script:Up)
		Wait-Settled $session | Out-Null
		@{ widest = $widest; narrowest = $narrowest; kb = $(if ($kilobytes.Count) { ($kilobytes | Measure-Object -Average).Average } else { 0 }) }
	}
	function Rest-Kilobytes($session) { if ((Hud $session).size -match "\s(\d+) kB") { [int]$Matches[1] } else { -1 } }
	$normal = Preview-Width $page
	Click-Slider $page "preview-slider" 0.02
	Expect ((Number-In (Value $page "preview-slider")) -le 15) "the moving resolution slider, clicked near its left end, goes down ($(Value $page 'preview-slider'))"
	$coarsePreviews = Preview-Width $page
	Expect ($coarsePreviews.widest -le 200 -and $coarsePreviews.widest -lt $normal.narrowest * 0.6) "and the previews while dragging shrink with it (widest $($coarsePreviews.widest) px against $($normal.narrowest) to $($normal.widest) px before)"
	Expect ((Hud $page).size -match "^(\d+)" -and [int]$Matches[1] -ge 950 -and (Hud $page).frame -eq "refined") "while the refined picture is still full size ('$((Hud $page).size)')"
	Click-Slider $page "preview-slider" 0.99
	$sharpPreviews = Preview-Width $page
	Expect ($sharpPreviews.narrowest -ge 950) "at the right end the previews are as big as the full frame (narrowest $($sharpPreviews.narrowest) px)"
	Click-Slider $page "preview-slider" 0.44
	Wait-Settled $page | Out-Null

	# the JPEG quality of the full frame, and of the previews
	Expect ((Value $page "quality-slider") -eq "90" -and (Value $page "preview-quality-slider") -eq "60") "the JPEG qualities start at 90 for the full frame and 60 for the previews"
	$restKb = Rest-Kilobytes $page
	Click-Slider $page "quality-slider" 0.02
	Wait-Settled $page | Out-Null
	$lowKb = Rest-Kilobytes $page
	Expect ((Number-In (Value $page "quality-slider")) -le 15 -and $lowKb -lt $restKb * 0.7) "the JPEG quality slider, clicked near its left end ($(Value $page 'quality-slider')), makes the full frame much lighter ($restKb kB -> $lowKb kB)"
	Shot $page "07b-low-quality"
	Click-Slider $page "quality-slider" 0.99
	Wait-Settled $page | Out-Null
	$highKb = Rest-Kilobytes $page
	Expect ((Number-In (Value $page "quality-slider")) -ge 95 -and $highKb -gt $restKb * 1.2) "and near its right end ($(Value $page 'quality-slider')) heavier ($highKb kB)"
	Click-SliderTo $page "quality-slider" 90 10 100
	Wait-Settled $page | Out-Null
	Expect ((Value $page "quality-slider") -eq "90" -and (Rest-Kilobytes $page) -ge $restKb * 0.9 -and (Rest-Kilobytes $page) -le $restKb * 1.1) "back to 90: the full frame is as it was ($(Rest-Kilobytes $page) kB)"
	Expect ((Hud $page).size -match "^(\d+)" -and [int]$Matches[1] -ge 950) "without any change in the size of the frame ('$((Hud $page).size)')"

	# the full frame, seen from the same place before and after the previews' quality is changed
	Click-Element $page "#reset-button"
	Wait-Settled $page | Out-Null
	$homeKb = Rest-Kilobytes $page
	$movingNormal = Preview-Width $page
	Click-Slider $page "preview-quality-slider" 0.02
	$movingLow = Preview-Width $page
	Expect ((Number-In (Value $page "preview-quality-slider")) -le 15 -and $movingLow.kb -lt $movingNormal.kb * 0.9 -and $movingLow.narrowest -eq $movingNormal.narrowest) "the moving JPEG quality slider, clicked near its left end ($(Value $page 'preview-quality-slider')), makes the previews lighter at the same size ($([math]::Round($movingNormal.kb, 1)) kB -> $([math]::Round($movingLow.kb, 1)) kB, $($movingLow.narrowest) px)"
	Click-Slider $page "preview-quality-slider" 0.99
	$movingHigh = Preview-Width $page
	Expect ((Number-In (Value $page "preview-quality-slider")) -ge 95 -and $movingHigh.kb -gt $movingNormal.kb * 1.2) "and near its right end ($(Value $page 'preview-quality-slider')) heavier ($([math]::Round($movingHigh.kb, 1)) kB)"
	Click-Element $page "#reset-button"
	Wait-Settled $page | Out-Null
	Expect ((Rest-Kilobytes $page) -ge $homeKb * 0.9 -and (Rest-Kilobytes $page) -le $homeKb * 1.1) "while the full frame, seen from the same place, is as it was ($homeKb kB, then $(Rest-Kilobytes $page) kB)"
	Click-SliderTo $page "preview-quality-slider" 60 10 100
	Wait-Settled $page | Out-Null
	Expect ((Value $page "preview-quality-slider") -eq "60") "and back to 60"
	Choose-Option $page "model-menu" 3
	Choose-Option $page "material-menu" 1
	Wait-Settled $page | Out-Null
	$plain = Picture $page
	Click-Element $page "#wireframe-toggle"
	Wait-Settled $page | Out-Null
	$wired = Picture $page
	Shot $page "08-wireframe"
	Expect ($wired.cyan -gt $plain.cyan + 150) "the wireframe toggle draws the mesh edges over the object ($($plain.cyan) -> $($wired.cyan) cyan samples)"
	Click-Element $page "#wireframe-toggle"
	Wait-Settled $page | Out-Null
	Expect ((Picture $page).cyan -lt $wired.cyan / 2) "and switching it off removes them"

	Heading "auto-rotate streams continuously"
	Click-Element $page "#spin-toggle"
	Start-Sleep -Milliseconds 1500
	$spinning = Hud $page
	$one = Picture $page
	Start-Sleep -Milliseconds 300
	$two = Picture $page
	Expect ([int]$spinning."frames/s" -ge 6) "the turntable keeps frames coming ($($spinning.'frames/s') frames/s, $($spinning.stream))"
	Expect ($one.checksum -ne $two.checksum) "and the picture keeps changing"
	Shot $page "09-spinning"
	Click-Element $page "#spin-toggle"
	Expect (Wait-Settled $page) "switching it off, the picture settles on a refined frame"
	Expect (Wait-For "idle" { (Hud $page)."frames/s" -eq "0" } 5) "and the stream goes quiet"

	if (-not $Quick) {
		Heading "a second browser shares the scene"
		Wait-Settled $page | Out-Null
		$pageLight = Value $page "light-slider"
		$pageShininess = Value $page "shininess-slider"
		$pageModel = Chosen $page "model-menu"
		$other = New-Browser "1000,700"
		Open-Page $other "http://127.0.0.1:2000/viewer/"
		Expect (Wait-For "two" { (Status $page).viewers -eq "2 viewers" -and (Status $other).viewers -eq "2 viewers" } 8) "a second browser makes it 2 viewers on both pages"
		Expect (Wait-For "frame" { (Hud $other).frame -eq "refined" } 10) "and it is streamed its own picture, at its own size"
		Expect ((Picture $other).w -lt (Picture $page).w) "(its canvas is $((Picture $other).w) px wide, the first one's $((Picture $page).w))"
		Expect (Wait-For "controls" { (Value $other "light-slider") -eq $pageLight -and (Value $other "shininess-slider") -eq $pageShininess } 6) "it opens on the scene as it stands: light $(Value $other 'light-slider') and shininess $(Value $other 'shininess-slider') like the first page's"
		Expect ((Chosen $other "model-menu") -eq $pageModel -and (Chosen $other "model-menu") -match "Sphere") "and the same model in the menu ('$(Chosen $other 'model-menu')')"
		Wait-Settled $other | Out-Null
		Shot $other "10-second-browser"

		# one browser's resolution is its own
		Click-Slider $other "resolution-slider" 0.02
		Wait-Settled $other | Out-Null
		Expect ((Hud $other).size -match "^(\d+)" -and [int]$Matches[1] -lt 400 -and (Hud $page).size -match "^(\d+)" -and [int]$Matches[1] -ge 950) "the resolution is not shared: the second browser gets small frames ('$((Hud $other).size)'), the first keeps full size ('$((Hud $page).size)')"
		Click-Slider $other "resolution-slider" 0.99
		Wait-Settled $other | Out-Null

		# what one does, the other shows
		Choose-Option $other "material-menu" 4
		Expect (Wait-For "ruby" { (Chosen $page "material-menu") -eq "Ruby" } 5) "choosing Ruby in the second browser changes the first one's menu ('$(Chosen $page 'material-menu')')"
		Wait-Settled $page | Out-Null
		Wait-Settled $other | Out-Null
		$ruby = Picture $page
		Expect ($ruby.red -gt $ruby.green * 1.4 -and (Picture $other).red -gt (Picture $other).green * 1.4) "and the object is red in both pictures"
		Shot $page "11-shared-ruby"

		$beforeDrag = Picture $page
		Invoke-Pointer $other @((Move-To 400 300), $script:Down, (Move-To 520 260 200), $script:Up)
		Expect (Wait-For "turned" { (Picture $page).checksum -ne $beforeDrag.checksum } 6) "dragging the object in the second browser turns it in the first"
		Wait-Settled $page | Out-Null
		Wait-Settled $other | Out-Null
		Shot $page "12-shared-turned"

		Click-Element $page "#wireframe-toggle"
		Expect (Wait-For "wire" { (Pressed $other "wireframe-toggle") -eq "true" -and (Pressed $page "wireframe-toggle") -eq "true" } 5) "switching wireframe on in the first browser presses the button in both"
		Wait-Settled $other | Out-Null
		Expect ((Picture $other).cyan -gt 150) "and draws the mesh edges in the second ($((Picture $other).cyan) cyan samples)"
		Click-Element $other "#wireframe-toggle"
		Expect (Wait-For "unwire" { (Pressed $page "wireframe-toggle") -eq "false" } 5) "and the second browser can switch it off again"

		Click-Element $other "#spin-toggle"
		Expect (Wait-For "spin" { (Pressed $page "spin-toggle") -eq "true" } 5) "starting the turntable in the second browser presses the button in the first"
		Start-Sleep -Milliseconds 1500
		Expect ([int](Hud $page)."frames/s" -ge 6 -and [int](Hud $other)."frames/s" -ge 6) "and both are streamed the turning object ($((Hud $page).'frames/s') and $((Hud $other).'frames/s') frames/s)"
		$spun1 = Picture $page
		Start-Sleep -Milliseconds 400
		Expect ($spun1.checksum -ne (Picture $page).checksum) "the first browser's picture keeps changing too"
		Click-Element $page "#spin-toggle"
		Expect (Wait-For "stopped" { (Pressed $other "spin-toggle") -eq "false" } 5) "the first browser stops it, and the second's button follows"
		Wait-Settled $page | Out-Null
		Wait-Settled $other | Out-Null

		# browsers with the same screen are sent the very same frames: one rendering, one compression
		$twin = New-Browser "1440,900"
		Open-Page $twin "http://127.0.0.1:2000/viewer/"
		Expect (Wait-For "three" { (Status $page).viewers -eq "3 viewers" -and (Status $twin).viewers -eq "3 viewers" } 8) "a third browser, as big as the first, makes it 3 viewers"
		Wait-Settled $twin | Out-Null
		Choose-Option $page "material-menu" 3
		Expect (Wait-For "ice" { (Chosen $twin "material-menu") -eq "Ice" -and (Chosen $other "material-menu") -eq "Ice" } 5) "choosing Ice in the first is followed by both of the others"
		Wait-Settled $page | Out-Null
		Wait-Settled $twin | Out-Null
		Wait-Settled $other | Out-Null
		$hudPage = Hud $page
		$hudTwin = Hud $twin
		$hudOther = Hud $other
		Expect ($hudPage.size -eq $hudTwin.size -and $hudPage."server render" -eq $hudTwin."server render" -and $hudPage."server encode" -eq $hudTwin."server encode") "the two browsers of the same size report the very same frame: size '$($hudPage.size)', render $($hudPage.'server render'), encode $($hudPage.'server encode')"
		Expect ($hudOther."server render" -ne $hudPage."server render") "while the smaller window, of another shape, was rendered for separately (render $($hudOther.'server render'), size '$($hudOther.size)')"
		Shot $twin "11b-twin"
		Close-Browser $twin
		$twin = $null

		# windows of any shape: the picture has the shape of the canvas, so a sphere is round whatever the
		# window (the server makes frames of at most 1280x720, which must not squeeze a tall or very wide one)
		foreach ($case in @(@{ name = "a tall, narrow window"; size = "560,1500"; tall = $true }, @{ name = "a very wide, low window"; size = "2200,560"; tall = $false })) {
			$shaped = New-Browser $case.size
			try {
				Open-Page $shaped "http://127.0.0.1:2000/viewer/"
				Wait-For "frame" { (Hud $shaped).frame -eq "refined" } 10 | Out-Null
				Wait-Settled $shaped | Out-Null
				$canvas = Run-Script $shaped $shapeJs
				Expect (($case.tall -and $canvas.h -gt 720) -or (-not $case.tall -and $canvas.w -gt 1280)) "$($case.name): its canvas ($($canvas.w)x$($canvas.h)) is bigger than the largest frame in the direction that matters"
				Expect ((Hud $shaped).size -match "^(\d+)\D+(\d+)") "and the HUD reports the frame ('$((Hud $shaped).size)')"
				$frameShape = [double]$Matches[1] / [double]$Matches[2]
				Expect ([math]::Abs([math]::Log($frameShape / ($canvas.w / $canvas.h))) -lt 0.03) "the frame has the shape of the canvas ($([math]::Round($frameShape, 3)) against $([math]::Round($canvas.w / $canvas.h, 3)))"
				Expect ([math]::Abs([math]::Log($canvas.boxWidth / $canvas.boxHeight)) -lt 0.06) "so the sphere is round on screen ($($canvas.boxWidth) x $($canvas.boxHeight) pixels)"
				Expect ($canvas.boxWidth -lt $canvas.w - 8 -and $canvas.boxHeight -lt $canvas.h - 8) "and fits the picture, not cut off at its edges"
				Shot $shaped ("12-" + ($case.name -replace '[^a-z]+', '-'))
			}
			finally { Close-Browser $shaped }
		}

		# not only browsers: any client of the protocol steers the same scene
		$raw = Open-Viewer
		Receive-Message $raw 3000 | Out-Null
		Send-Set $raw 5 1 @{ light = -1.0 }
		Expect (Wait-For "raw" { (Value $page "light-slider") -eq "-57$([char]0xb0)" -and (Value $other "light-slider") -eq "-57$([char]0xb0)" } 5) "a change from a client that is not a browser shows in both pages' controls (light $(Value $page 'light-slider'))"
		$raw.Socket.Dispose()

		Close-Browser $other
		$other = $null
		Expect (Wait-For "one" { (Status $page).viewers -eq "1 viewer" } 8) "when the second browser closes the first sees 1 viewer again"

		Heading "the server goes away and comes back"
		Wait-Settled $page | Out-Null
		Stop-Process -Id $server.Id -Force
		Expect (Wait-For "down" { (Status $page).state -eq "down" } 8) "the page says the connection is lost ('$((Status $page).text)')"
		Expect ((Status $page).text -match "reconnecting|offline") "and that it is retrying"
		Shot $page "13-offline"
		$server = Start-BackEnd $serverArguments $restartLog
		Expect (Wait-For "live" { (Status $page).state -eq "live" } 30) "the page reconnects by itself"
		Expect (Wait-Settled $page) "and the stream resumes"
		Expect ((Picture $page).checksum -eq $home_.checksum) "a restarted server starts again from its default scene, and the page follows it: the first picture again"
		Expect ((Value $page "light-slider") -eq "40$([char]0xb0)" -and (Value $page "shininess-slider") -eq "48" -and (Chosen $page "material-menu") -eq "Bronze" -and (Chosen $page "model-menu") -match "Torus knot") "with its controls back at the defaults too (light $(Value $page 'light-slider'), $(Chosen $page 'material-menu'), $(Chosen $page 'model-menu'))"
	}
}
catch {
	# an error inside the run would otherwise stop it silently and let the script go on to say "passed"
	Write-Host ("  FAIL the run stopped early: " + ($_ | Out-String))
	$script:failures.Add("the run stopped early: $_")
}
finally {
	Close-Browser $page
	Close-Browser $other
	Close-Browser $twin
	Stop-Process -Id $driver.Id -Force -ErrorAction SilentlyContinue
	Start-Sleep -Milliseconds 500
	Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
}

Write-Host ("server log   [{0:N0} s]" -f ((Get-Date) - $script:runStart).TotalSeconds)
$log = (Read-BackEndLog $serverLog) + "`n" + (Read-BackEndLog $restartLog)
Write-Host (($log -split "`n" | Where-Object { $_ -match "ill-formed|refused|panicked|error" }) -join "`n" -replace "(?m)^", "  | ")
Expect (([regex]::Matches($log, "viewer connected")).Count -ge $(if ($Quick) { 9 } else { 13 })) "the server saw every connection ($(([regex]::Matches($log, 'viewer connected')).Count))"
Expect (([regex]::Matches($log, "ill-formed viewer message ignored")).Count -eq 2) "it reported exactly the two malformed messages the test sent"
Expect ($log -match 'WebSocket refused, origin Some\("https://evil.example"\)') "it refused the foreign origin, and logged it escaped"
Expect ($log -notmatch "panicked") "nothing in the server panicked"

if ($script:failures.Count -gt 0) { Write-Host "`n$($script:failures.Count) check(s) failed"; exit 1 }
Write-Host "`nall checks passed"
