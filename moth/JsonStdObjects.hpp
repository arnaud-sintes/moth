namespace Moth {


//! \brief  std::atomic serialization
//! \tparam		_Type		the C++ Object type stored in the atomic (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::atomic to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::atomic< _Type > & _data )
{
	return Json{} << _data.load();
}


//! \brief  std::atomic deserialization
//! \tparam		_Type		the C++ Object type stored in the atomic (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::atomic to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::atomic< _Type > & _data )
{
	_Type value;
	_generic >> value;
	_data.store( value );
	return _generic;
}


// ----------

//! \brief  std::reference_wrapper serialization
//! \tparam		_Type		the C++ Object type referenced by the reference_wrapper (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::reference_wrapper to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::reference_wrapper< _Type > & _data )
{
	return Json{} << _data.get();
}


//! \brief  std::reference_wrapper deserialization
//! \tparam		_Type		the C++ Object type referenced by the reference_wrapper (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::reference_wrapper to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::reference_wrapper< _Type > & _data )
{
	_generic >> _data.get();
	return _generic;
}


// ----------

//! \brief  std::unique_ptr serialization
//! \tparam		_Type		the C++ Object type stored in the unique_ptr (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::unique_ptr to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::unique_ptr< _Type > & _data )
{
	return _data ? Json{} << *_data : Json::Null::Ptr{};
}


//! \brief  std::unique_ptr deserialization
//! \tparam		_Type		the C++ Object type stored in the unique_ptr (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::unique_ptr to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::unique_ptr< _Type > & _data )
{
	// deserialization from a Json Null:
	if( dynamic_cast< const Json::Null * >( &_generic ) != nullptr )
		return _generic;

	// regular case:
	_Type * pValue{ new _Type };
	_generic >> *pValue;
	_data.reset( pValue );
	return _generic;
}


// ----------

//! \brief  std::optional serialization
//! \tparam		_Type		the C++ Object type stored in the optional (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::optional to serialize from
//! \return		a Json Generic pointer
template< typename _Type >
JsonGeneric::Ptr operator << ( const Json &, const std::optional< _Type > & _data )
{
	return _data ? Json{} << *_data : Json::Null::Ptr{};
}


//! \brief  std::optional deserialization
//! \tparam		_Type		the C++ Object type stored in the optional (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::optional to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::optional< _Type > & _data )
{
	// deserialization from a Json Null:
	if( dynamic_cast< const Json::Null * >( &_generic ) != nullptr )
		return _generic;

	// regular case:
	_Type value;
	_generic >> value;
	_data.emplace( std::move( value ) );
	return _generic;
}


// ----------

//! \brief  std::pair serialization
//! \tparam		_Type1		the C++ Object type stored in the first Pair element (will be automatically deduced)
//! \tparam		_Type2		the C++ Object type stored in the second Pair element (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::pair to serialize from
//! \return		a Json Generic pointer
template< typename _Type1, typename _Type2 >
JsonGeneric::Ptr operator << ( const Json &, const std::pair< _Type1, _Type2 > & _data )
{
	Json::Array::Ptr arr;
	arr.ptr->data.emplace_back( Json{} << _data.first );
	arr.ptr->data.emplace_back( Json{} << _data.second );
	return arr;
}


//! \brief  std::pair deserialization
//! \tparam		_Type1		the C++ Object type stored in the first Pair element (will be automatically deduced)
//! \tparam		_Type2		the C++ Object type stored in the second Pair element (will be automatically deduced)
//! \param[in]	_generic	the Json Generic type to use for deserialization
//! \param[in,out]	_data	a reference over the C++ std::pair to deserialize to
//! \return		a reference over the input Json Generic
template< typename _Type1, typename _Type2 >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::pair< _Type1, _Type2 > & _data )
{
	const auto & arr{ _generic.As< Json::Array >() };
	*arr.data.front() >> _data.first;
	*arr.data.back() >> _data.second;
	return _generic;
}


// ----------

//! \brief  std::chrono serialization
//! \tparam		_Rep		time duration representation (will be automatically deduced)
//! \tparam		_Period		period of time (will be automatically deduced)
//! \param		Json		(just a placeholder for Json serialization streaming operation)
//! \param[in]	_data		a reference over the C++ std::chrono to serialize from
//! \return		a Json Generic pointer
template< class _Rep, class _Period >
JsonGeneric::Ptr operator << ( const Json &, const std::chrono::duration< _Rep, _Period > & _data )
{
	return Json{} << _data.count();
}


//! \brief  std::chrono deserialization
//! \tparam		_Rep		time duration representation (will be automatically deduced)
//! \tparam		_Period		period of time (will be automatically deduced)
//! \param[in,out]	_data	a reference over the C++ std::chrono to deserialize to
//! \return		a reference over the input Json Generic
template< class _Rep, class _Period >
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::chrono::duration< _Rep, _Period > & _data )
{
	_Rep count;
	_generic >> count;
	_data = std::chrono::duration< _Rep, _Period >{ count };
	return _generic;
}


} // Moth