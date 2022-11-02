#include "Base64.h"


namespace Moth {


constexpr std::string_view g_b64c{ "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/" };


static bool _IsBase64( const unsigned char _c )
{
  return std::isalnum( _c ) || ( _c == '+' ) || ( _c == '/' );
}


std::string Base64::Encode( const Buffer & _data )
{
	const auto * data{ static_cast< const unsigned char * >( _data.pData ) };
	unsigned long long dataLength{ _data.length };
	std::string encodedData;
	unsigned char ca3[ 3 ], ca4[ 4 ];
	auto fn{ [ & ]
	{
		ca4[ 0 ] = ( ca3[ 0 ] & 0xfc ) >> 2;
		ca4[ 1 ] = ( ( ca3[ 0 ] & 0x03 ) << 4 ) + ( ( ca3[ 1 ] & 0xf0 ) >> 4 );
		ca4[ 2 ] = ( ( ca3[ 1 ] & 0x0f ) << 2 ) + ( ( ca3[ 2 ] & 0xc0 ) >> 6 );
		ca4[ 3 ] = ca3[ 2 ] & 0x3f;
	} };
	int pos{ 0 };
	while( dataLength-- != 0 ) {
		ca3[ pos++ ] = *( data++ );
		if( pos != 3 )
			continue;
		fn();
		for( pos = 0; pos < 4; pos++ )
			encodedData.push_back( g_b64c[ ca4[ pos ] ] );
		pos = 0;
	}
	if( pos == 0 )
		return encodedData;
	for( int i{ pos }; i < 3; i++ )
		ca3[ i ] = '\0';
	fn();
	for( int i{ 0 }; i < pos + 1; i++ )
		encodedData.push_back( g_b64c[ ca4[ i ] ] );
	while( pos++ < 3 )
		encodedData.push_back( '=' );
	return encodedData;
}


std::vector< unsigned char > Base64::Decode( const std::string & _encodedData )
{
	const char * const encodedData{ _encodedData.c_str() };
	size_t dataLength{ _encodedData.size() };
	std::vector< unsigned char > data;
	unsigned char ca4[ 4 ], ca3[ 3 ];
	auto fn{ [ & ]
	{
		for( int i{ 0 }; i < 4; i++ )
			ca4[ i ] = static_cast< char >( g_b64c.find( ca4[ i ] ) );
		ca3[ 0 ] = ( ca4[ 0 ] << 2 ) + ( ( ca4[ 1 ] & 0x30 ) >> 4 );
		ca3[ 1 ] = ( ( ca4[ 1 ] & 0xf ) << 4 ) + ( ( ca4[ 2 ] & 0x3c ) >> 2 );
		ca3[ 2 ] = ( ( ca4[ 2 ] & 0x3 ) << 6 ) + ca4[ 3 ];
	} };
	int epos{ 0 }, pos{ 0 };
	while( dataLength-- != 0 && encodedData[ epos ] != '=' && _IsBase64( static_cast< const unsigned char >( encodedData[ epos ] ) ) ) {
		ca4[ pos++ ] = static_cast< const unsigned char >( encodedData[ epos++ ] );
		if( pos == 4 ) {
			fn();
			for( pos = 0; pos < 3; pos++ )
				data.push_back( ca3[ pos ] );
			pos = 0;
		}
	}
	if( pos == 0 )
		return data;
	for( int i{ pos }; i < 4; i++ )
		ca4[ i ] = 0;
	fn();
	for( int i( 0 ); i < pos - 1; i++ )
		data.push_back( ca3[ i ] );
	return data;
}


} // Moth