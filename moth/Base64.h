#pragma once

// std library:
#include "StdIncludes.h"

// Moth:
#include "Buffer.h"


namespace Moth {


// -----------------
//! \brief	Base64 encoding/decoding
struct Base64
{
	//! \brief	encode some data to a Base64 string
	//! \param[in]	_data	the data buffer to encode
	//! \return		the encoded Base64 string
	static std::string Encode( const Buffer & _data );

	//! \brief	decode a Base64 string
	//! \param[in]	_encodedData	encoded Base64 string
	//! \return		the decoded data
	static std::vector< unsigned char > Decode( const std::string & _encodedData );
};


} // Moth