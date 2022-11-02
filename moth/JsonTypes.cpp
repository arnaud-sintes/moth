#include "Json.h"


namespace Moth {


// Json Numbers Generic definition:
#define JsonNumberImpl( _Type ) \
template<> \
JsonGeneric::Ptr operator << ( const Json &, const _Type & _data ) \
{ \
	System::Assert( _data >= std::numeric_limits< double >::lowest() && _data <= std::numeric_limits< double >::max(), L"Number is too big to be stored in a double" ); \
	return Json::Number::Ptr{ static_cast< double >( _data ) }; \
} \
 \
template<> \
const JsonGeneric & operator >> ( const JsonGeneric & _generic, _Type & _data ) \
{ \
	const auto value{ _generic.As< Json::Number >().data }; \
	System::Assert( value >= static_cast< double >( std::numeric_limits< _Type >::lowest() ) && value <= static_cast< double >( std::numeric_limits< _Type >::max() ), L"bad internal Number Storage to _Type conversion" ); \
	_data = static_cast< const _Type >( value ); \
	return _generic; \
}


// Json Numbers specialization (implementation):
JsonNumberImpl( char );
JsonNumberImpl( unsigned char );
JsonNumberImpl( short );
JsonNumberImpl( unsigned short );
JsonNumberImpl( long );
JsonNumberImpl( unsigned long );
JsonNumberImpl( long long );
JsonNumberImpl( unsigned long long );
JsonNumberImpl( float );
JsonNumberImpl( double );
JsonNumberImpl( int );
JsonNumberImpl( unsigned int );
JsonNumberImpl( int8_t );
JsonNumberImpl( wchar_t );


} // Moth