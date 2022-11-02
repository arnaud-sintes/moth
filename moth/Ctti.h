#pragma once

// std library:
#include "StdIncludes.h"


namespace Moth {


// -----------------
//! \brief		Compile-Time Type Information:
struct Ctti
{
	// Ctti name:
	template< typename _Type >
	static constexpr std::string_view Name();

	// Ctti short name:
	template< typename _Type >
	static constexpr std::string_view ShortName();

	// compile-time FNV1a hash computation:
	static constexpr unsigned long long FNV1a( const std::string_view & _data );

	// Ctti hash computation:
	template< typename _Type >
	static constexpr unsigned long long Hash();

private:
	// Ctti name, without reference:
	template< typename _Type >
	static constexpr std::string_view _Name();

// internal:
private:
	// constants:
	static constexpr std::string_view left{ "[_Type = Moth::" };
};


template< typename _Type >
constexpr std::string_view Ctti::_Name()
{
	constexpr std::string_view funcSig{ __PRETTY_FUNCTION__ };
	constexpr size_t pos{ funcSig.find( left ) };
	return funcSig.substr( pos + left.length(), funcSig.length() - pos - left.length() - 1 );
}


template< typename _Type >
constexpr std::string_view Ctti::Name()
{
	return _Name< std::remove_cvref_t< _Type > >();
}


template< typename _Type >
constexpr std::string_view Ctti::ShortName()
{
	constexpr std::string_view data{ Name< _Type >() };
	constexpr unsigned long long pos{ data.find_last_of( ":" ) };
	if constexpr( pos == std::string_view::npos )
		return data;
	else
		return data.substr( pos + 1, data.size() - pos - 1 );
}


constexpr unsigned long long Ctti::FNV1a( const std::string_view & _data )
{
	unsigned long long hash{ 0xcbf29ce484222325 };
	for( auto && c : _data )
		hash = ( hash ^ static_cast< unsigned long long >( c ) ) * 0x100000001b3;
	return hash;
}


template< typename _Type >
constexpr unsigned long long Ctti::Hash()
{
	return FNV1a( Name< _Type >() );
}


} // Moth
