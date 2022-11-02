namespace Moth {


//! \brief  Generic container serialization to a Json Array
//! \tparam		_Container	the C++ std container type
//! \param[in]	_container	a reference over the C++ container to serialize from
//! \return		a Json Generic pointer
template< typename _Container >
static JsonGeneric::Ptr SerializeArray( const _Container & _container )
{
	Json::Array::Ptr arr;
	for( const auto & value : _container )
		arr.ptr->data.emplace_back( Json{} << value );
	return arr;
}


//! \brief  Generic container serialization to a Json Array (front version)
//! \details	In this case, the provided container must be copied in order to be popped
//! \tparam		_Container	the C++ std container type
//! \param[in]	_container	a reference over the C++ container to serialize from
//! \remark		a front() method must be accessible on the container
//! \param[in]	_fnEmplace	a Generic insertion functor
//! \return		a Json Generic pointer
template< typename _Container >
static JsonGeneric::Ptr SerializeArrayCopy( const _Container & _container, std::function< void( std::list< JsonGeneric::Ptr > & _array, JsonGeneric::Ptr && _genericPtr ) > && _fnEmplace )
{
	auto data{ _container }; //!< requires a copy (subsequent pop)
	Json::Array::Ptr arr;
	while( !data.empty() ) {
		_fnEmplace( arr.ptr->data, Json{} << data.front() );
		data.pop();
	}
	return arr;
}


//! \brief  Generic container serialization to a Json Array (top version)
//! \details	In this case, the provided container must be copied in order to be popped
//! \tparam		_Container	the C++ std container type
//! \param[in]	_container	a reference over the C++ container to serialize from
//! \remark		a top() method must be accessible on the container
//! \param[in]	_fnEmplace	a Generic insertion functor
//! \return		a Json Generic pointer
template< typename _Container >
static JsonGeneric::Ptr SerializeArrayStackCopy( const _Container & _container, std::function< void( std::list< JsonGeneric::Ptr > & _array, JsonGeneric::Ptr && _genericPtr ) > && _fnEmplace )
{
	auto data{ _container }; //!< requires a copy (subsequent pop)
	Json::Array::Ptr arr;
	while( !data.empty() ) {
		_fnEmplace( arr.ptr->data, Json{} << data.top() );
		data.pop();
	}
	return arr;
}


//! \brief  Generic container serialization to a Json Object
//! \tparam		_KeyType	the C++ std container key type (first in the std::pair)
//! \tparam		_Container	the C++ std container type
//! \param[in]	_container	a reference over the C++ container to serialize from
//! \param[in]	_fnKey	a key convertor functor (used to convert ANSI to UCS-2 keys)
//! \return		a Json Generic pointer
template< typename _KeyType, typename _Container >
static JsonGeneric::Ptr SerializeObjectPair( const _Container & _container, std::function< std::wstring( const _KeyType & _key ) > && _fnKey )
{
	Json::Object::Ptr obj;
	for( const auto & Pair : _container )
		*obj << Json::Pair< decltype( Pair.second ) >{ _fnKey( Pair.first ), Pair.second };
	return obj;
}


//! \brief  Generic container deserialization from a Json Array (forward)
//! \tparam		_Type	the C++ Object type
//! \param[in]	_generic	the Json Generic type to use as a Json Array
//! \param[in]	_fnEmplace	a Generic insertion functor
//! \return		a reference over the input Json Generic
template< typename _Type >
static const JsonGeneric & DeSerializeArray( const JsonGeneric & _generic, std::function< void( _Type && _value ) > && _fnEmplace )
{
	// deserialization from a Json Null:
	if( dynamic_cast< const Json::Null * >( &_generic ) != nullptr )
		return _generic;

	// regular case:
	const auto & arr{ _generic.As< Json::Array >() };
	for( const auto & data : arr.data ) {
		_Type value;
		*data >> value;
		_fnEmplace( std::move( value ) );
	}
	return _generic;
}


//! \brief  Generic container deserialization from a Json Array (backward)
//! \tparam		_Type	the C++ Object type
//! \param[in]	_generic	the Json Generic type to use as a Json Array
//! \param[in]	_fnEmplace	a Generic insertion functor
//! \remark		the Array will be parsed from back to front (reversed order)
//! \return		a reference over the input Json Generic
template< typename _Type >
static const JsonGeneric & DeSerializeArrayReverse( const JsonGeneric & _generic, std::function< void( _Type && _value ) > && _fnEmplace )
{
	// deserialization from a Json Null:
	if( dynamic_cast< const Json::Null * >( &_generic ) != nullptr )
		return _generic;

	// regular case:
	const auto & arr{ _generic.As< Json::Array >() };
	for( auto it{ arr.data.crbegin() }, itEnd{ arr.data.crend() }; it != itEnd; ++it  ) {
		_Type value;
		**it >> value;
		_fnEmplace( std::move( value ) );
	}
	return _generic;
}


//! \brief  Generic container deserialization from a Json Object
//! \tparam		_KeyType	the C++ std container key type (first in the std::pair)
//! \tparam		_Container	the C++ std container type
//! \param[in]	_generic	the Json Generic type to use as a Json Object
//! \param[in]	_fnEmplace	a Generic insertion functor, providing both the element key and value
//! \return		a reference over the input Json Generic
template< typename _KeyType, typename _Type >
static const JsonGeneric & DeSerializeObjectPair( const JsonGeneric & _generic, std::function< void( const _KeyType & _key, _Type && _value ) > && _fnEmplace )
{
	// deserialization from a Json Null:
	if( dynamic_cast< const Json::Null * >( &_generic ) != nullptr )
		return _generic;

	// regular case:
	const auto & obj{ _generic.As< Json::Object >() };
	for( const auto & Pair : obj.data ) {
		_Type value;
		*Pair.second >> value;
		_fnEmplace( Pair.first, std::move( value ) );
	}
	return _generic;
}


// ----------

//! \brief  std::list serialization
//! \tparam		_Type		the C++ Object type stored in the list (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::list to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::list< _Type > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::list deserialization
//! \tparam		_Type		the C++ Object type stored in the list (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::list to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::list< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace_back( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::vector serialization
//! \tparam		_Type		the C++ Object type stored in the vector (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::vector to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::vector< _Type > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::vector deserialization
//! \tparam		_Type		the C++ Object type stored in the vector (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::vector to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::vector< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace_back( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::array serialization
//! \tparam		_Type		the C++ Object type stored in the Array (will be automatically deduced)
//! \tparam		_Size		the C++ Array size (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::array to serialize from
//! \return		a Json Generic pointer
template< typename _Type, unsigned long long _Size >
JsonGeneric::Ptr operator << ( const Json &, const std::array< _Type, _Size > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::array deserialization
//! \tparam		_Type		the C++ Object type stored in the Array (will be automatically deduced)
//! \tparam		_Size		the C++ Array size (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::array to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type, unsigned long long _Size >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::array< _Type, _Size > & _data )
{
	// deserialization from a Json Null:
	if( dynamic_cast< const Json::Null * >( &_generic ) != nullptr )
		return _generic;

	// regular case:
	const auto & arr{ _generic.As< Json::Array >() };
	unsigned long long i{ 0 };
	for( const auto & data : arr.data ) {
		_Type value;
		*data >> value;
		_data.at( i++ ) = std::move( value );
	}
	return _generic;
}


// ----------

//! \brief  std::forward_list serialization
//! \tparam		_Type		the C++ Object type stored in the forward_list (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::forward_list to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::forward_list< _Type > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::forward_list deserialization
//! \tparam		_Type		the C++ Object type stored in the forward_list (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::forward_list to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::forward_list< _Type > & _data )
{
	return DeSerializeArrayReverse< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace_front( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::deque serialization
//! \tparam		_Type		the C++ Object type stored in the deque (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::deque to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::deque< _Type > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::deque deserialization
//! \tparam		_Type		the C++ Object type stored in the deque (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::deque to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::deque< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace_back( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::stack serialization
//! \tparam		_Type		the C++ Object type stored in the stack (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::stack to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::stack< _Type > & _data )
{
	return SerializeArrayStackCopy( _data, []( auto & _array, auto && _genericPtr ) {
			_array.emplace_front( std::move( _genericPtr ) );
		} );
}


//! \brief  std::stack deserialization
//! \tparam		_Type		the C++ Object type stored in the stack (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::stack to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::stack< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::queue serialization
//! \tparam		_Type		the C++ Object type stored in the queue (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::queue to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::queue< _Type > & _data )
{
	return SerializeArrayCopy( _data, []( auto & _array, auto && _genericPtr ) {
			_array.emplace_back( std::move( _genericPtr ) );
		} );
}


//! \brief  std::queue deserialization
//! \tparam		_Type		the C++ Object type stored in the queue (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::queue to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::queue< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::priority_queue serialization
//! \tparam		_Type		the C++ Object type stored in the priority_queue (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::priority_queue to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::priority_queue< _Type > & _data )
{
	return SerializeArrayStackCopy( _data, []( auto & _array, auto && _genericPtr ) {
			_array.emplace_back( std::move( _genericPtr ) );
		} );
}


//! \brief  std::priority_queue deserialization
//! \tparam		_Type		the C++ Object type stored in the priority_queue (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::priority_queue to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::priority_queue< _Type > & _data )
{
	return DeSerializeArrayReverse< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::set serialization
//! \tparam		_Type		the C++ Object type stored in the set (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::set to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::set< _Type > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::set deserialization
//! \tparam		_Type		the C++ Object type stored in the set (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::set to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::set< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::unordered_set serialization
//! \tparam		_Type		the C++ Object type stored in the unordered_set (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::unordered_set to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::unordered_set< _Type > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::unordered_set deserialization
//! \tparam		_Type		the C++ Object type stored in the unordered_set (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::unordered_set to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::unordered_set< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::multiset serialization
//! \tparam		_Type		the C++ Object type stored in the multiset (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::multiset to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::multiset< _Type > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::multiset deserialization
//! \tparam		_Type		the C++ Object type stored in the multiset (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::multiset to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::multiset< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::unordered_multiset serialization
//! \tparam		_Type		the C++ Object type stored in the unordered_multiset (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::unordered_multiset to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::unordered_multiset< _Type > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::unordered_multiset deserialization
//! \tparam		_Type		the C++ Object type stored in the unordered_multiset (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::unordered_multiset to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::unordered_multiset< _Type > & _data )
{
	return DeSerializeArray< _Type >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::map serialization
//! \tparam		_Key		the C++ Object type used for the key (will be automatically deduced)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::map to serialize from
//! \return		a Json Generic pointer
template< typename _Key, typename _Value >
JsonGeneric::Ptr operator << ( const Json &, const std::map< _Key, _Value > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::map deserialization
//! \tparam		_Key		the C++ Object type used for the key (will be automatically deduced)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::map to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Key, typename _Value >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::map< _Key, _Value > & _data )
{
	return DeSerializeArray< std::pair< _Key, _Value > >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::map serialization (UCS-2 String version)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::map to serialize from
//! \remark		this is a specific Object serialization process used when the key is a String
//! \return		a Json Generic pointer
template< typename _Value >
JsonGeneric::Ptr operator << ( const Json &, const std::map< std::wstring, _Value > & _data )
{
	return SerializeObjectPair< std::wstring >( _data, []( const auto & _key ){ return _key; } );
}


//! \brief  std::map deserialization (UCS-2 String version)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::map to deserialize to
//! \remark		this is a specific Object deserialization process used when the key is a String
//! \return		a reference over the input Json Generic
template< typename _Value >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::map< std::wstring, _Value > & _data )
{
	return DeSerializeObjectPair< std::wstring, _Value >( _generic, [ &_data ]( const auto & _key, auto && _value ) {
			_data.emplace( _key, std::move( _value ) );
		} );
}


// ----------

//! \brief  std::map serialization (ANSI String version)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::map to serialize from
//! \remark		this is a specific Object serialization process used when the key is a String
//! \return		a Json Generic pointer
template< typename _Value >
JsonGeneric::Ptr operator << ( const Json &, const std::map< std::string, _Value > & _data )
{
	return SerializeObjectPair< std::string >( _data, []( const auto & _key ) {
			return System::MbcsToWide( _key );
		} );
}


//! \brief  std::map deserialization (ANSI String version)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::map to deserialize to
//! \remark		this is a specific Object deserialization process used when the key is a String
//! \return		a reference over the input Json Generic
template< typename _Value >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::map< std::string, _Value > & _data )
{
	return DeSerializeObjectPair< std::wstring, _Value >( _generic, [ &_data ]( const auto & _key, auto && _value ) {
			_data.emplace( System::WideToMbcs( _key ), std::move( _value ) );
		} );
}


// ----------

//! \brief  std::unordered_map serialization
//! \tparam		_Key		the C++ Object type used for the key (will be automatically deduced)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::unordered_map to serialize from
//! \return		a Json Generic pointer
template< typename _Key, typename _Value >
JsonGeneric::Ptr operator << ( const Json &, const std::unordered_map< _Key, _Value > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::unordered_map deserialization
//! \tparam		_Key		the C++ Object type used for the key (will be automatically deduced)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::unordered_map to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Key, typename _Value >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::unordered_map< _Key, _Value > & _data )
{
	return DeSerializeArray< std::pair< _Key, _Value > >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::unordered_map serialization (UCS-2 String version)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::unordered_map to serialize from
//! \remark		this is a specific Object serialization process used when the key is a String
//! \return		a Json Generic pointer
template< typename _Value >
JsonGeneric::Ptr operator << ( const Json &, const std::unordered_map< std::wstring, _Value > & _data )
{
	return SerializeObjectPair< std::wstring >( _data, []( const auto & _key ){ return _key; } );
}


//! \brief  std::unordered_map deserialization (UCS-2 String version)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::unordered_map to deserialize to
//! \remark		this is a specific Object deserialization process used when the key is a String
//! \return		a reference over the input Json Generic
template< typename _Value >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::unordered_map< std::wstring, _Value > & _data )
{
	return DeSerializeObjectPair< std::wstring, _Value >( _generic, [ &_data ]( const auto & _key, auto && _value ) {
			_data.emplace( _key, std::move( _value ) );
		} );
}


// ----------

//! \brief  std::unordered_map serialization (ANSI String version)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::unordered_map to serialize from
//! \remark		this is a specific Object serialization process used when the key is a String
//! \return		a Json Generic pointer
template< typename _Value >
JsonGeneric::Ptr operator << ( const Json &, const std::unordered_map< std::string, _Value > & _data )
{
	return SerializeObjectPair< std::string >( _data, []( const auto & _key ) {
			return System::MbcsToWide( _key );
		} );
}


//! \brief  std::unordered_map deserialization (ANSI String version)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::unordered_map to deserialize to
//! \remark		this is a specific Object deserialization process used when the key is a String
//! \return		a reference over the input Json Generic
template< typename _Value >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::unordered_map< std::string, _Value > & _data )
{
	return DeSerializeObjectPair< std::wstring, _Value >( _generic, [ &_data ]( const auto & _key, auto && _value ) {
			_data.emplace( System::WideToMbcs( _key ), std::move( _value ) );
		} );
}


// ----------

//! \brief  std::multimap serialization
//! \tparam		_Key		the C++ Object type used for the key (will be automatically deduced)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::multimap to serialize from
//! \return		a Json Generic pointer
template< typename _Key, typename _Value >
JsonGeneric::Ptr operator << ( const Json &, const std::multimap< _Key, _Value > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::multimap deserialization
//! \tparam		_Key		the C++ Object type used for the key (will be automatically deduced)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::multimap to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Key, typename _Value >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::multimap< _Key, _Value > & _data )
{
	return DeSerializeArray< std::pair< _Key, _Value > >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


// ----------

//! \brief  std::unordered_multimap serialization
//! \tparam		_Key		the C++ Object type used for the key (will be automatically deduced)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::unordered_multimap to serialize from
//! \return		a Json Generic pointer
template< typename _Key, typename _Value >
JsonGeneric::Ptr operator << ( const Json &, const std::unordered_multimap< _Key, _Value > & _data )
{
	return SerializeArray( _data );
}


//! \brief  std::unordered_multimap deserialization
//! \tparam		_Key		the C++ Object type used for the key (will be automatically deduced)
//! \tparam		_Value		the C++ Object type used for the value (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::unordered_multimap to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Key, typename _Value >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::unordered_multimap< _Key, _Value > & _data )
{
	return DeSerializeArray< std::pair< _Key, _Value > >( _generic, [ &_data ]( auto && _value ) {
			_data.emplace( std::move( _value ) );
		} );
}


} // Moth