#include "Buffer.h"


namespace Moth {


template<>
Buffer Buffer::From( const std::string & _data )
{
	return Buffer{ _data.c_str(), _data.size() };
}


} // Moth