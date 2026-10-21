# A raw (browser-less) client of the viewer protocol, for the end-to-end script and for measuring the
# server on its own. Dot-source it. Never cancels a receive: cancelling a pending ReceiveAsync aborts a
# .NET ClientWebSocket, so a timeout waits on the task, which stays pending for the next call.
#
# The scene (object, material, light, camera, turntable) is shared by everyone connected to the server:
# `Send-Set` asks for a change to it, `Send-Display` says how big this client's own frames are. Every
# text message the server sends is a hub snapshot with the scene in it, kept in `$viewer.LastHub`.

function Open-Viewer($port = 2000, $origin = $null) {
	$socket = New-Object System.Net.WebSockets.ClientWebSocket
	if ($origin) { $socket.Options.SetRequestHeader("Origin", $origin) }
	try { $socket.ConnectAsync([Uri]"ws://127.0.0.1:$port/viewer/ws", [Threading.CancellationToken]::None).Wait(4000) | Out-Null } catch { }
	[pscustomobject]@{ Socket = $socket; Pending = $null; Buffer = (New-Object byte[] 262144); LastHub = $null }
}

function Send-Json($viewer, $object) {
	$bytes = [Text.Encoding]::UTF8.GetBytes(($object | ConvertTo-Json -Compress -Depth 5))
	$viewer.Socket.SendAsync([ArraySegment[byte]]$bytes, "Text", $true, [Threading.CancellationToken]::None).Wait(3000) | Out-Null
}

function Send-Raw($viewer, [byte[]]$bytes, $type = "Binary") {
	$viewer.Socket.SendAsync([ArraySegment[byte]]$bytes, $type, $true, [Threading.CancellationToken]::None).Wait(3000) | Out-Null
}

# This client's own frame size and quality: `$scale` is the share of the canvas the full frame is
# rendered at, `$preview` the share of the full frame the previews (sent while the scene moves) are,
# `$quality` and `$previewQuality` the JPEG quality (10 to 100) of the one and of the other.
function Send-Display($viewer, $width = 640, $height = 480, $scale = 1.0, $preview = 0.5, $quality = 90, $previewQuality = 60) {
	Send-Json $viewer @{ type = "display"; width = $width; height = $height; scale = $scale; preview = $preview; quality = $quality; preview_quality = $previewQuality }
}

# A change to the shared scene: `$fields` holds only what changes (model, material, yaw, pitch, distance,
# light, shininess, wireframe, spin). `$client` names the sender and `$seq` counts its changes.
function Send-Set($viewer, $client, $seq, $fields = @{}) {
	$message = @{ type = "set"; client = $client; seq = $seq }
	foreach ($key in $fields.Keys) { $message[$key] = $fields[$key] }
	Send-Json $viewer $message
}

# Everything back to the defaults, whoever changed it last.
function Reset-Scene($viewer, $client = 900, $seq = 1) {
	Send-Set $viewer $client $seq @{ model = "torus_knot"; material = "bronze"; yaw = 0.6; pitch = 0.35; distance = 6.3; light = 0.7; shininess = 48; wireframe = $false; spin = $false }
}

# The next whole message, or $null after `$ms` without one (or if the connection ended).
function Receive-Message($viewer, $ms) {
	$stream = New-Object IO.MemoryStream
	while ($true) {
		if (-not $viewer.Pending) { $viewer.Pending = $viewer.Socket.ReceiveAsync([ArraySegment[byte]]$viewer.Buffer, [Threading.CancellationToken]::None) }
		try { if (-not $viewer.Pending.Wait($ms)) { return $null } } catch { $viewer.Pending = $null; return $null }
		$received = $viewer.Pending.Result
		$viewer.Pending = $null
		if ($received.MessageType -eq "Close") { return $null }
		$stream.Write($viewer.Buffer, 0, $received.Count)
		if ($received.EndOfMessage) {
			$message = [pscustomobject]@{ Type = $received.MessageType.ToString(); Bytes = $stream.ToArray() }
			if ($message.Type -eq "Text") { $viewer.LastHub = [Text.Encoding]::UTF8.GetString($message.Bytes) | ConvertFrom-Json }
			return $message
		}
	}
}

function Read-Frame($bytes) {
	[pscustomobject]@{
		Magic = [Text.Encoding]::ASCII.GetString($bytes[0..3])
		Seq = [BitConverter]::ToUInt32($bytes, 4)
		ViewId = [BitConverter]::ToUInt32($bytes, 8)
		Kind = $(if ($bytes[12] -eq 0) { "preview" } else { "final" })
		Width = [BitConverter]::ToUInt16($bytes, 14)
		Height = [BitConverter]::ToUInt16($bytes, 16)
		RenderMs = [BitConverter]::ToUInt32($bytes, 18) / 1000
		EncodeMs = [BitConverter]::ToUInt32($bytes, 22) / 1000
		Jpeg = ($bytes.Length -gt 27 -and $bytes[26] -eq 255 -and $bytes[27] -eq 216)
		Length = $bytes.Length
	}
}

# Receive frames until nothing arrives for a while; acknowledge each if `$ack`. "A while" is `$ms`,
# but at most 800: a preview is followed by the full frame after 220 ms of rest plus a render (well
# under half a second), so a longer silence adds waiting and no information.
function Receive-Frames($viewer, $ms, $ack = $true) {
	$frames = @()
	$quiet = [math]::Min($ms, 800)
	while ($true) {
		$message = Receive-Message $viewer $quiet
		if (-not $message) { break }
		if ($message.Type -ne "Binary") { continue }
		$frame = Read-Frame $message.Bytes
		$frames += $frame
		if ($ack) { Send-Json $viewer @{ type = "ack"; seq = [int]$frame.Seq } }
	}
	$frames
}

# Receive for `$seconds` whatever arrives (a stream that never goes quiet, like the turntable's):
# the frames, and the scene of every hub snapshot. Frames are acknowledged if `$ack`.
function Watch-Viewer($viewer, $seconds, $ack = $true) {
	$watched = [pscustomobject]@{ Frames = @(); Scenes = @() }
	$deadline = (Get-Date).AddSeconds($seconds)
	while ($true) {
		$left = [int]($deadline - (Get-Date)).TotalMilliseconds
		if ($left -lt 1) { break }
		$message = Receive-Message $viewer $left
		if (-not $message) { break }
		if ($message.Type -eq "Text") { $watched.Scenes += $viewer.LastHub.scene; continue }
		$frame = Read-Frame $message.Bytes
		$watched.Frames += $frame
		if ($ack) { Send-Json $viewer @{ type = "ack"; seq = [int]$frame.Seq } }
	}
	$watched
}

# The newest scene the server has announced to this client, after reading (and acknowledging the
# frames of) whatever arrives until `$ms` pass quietly.
function Latest-Scene($viewer, $ms = 400) {
	while ($message = Receive-Message $viewer $ms) {
		if ($message.Type -eq "Binary") { Send-Json $viewer @{ type = "ack"; seq = [int](Read-Frame $message.Bytes).Seq } }
	}
	$viewer.LastHub.scene
}
