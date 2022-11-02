#pragma once

// moth:
#include "Buffer.h"

// std library:
#include "StdIncludes.h"


namespace Moth {


// -----------------
//! \brief		Client WebSocket abstraction
class WebSocket
{
// definitions:
public:
	//! \brief		callback signature to be called when ready
	using FnOnReady = std::function< void() >;

	//! \brief		callback signature to be called when receiving data
	//! \param[in]	_data		a buffer (weak-possession) over the received data
	using FnOnReceive = std::function< void( Buffer && _data ) >;

	//! \brief		callback signature to be called when the socket was terminated by the server
	using FnOnTerminated = std::function< void() >;

// ctor/dtor:
public:
	//! \brief		create a new WebSocket object
	//! \param[in]	_onReady		the callback to be called when the socket is ready (connected)
	//! \param[in]	_onReceive		the callback to be called with data when the socket receive something from the server
	//! \param[in]	_onTerminated	the callback to be called when the socket was terminated by the server
	//! \warning	will raise an assertion if our current browser is not compatible with WebSocket
	WebSocket( FnOnReady && _onReady, FnOnReceive && _onReceive, FnOnTerminated && _onTerminated );
	
	//! \brief		dtor, will close the socket if needed
	~WebSocket();
	
// public methods:
public:
	//! \brief		try to connect to a WebSocket server
	//! \param[in]	_address	the server address, under the form "ws://hostNameOrIp:portNumber"
	//! \param[in]	_protocol	[optional] the protocol to use for connection (default: none)
	//! \return		true if the WebSocket was properly connected
	//! \warning	will raise an assertion if we try to connect over an already connected WebSocket (need to close first)
	bool Connect( const std::wstring & _address, const std::wstring & _protocol = {} );

	//! \brief		try to send data to the server
	//! \param[in]	_data		the data to be sent
	//! \return		true if the sent was properly performed
	bool Send( const Buffer & _data );

	//! \brief		close the current client WebSocket, so you can re-use it
	void Close();
	
// members:
private:
	FnOnReady m_onReady; //!< the callback to be called when ready
	FnOnReceive m_onReceive; //!< the callback to be called when receiving data
	FnOnTerminated m_onTerminated; //!< the callback to be called when the socket was terminated by the server
	int m_ws{ 0 }; //!< internal handle over the emscripten websocket object
};


} // Moth