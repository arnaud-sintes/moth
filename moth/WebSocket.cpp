#include "WebSocket.h"

// moth:
#include "System.h"

// emscripten:
#include <emscripten/websocket.h>


namespace Moth {


WebSocket::WebSocket( FnOnReady && _onReady, FnOnReceive && _onReceive, FnOnTerminated && _onTerminated )
	: m_onReady{ std::move( _onReady ) }
	, m_onReceive{ std::move( _onReceive ) }
	, m_onTerminated{ std::move( _onTerminated ) }
{
	System::Assert( emscripten_websocket_is_supported(), L"WebSockets are not supported" );
}


WebSocket::~WebSocket()
{
	Close();
}


bool WebSocket::Connect( const std::wstring & _address, const std::wstring & _protocol )
{
	System::Assert( m_ws == 0, L"WebSocket already connected" );
	const auto address{ System::WideToMbcs( _address ) };
	const auto protocol{ System::WideToMbcs( _protocol ) };
	EmscriptenWebSocketCreateAttributes wsAttributes{
        address.c_str(),
        _protocol.empty() ? nullptr : protocol.c_str(),
        EM_TRUE
    };
	m_ws = emscripten_websocket_new( &wsAttributes );
    if( m_ws == 0 )
        return false;
	emscripten_websocket_set_onopen_callback( m_ws, this, []( int, const auto *, void * _this )
	{
		reinterpret_cast< WebSocket * >( _this )->m_onReady();
		return EM_TRUE;
	} );
	emscripten_websocket_set_onclose_callback( m_ws, this, []( int, const auto *, void * _this )
	{
		reinterpret_cast< WebSocket * >( _this )->m_onTerminated();
		return EM_TRUE;
	} );
	emscripten_websocket_set_onmessage_callback( m_ws, this, []( int, const auto * _event, void * _this )
	{
		reinterpret_cast< WebSocket * >( _this )->m_onReceive( Buffer{ _event->data, _event->numBytes } );
		return EM_TRUE;
	} );
	return true;
}


bool WebSocket::Send( const Buffer & _data )
{
	return emscripten_websocket_send_binary( m_ws, const_cast< void * >( _data.pData ), _data.length );
}


void WebSocket::Close()
{
	if( m_ws == 0 )
		return;
	emscripten_websocket_delete( m_ws );
	m_ws = 0;
}


} // Moth