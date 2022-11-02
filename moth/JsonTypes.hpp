namespace Moth {


//! \brief  Json Numbers serialization
//! \tparam		_Type		the C++ Object type to serialize as a Json Number (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ Object to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const _Type & _data );


//! \brief  Json Numbers deserialization
//! \tparam		_Type		the C++ Object type to deserialize from a Json Number (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use as a Json Number
//! \param[in,out]	_data		a reference over the C++ Object to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, _Type & _data );


// Json Numbers Generic definition:
#define JsonNumber( _Type )                                                     \
template<>                                                                      \
JsonGeneric::Ptr operator << ( const Json &, const _Type & _data );             \
                                                                                \
template<>                                                                      \
const JsonGeneric & operator >> ( const JsonGeneric & _generic, _Type & _data )


// Json Numbers specialization (declaration):
JsonNumber( char );
JsonNumber( unsigned char );
JsonNumber( short );
JsonNumber( unsigned short );
JsonNumber( long );
JsonNumber( unsigned long );
JsonNumber( long long );
JsonNumber( unsigned long long );
JsonNumber( float );
JsonNumber( double );
JsonNumber( int );
JsonNumber( unsigned int );
JsonNumber( int8_t );
JsonNumber( wchar_t );


// ----------

//! \brief  Json Boolean serialization (declaration)
JsonGeneric::Ptr operator << ( const Json &, const bool & _data );

//! \brief  Json Boolean deserialization (declaration)
const JsonGeneric & operator >> ( const JsonGeneric & _generic, bool & _data );


// ----------

//! \brief  Json std::wstring (UCS-2/UTF-16) serialization (declaration)
JsonGeneric::Ptr operator << ( const Json &, const std::wstring & _data );

//! \brief  Json std::wstring (UCS-2/UTF-16) deserialization (declaration)
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::wstring & _data );


// ----------

//! \brief  Json std::wstring (ANSI) serialization (declaration)
JsonGeneric::Ptr operator << ( const Json &, const std::wstring & _data );

//! \brief  Json std::wstring (ANSI) deserialization (declaration)
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::wstring & _data );


// ----------

//! \brief  Json Array stacking
//! \tparam		_Type		the C++ Object type to stack in the Json Array (will be automatically deduced)
//! \param[in,out]	_array		a reference over the Json Array in which the data will be stacked
//! \param[in]	_data		a reference over the C++ Object to stack
//! \return		a reference over the Json Array
template< typename _Type >
Json::Array & operator << ( Json::Array & _array, const _Type & _data )
{
	_array.data.emplace_back( Json{} << _data );
	return _array;
}


//! \brief  Json Array unstacking
//! \tparam		_Type		the C++ Object type to unstack from the Json Array (will be automatically deduced)
//! \param[in]		_array		a reference over the Json Array in which the data will be unstacked
//! \param[in,out]	_data		a reference over the C++ Object where we will unstack the data
//! \return		a reference over the Json Array
template< typename _Type >
const Json::Array & operator >> ( const Json::Array & _array, _Type & _data )
{
	// we can safely break the constness here, in order to increment the deserialization iterator:
	auto & pitDeserialize{ const_cast< Json::Array & >( _array ).pitDeserialize };

	// deserialization iterator must be initialized the first time:
	if( !pitDeserialize )
		pitDeserialize.reset( new std::list< JsonGeneric::Ptr >::const_iterator{ _array.data.cbegin() } );

	System::Assert( *pitDeserialize != _array.data.cend(), L"too much unstacking" );
	***pitDeserialize >> _data;
	++*pitDeserialize;
	return _array;
}


// ----------

//! \brief  Json Object stacking
//! \tparam		_Type		the C++ Object type to stack in the Json Object (will be automatically deduced)
//! \param[in,out]	_object		a reference over the Json Object in which the data will be stacked
//! \param[in]	_data		a Json Pair containing the data identifier name and a reference over the data to stack
//! \return		a reference over the Json Object
template< typename _Type >
Json::Object & operator << ( Json::Object & _object, Json::Pair< _Type > && _data )
{
	System::Assert( _object.data.find( _data.name ) == _object.data.cend(), L"key already exists" );
	_object.data.emplace( _data.name, Json{} << _data.data );
	return _object;
}


//! \brief  Json Object unstacking
//! \tparam		_Type		the C++ Object type to unstack from the Json Object (will be automatically deduced)
//! \param[in]	_object		a reference over the Json Object in which the data will be unstacked
//! \param[in,out]	_data		a Json Pair containing the data identifier name and a reference over the data used to unstack
//! \return		a reference over the Json Object
template< typename _Type >
const Json::Object & operator >> ( const Json::Object & _object, Json::Pair< _Type > && _data )
{
	auto it{ _object.data.find( _data.name ) };
	if( it != _object.data.cend() ) {
		*it->second >> _data.data;
		return _object;
	}

	// if the key identifier (data name) was not found in the Object, we try fallback to a Json Null deserialization
	// which is possible with some objects (std::optional, std::unique_ptr...):
	Json::Null Null;
	static_cast< JsonGeneric & >( Null ) >> _data.data;
	return _object;
}


} // Moth