#pragma once

// std library:
#include "StdIncludes.h"


namespace Moth {


// -----------------
//! \brief		Deal with raw memory buffers
//! details		This is a weak possession container, not owning the data
struct Buffer
{
	const void * const pData;			//!< data pointer
	const unsigned long long length;	//!< data length

	//! \brief		Convert the current buffer to a given container object
	//! \tparam		_Type		the target container to convert to
	template< typename _Type >
	_Type To() const;

	//! \brief		Create a buffer from a given container object
	//! \tparam		_Type		the source container type
	//! \param[in]	_data		a reference over the source data container
	//! \return		a new buffer object instance
	template< typename _Type >
	static Buffer From( const _Type & _data );
};


//! \brief		generic implementation
template< typename _Type >
_Type Buffer::To() const
{
	_Type data;
	data.resize( length );
	std::memcpy( data.data(), pData, length );
	return data;
}


//! \brief		specific std::string implementation
template<>
Buffer Buffer::From( const std::string & _data );


} // Moth