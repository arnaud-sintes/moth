// required dependencies:
const http = require( "http" );
const ws = require( "ws" );
const fs = require( "fs" );

// configuration settings:
const settings =
{
	httpPort: 1000,
	wsPort: 2000,
	httpFolder: process.argv[ 2 ]
};

// create HTTP server:
console.log( "> Http listening over folder '" + settings.httpFolder + "' on port", settings.httpPort );
const express = require( "express" );
const app = express();
// set properly cross-origin parameters:
app.use( ( _, _res, _next ) =>
{
	_res.header( "Cross-Origin-Embedder-Policy", "require-corp" );
	_res.header( "Cross-Origin-Opener-Policy", "same-origin" );
	_next();
} );
app.use( express.static( __dirname + "/" + settings.httpFolder ) );
app.listen( settings.httpPort );

// create WebSocket server:
console.log( "> WebSocket listening on port", settings.wsPort );
const wss = new ws.WebSocketServer( { port: settings.wsPort } );
wss.on( "connection", ( _ws, _request ) =>
{
	console.log( "[" + _request.socket.remoteAddress + "]", "WebSocket client connection" );
	_ws.on( "message", _data =>
	{
		const data = JSON.parse( _data.toString() );
		if( data.message != undefined )
			return console.log( "[" + _request.socket.remoteAddress + "]", "WebSocket receive '" + data.message + "'" );
		if( data.stream != undefined )
			return Stream( _ws, _request, data.stream );
	} );
	_ws.on( "close", _event =>
	{
		console.log( "[" + _request.socket.remoteAddress + "]", "WebSocket client disconnection" );
	} );
	_ws.send( JSON.stringify(
		{
			message: "(Back-End) you're now connected '" + _request.socket.remoteAddress + "'"
		}
	) );
} );


// load image to stream as Base64 string:
const imageBase64 = "data:image/jpeg;base64," + fs.readFileSync( "image.jpg" ).toString( "base64" );

// streaming timer:
var stream = null;

// stream function:
function Stream( _ws, _request, _stream )
{
	console.log( "[" + _request.socket.remoteAddress + "]", "Image streaming: " + ( _stream ? "active" : "inactive" ) );
	// clear previous stream interval if any:
	if( stream ) {
		clearInterval( stream );
		stream = null;
	}
	// create interval if active:
	if( _stream )
		stream = setInterval( _ =>
			_ws.send( JSON.stringify(
				{
					image: imageBase64
				}
			) )
		, 0 );
	return;
}