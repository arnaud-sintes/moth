#pragma once

// moth:
#include "IndexedMap.h"
#include "Status.h"
#include "System.h"

// std library:
#include "StdIncludes.h"


namespace Moth {


// parse error statuses:
DeclStatus( FailToParse, L"Fail to parse content: '%' around row %, col %" );
DeclStatus( FailToParseUncatched, L"Downcast unacceptable (bad target type) or parsing failure due to uncatched cause" );

// path error statuses:
DeclStatus( PathErrorUnknownObjectIdentifier, L"Uknown object identifier '%'" );
DeclStatus( PathErrorOutOfArrayBoundaries, L"Index % is out of array boundaries containing % elements(s)" );
DeclStatus( PathErrorMissingOpenSquareBracket, L"Open square bracket is missing" );
DeclStatus( PathErrorUncatched, L"Downcast unacceptable (bad target type) or path parsing failure due to uncatched cause" );



// ----------
//! \brief		Generic neutral interface for Json types
//! \details	This is the motherclass of any specifc Json type, providing downcast conversions and stringification interface
//! \remark		note it's not a sub-class of Json to allow forward declaration in foreign structures
struct JsonGeneric
{
	//! \brief		Notation shortcut when dealing with Generic pointers
	using Ptr = std::unique_ptr< JsonGeneric >;

	//! \brief		virtual destructor
	virtual ~JsonGeneric() = default;

	//! \brief		Downcast conversion from the current Generic type to a specific Json type
	//! \tparam		_JsonType		specific Json type target for downcast
	//! \return		a reference over the Json type downcasted type
	//! \warning	an assertion (bad cast) will occurs if we're using a bad downcast target
	template< typename _JsonType >
	const _JsonType & As() const;

	//! \brief		Stringification interface
	//! \param[in]	_tab		an optional String to use to tabulate the content of the stringified Json data
	//! \remark		if _tab is left empty, the result of the stringification process will be a very compact String
	//! \param[in]	_depth		the current tabulation depth (for internal usage only)
	//! \return		the stringified Json data for the current Json type represented implicitely by the Generic
	virtual std::wstring Stringify( const std::wstring & _tab = L"", const unsigned long long _depth = 0 ) const = 0;
};


// ----------
//! \brief		Json (https://en.wikipedia.org/wiki/Json) manipulation
//! \details	Defines basic Json types (Null, Number, Boolean, String, Array & Object), all inheriting from a specific intermediate Storage class,
//!				this Storage class inheriting from a Generic neutral interface.
//!				Manipulation of Json types occurs directly through the usage of a Generic or via the creation of a Generic type pointer, shortened Generic::Ptr.
//!				Specific Json types pointers are automatically defines via the Storage class, allowing notations like Boolean::Ptr, Object::Ptr...
struct Json final
{
	//! \brief		Specific Json type pointer class (forward declaration)
	template< typename _JsonType, typename _Type >
	struct Ptr;

	//! \brief		Specific intermediate Json type Storage class (forward declaration)
	template< typename _JsonType, typename _RestrictedType >
	struct Storage;

	//! \brief		Null Json type (forward declaration)
	struct Null;

	//! \brief		Number Json type (forward declaration)
	struct Number;

	//! \brief		Boolean Json type (forward declaration)
	struct Boolean;

	//! \brief		String Json type (forward declaration)
	struct String;

	//! \brief		Array Json type (forward declaration)
	struct Array;

	//! \brief		Object Json type (forward declaration)
	struct Object;

	//! \brief		Pair is used to give access to Object's data, requiring then a systematic data name identifier (forward declaration)
	template< typename _Type >
	struct Pair;
};


// ----------
//! \brief		we need to separate static functions, for a fucking UNKNOWN REASON
struct JSON
{
	//! \brief		Checks at compile time if the type is a valid Json type (Null, Number, Boolean, String, Array or Object)
	//!	\tparam		_Type		the type to check (will be automatically deduced)
	//! \return		true if it's a regular Json type (used for static assertions)
	template< typename _Type >
	static constexpr bool IsJsonType();

	//! \brief		Checks at compile time if the type provided is a valid Json data Storage candidate
	//!	\tparam		_Type		the type to check (will be automatically deduced)
	//! \return		true if it's a regular Json data Storage candidate (used for static assertions)
	template< typename _Type >
	static constexpr bool IsRestrictedType();

	//! \brief		Checks at compile time if the C++ type provided is compatible with a Json Number Storage
	//!	\tparam		_Type		the type to check (will be automatically deduced)
	//! \return		true if it's a regular Json Number Storage type (used for static assertions)
	template< typename _Type >
	static constexpr bool IsNumberType();

	//! \brief		Converts a Generic pointer to a specific Json type (if the downcast is acceptable)
	//!	\tparam		_JsonType		the Json type to convert the Generic pointer to
	//!	\param[in]	_genericPtr		a rvalue reference over a Generic pointer to convert from
	//! \return		a proper Json type
	//! \warning	an assertion will occurs (bad cast) if we're using a bad downcast target
	template< typename _JsonType >
	static _JsonType As( JsonGeneric::Ptr && _genericPtr );

	//! \brief		Generic serialization of a C++ Object into a Json type, if acceptable
	//!	\tparam		_JsonType		the Json type to serialize the C++ Object to
	//!	\tparam		_Type			the source C++ Object type to serialize (will be automatically deduced)
	//!	\param[in]	_data			a reference over the C++ Object to serialize
	//! \return		a proper Json type
	template< typename _JsonType, typename _Type >
	static _JsonType Serialize( const _Type & _data );

	//! \brief		Generic deserialization of a Json type into a C++ Object, if acceptable
	//!	\tparam			_Type			the C++ Object target type for deserialization
	//!	\param[in]		_generic		a reference over the Generic type to deserialize
	//! \return			a proper C++ Object
	template< typename _Type >
	static _Type DeSerialize( const JsonGeneric & _generic );

	//! \brief		Generic String parsing with an explicit conversion to a Json type, if acceptable
	//!	\tparam		_JsonType		the Json type to return at the end of the parsing process
	//!	\param[in]	_data			the String data to parse
	//! \return		a proper Json type if no error
	template< typename _JsonType >
	static Status::And< _JsonType > Parse( const std::wstring & _data );

	//! \brief		Generic String parsing to a Generic pointer
	//!	\param[in]	_data			the String data to parse
	//! \return		a proper Json Generic pointer if no error
	static Status::And< JsonGeneric::Ptr > Parse( const std::wstring & _data );

	//! \brief		Generic JsonPath expression application to find a Json type and turn it into a C++ Object, if acceptable
	//!	\tparam		_Type			the C++ Object target type for deserialization
	//!	\param[in]	_generic		a reference over the Generic type to search
	//!	\param[in]	_expression		the JsonPath expression to apply
	//! \remark		Please note only the following operators are currently supported in the JsonPath expression:
	//!					$			: root element to query
	//!					.name		: dot-notated child
	//!					['name']	: bracket-notated child
	//!					[Number]	: Array index
	//!				functions and filter operators are not supported yet, nor the following operators:
	//!					@			: the current node being processed by a filter predicate
	//!					*			: wildcard
	//!					..			: deep scan
	//!					['name',..]	: bracket-notated children
	//!					[Number,..]	: Array indexes
	//!					[start:end]	: Array slice operator
	//!					[?(exp)]	: filter Boolean expression
	//! \return			a proper C++ Object resulting from the JsonPath expression application
	template< typename _Type >
	static Status::And< _Type > Path( const JsonGeneric & _generic, const std::wstring & _expression );

	//! \brief		Generic JsonPath expression application to find a Generic pointer
	//!	\param[in]	_generic		a reference over the Generic type to search
	//!	\param[in]	_expression		the JsonPath expression to apply
	//! \remark		this method is recursive
	//! \return		a Json Generic reference resulting from the JsonPath expression application
	static Status::And< std::reference_wrapper< const JsonGeneric > > Path( const JsonGeneric & _generic, std::wstring _expression );
};


template< typename _JsonType >
const _JsonType & JsonGeneric::As() const
{
	static_assert( JSON::IsJsonType< _JsonType >(), "_JsonType is not valid Json type" );
	return dynamic_cast< const _JsonType & >( *this );
}


// ----------
//! \brief	Specific Json type pointer class, with pointer Storage, data access and Generic::Ptr conversion
//! \tparam		_JsonType	Json type (Null, Number, Boolean, String, Array or Object)
//! \tparam		_Type		any data type that will be transferred to the Json type for its creation via pointer
template< typename _JsonType, typename _Type >
struct Json::Ptr final
{
	//! \brief	the specific Json Storage pointer
	std::unique_ptr< _JsonType > ptr;

	//! \brief	default constructor, will create a pointer to a Json type created with default parameter (ok for Null, Array & Object)
	Ptr()
		: ptr{ new _JsonType }
	{}

	//! \brief	construct a Json type from C++ data
	//! \param[in]		_data	the C++ data to provide for Json type construction (Boolean, Number & String)
	Ptr( const _Type & _data )
		: ptr{ new _JsonType{ _data } }
	{}

	//! \return		a reference over the Json type contained in the pointer
	_JsonType & operator *() { return *ptr; }

	//! \return		a Generic pointer from the current Json type pointer (upcast)
	operator JsonGeneric::Ptr() { return std::move( ptr ); }
};


// ----------
//! \brief		Specific intermediate Json type Storage class
//! \tparam		_JsonType			Json type (Null, Number, Boolean, String, Array or Object)
//! \tparam		_RestrictedType		the C++ restricted type used for the data Storage
template< typename _JsonType, typename _RestrictedType >
struct Json::Storage
	: JsonGeneric
{
	//! \brief		virtual destructor
	virtual ~Storage() = default;

	//! \brief		notation shortcut to create specific Json type pointers
	using Ptr = Json::Ptr< _JsonType, _RestrictedType >;

	//! \brief	the specific data Storage
	_RestrictedType data{};

	//! \brief	default constructor, will create a Storage type using default parameter (ok for Null, Array & Object)
	Storage()
	{
		static_assert( JSON::IsRestrictedType< _RestrictedType >(), "_RestrictedType is not compatible with Json storages" );
	}

	//! \brief	construct a Storage from C++ data
	//! \param[in]		_data	the C++ data to provide for Json type construction (Boolean, Number & String)
	Storage( const _RestrictedType & _data )
		: data{ _data }
	{
		static_assert( JSON::IsRestrictedType< _RestrictedType >(), "_RestrictedType is not compatible with Json storages" );
	}

	//! \brief	move constructor
	//! \param[in]		_other	a rvalue reference over the Storage to swap from
	Storage( Storage && _other ) noexcept
		: data{ std::move( _other.data ) }
	{}
};


// ----------
//! \brief		Null Json type
//! \details	Null is a specific Json type used to represent empty values
struct Json::Null final
	: Storage< Null, void * >
{
	//! \brief		default constructor
	//! \remark		there's no need to initialize the Storage here, the void * being just a placeholder
	Null() = default;

	//! \brief		Generic Stringification interface override
	std::wstring Stringify( const std::wstring & _tab, const unsigned long long _depth ) const final;
};


// ----------
//! \brief		Number Json type
//! \details	Number is a specific Json type used to represent signed decimal Number that may contain a fractional part
//! \remark		a Number cannot include non-numbers such as NaN and there's no distinction between integer and floating-point
//!				Number may also use exponential E notation
struct Json::Number final
	: Storage< Number, double >
{
	//! \brief		constructor of a Json Number
	//! \param[in]	_data	the double-precision floating-point formatted value used to create the Number Json type
	Number( const double & _data )
		: Storage{ _data }
	{}

	//! \brief		Generic Stringification interface override
	std::wstring Stringify( const std::wstring & _tab, const unsigned long long _depth ) const final;
};


// ----------
//! \brief		Boolean Json type
struct Json::Boolean final
	: Storage< Boolean, bool >
{
	//! \brief		constructor of a Json Boolean
	//! \param[in]	_data	the Boolean value used to create the Boolean Json type
	Boolean( const bool & _data )
		: Storage{ _data }
	{}

	//! \brief		Generic Stringification interface override
	std::wstring Stringify( const std::wstring & _tab, const unsigned long long _depth ) const final;
};


// ----------
//! \brief		String Json type
//! \details	String is a specific Json type that is a sequence of zero or more Unicode characters
//! \remark		String supports backslah escaping syntax and use double-quote delimiters representation
struct Json::String final
	: Storage< String, std::wstring >
{
	//! \brief		constructor of a Json String
	//! \param[in]	_data	the UCS-2 String value used to create the String Json type
	String( const std::wstring & _data )
		: Storage{ _data }
	{}

	//! \brief		Generic Stringification interface override
	std::wstring Stringify( const std::wstring & _tab, const unsigned long long _depth ) const final;
};


// ----------
//! \brief		Array Json type
//! \details	Array is a specific Json type under the form of an ordered list of zero or more elements, each of which may be of any type
//! \remark		arrays use square bracket notation with comma-separated elements
struct Json::Array final
	: Storage< Array, std::list< JsonGeneric::Ptr > >
{
	//! \brief		the iterator that will be used for deserialization process
	std::unique_ptr< std::list< JsonGeneric::Ptr >::const_iterator > pitDeserialize;

	//! \brief		default constructor
	//! \remark		by default the Array is empty
	Array() = default;

	//! \brief		Specific serialization of a C++ Object into a Json Array
	//!	\tparam		_Type			the source C++ Object type to serialize (will be automatically deduced)
	//!	\param[in]	_data			a reference over the C++ Object to serialize
	//! \remark		of course _data better be a container like a std::list<>, std::vector<> ...
	//! \return		a proper Json Array
	template< typename _Type >
	static Array Serialize( const _Type & _data ) { return JSON::Serialize< Array >( _data ); }

	//! \brief		Specific deserialization of the current Json Array into a C++ Object
	//!	\tparam			_Type			the C++ Object target type for deserialization
	//! \remark		of course _Type better be a container like a std::list<>, std::vector<> ...
	//! \return			a proper C++ Object
	template< typename _Type >
	_Type DeSerialize() const { return JSON::DeSerialize< _Type >( static_cast< const JsonGeneric & >( *this ) ); }

	//! \brief		Generic Stringification interface override
	std::wstring Stringify( const std::wstring & _tab = L"", const unsigned long long _depth = 0 ) const final;

	//! \brief		Specific String parsing with an explicit conversion to a Json Array
	//!	\param[in]	_data			the String data to parse
	//! \return		a proper Json Array
	static Status::And< Array > Parse( const std::wstring & _data ) { return JSON::Parse< Array >( _data ); }

	//! \brief		Specific JsonPath expression application to find a Json type and turn it into a C++ Object, if acceptable
	//!	\tparam		_Type			the C++ Object target type for deserialization
	//!	\param[in]	_expression		the JsonPath expression to apply
	//! \remark		of course _Type better be a container like a std::list<>, std::vector<> ...
	//! \return			a proper C++ Object
	template< typename _Type >
	Status::And< _Type > Path( const std::wstring & _expression ) const { return JSON::Path< _Type >( static_cast< const JsonGeneric & >( *this ), _expression ); }
};


// ----------
//! \brief		Object Json type
//! \details	Object is a specific Json type  which is a a collection of name�value pairs where the names (also called keys) are strings
//!				objects are intended to represent associative arrays where each key is unique within an Object
//!				objects are delimited with curly brackets and use commas to separate each Pair, while within each Pair the colon ':' character separates the key or name from its value
struct Json::Object final
	: Storage< Object, IndexedMap< std::wstring, JsonGeneric::Ptr > >
{
	//! \brief		default constructor
	//! \remark		by default the Object is empty
	Object() = default;

	//! \brief		Specific serialization of a C++ Object into a Json Object
	//!	\tparam		_Type			the source C++ Object type to serialize (will be automatically deduced)
	//!	\param[in]	_data			a reference over the C++ Object to serialize
	//! \remark		of course _data better be a C++ structure
	//! \return		a proper Json Object
	template< typename _Type >
	static Object Serialize( const _Type & _data ) { return JSON::Serialize< Object >( _data ); }

	//! \brief		Specific deserialization of the current Json Object into a C++ Object
	//!	\tparam			_Type			the C++ Object target type for deserialization
	//! \remark		of course _data better be a C++ structure
	//! \return			a proper C++ Object
	template< typename _Type >
	_Type DeSerialize() const { return JSON::DeSerialize< _Type >( static_cast< const JsonGeneric & >( *this ) ); }

	//! \brief		Generic Stringification interface override
	std::wstring Stringify( const std::wstring & _tab = L"", const unsigned long long _depth = 0 ) const final;

	//! \brief		Specific String parsing with an explicit conversion to a Json Object
	//!	\param[in]	_data			the String data to parse
	//! \return		a proper Json Object
	static Status::And< Object > Parse( const std::wstring & _data ) { return JSON::Parse< Object >( _data ); }

	//! \brief		Specific JsonPath expression application to find a Json type and turn it into a C++ Object, if acceptable
	//!	\tparam		_Type			the C++ Object target type for deserialization
	//!	\param[in]	_expression		the JsonPath expression to apply
	//! \remark		of course _data better be a C++ structure
	//! \return			a proper C++ Object
	template< typename _Type >
	Status::And< _Type > Path( const std::wstring & _expression ) const { return JSON::Path< _Type >( static_cast< const JsonGeneric & >( *this ), _expression ); }
};


// ----------
//! \brief		Pair Object is used to give access to Object's data, requiring then a systematic data name identifier
template< typename _Type >
struct Json::Pair final
{
	//! \brief		the name identifier of the Object's data
	std::wstring name;

	//! \brief		a reference of the Object data
	_Type & data;

	//! \brief		constructor of a Pair
	//! \param[in]	_name	the named identifier of the Object's data
	//! \param[in]	_data	the reference of the Object data
	Pair( const std::wstring & _name, const _Type & _data )
		: name{ _name }
		, data{ const_cast< _Type & >( _data ) }
	{}
};


template< typename _Type >
constexpr bool JSON::IsJsonType()
{
	return	std::is_same< _Type, Json::Null >::value || std::is_same< _Type, Json::Number >::value || std::is_same< _Type, Json::Boolean >::value ||
			std::is_same< _Type, Json::String >::value || std::is_same< _Type, Json::Array >::value || std::is_same< _Type, Json::Object >::value;
}


template< typename _Type >
constexpr bool JSON::IsRestrictedType()
{
	return	std::is_same< _Type, void * >::value ||											// Null
			std::is_same< _Type, double >::value ||											// Number
			std::is_same< _Type, bool >::value ||											// Boolean
			std::is_same< _Type, std::wstring >::value ||									// String
			std::is_same< _Type, std::list< JsonGeneric::Ptr > >::value ||					// Array
			std::is_same< _Type, IndexedMap< std::wstring, JsonGeneric::Ptr > >::value;		// Object
}


template< typename _Type >
constexpr bool JSON::IsNumberType()
{
	return	std::is_same< _Type, char >::value || std::is_same< _Type, unsigned char >::value ||			// 8 bits
			std::is_same< _Type, short >::value || std::is_same< _Type, unsigned short >::value ||			// 16 bits
			std::is_same< _Type, long >::value || std::is_same< _Type, unsigned long >::value ||			// 32 bits
			std::is_same< _Type, long long >::value || std::is_same< _Type, unsigned long long >::value ||	// 64 bits
			std::is_same< _Type, float >::value || std::is_same< _Type, double >::value ||					// real
			std::is_same< _Type, int >::value || std::is_same< _Type, unsigned int >::value ||				// integer
			std::is_same< _Type, int8_t >::value || std::is_same< _Type, wchar_t >::value;					// specific
}


template< typename _JsonType >
_JsonType JSON::As( JsonGeneric::Ptr && _genericPtr )
{
	_JsonType specific;
	// we can safely break the constness here:
	std::swap( specific.data, const_cast< decltype( specific.data ) & >( _genericPtr->As< _JsonType >().data ) );
	return std::move( specific );
}


template< typename _JsonType, typename _Type >
_JsonType JSON::Serialize( const _Type & _data )
{
	return As< _JsonType >( Json{} << _data );
}


template< typename _Type >
_Type JSON::DeSerialize( const JsonGeneric & _generic )
{
	_Type data;
	_generic >> data;
	return std::move( data );
}


template< typename _JsonType >
Status::And< _JsonType > JSON::Parse( const std::wstring & _data )
{
	auto statusAnd{ JSON::Parse( _data ) };
	if( !statusAnd )
		return { std::move( statusAnd.status ) };
	try {
		auto object{ As< _JsonType >( std::move( *statusAnd.object ) ) };
		return { StatusOk, std::move( object ) };
	}
	catch( ... ) {
		return { StatusFailure( FailToParseUncatched ) };
	}
}


template< typename _Type >
Status::And< _Type > JSON::Path( const JsonGeneric & _generic, const std::wstring & _path )
{
	auto statusAnd{ JSON::Path( _generic, _path ) };
	if( !statusAnd )
		return { std::move( statusAnd.status ) };
	try {
		_Type data;
		*statusAnd.object >> data;
		return { StatusOk, std::move( data ) };
	}
	catch( ... ) {
		return { StatusFailure( PathErrorUncatched ) };
	}
}


} // Moth


//! \brief  macros exposition
#include "JsonReflectionMacros.h"

//! \brief	inclusion of Json types serializations
#include "JsonTypes.hpp"

//! \brief	inclusion of std containers serializations
#include "JsonStdContainers.hpp"

//! \brief	inclusion of std objects serializations
#include "JsonStdObjects.hpp"

//! \brief	inclusion of our object
#include "JsonCoreObjects.hpp"