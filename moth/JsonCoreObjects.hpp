#include "VariadicExpanderMacros.h"

 namespace Moth {


// -----------------
// json reflection of an IndexedMap:

template< typename _Key, typename _Value >
::Moth::JsonGeneric::Ptr operator << ( const ::Moth::Json &, const IndexedMap< _Key, _Value > & _data )
{
	Json::Object::Ptr obj;
	for( const auto & pair : _data )
		*obj << Json::Pair< decltype( pair.second ) >{ pair.first, pair.second };
	return obj;
}


template< typename _Key, typename _Value >
const ::Moth::JsonGeneric & operator >> ( const ::Moth::JsonGeneric & _generic, IndexedMap< _Key, _Value > & _data )
{
	return DeSerializeObjectPair< _Key, _Value >( _generic, [ &_data ]( const auto & _key, auto && _value )
	{
		_data.emplace( _key, std::move( _value ) );
	} );
}



} // Moth