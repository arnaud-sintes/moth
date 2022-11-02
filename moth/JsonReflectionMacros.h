//! \brief		unique reflection macro preprocessor definitions
#define json_reflection_Separator()						;
#define json_reflection_Fnc_Serialize( _DATA )			*object << ::Moth::Json::Pair< decltype( _data._DATA ) >{ L""#_DATA, _data._DATA }
#define json_reflection_Fnc_DeSerialize( _DATA )		object >> ::Moth::Json::Pair< decltype( _data._DATA ) >{ L""#_DATA, _data._DATA }
#define json_reflection_Expander_Serialize( ... )		Expander( json_reflection_Separator, json_reflection_Fnc_Serialize, __VA_ARGS__ )
#define json_reflection_Expander_DeSerialize( ... )		Expander( json_reflection_Separator, json_reflection_Fnc_DeSerialize, __VA_ARGS__ )


//! \brief		unique reflection macro to be used by custom structures
#define JsonReflection( _Type, ... )																						\
																															\
	::Moth::JsonGeneric::Ptr operator << ( const ::Moth::Json &, const _Type & _data )										\
	{																														\
		::Moth::Json::Object::Ptr object;																					\
		json_reflection_Expander_Serialize( __VA_ARGS__ );																	\
		return object;																										\
	}																														\
																															\
	const ::Moth::JsonGeneric & operator >> ( const ::Moth::JsonGeneric & _generic, _Type & _data )							\
	{																														\
		const auto & object{ _generic.As< ::Moth::Json::Object >() };														\
		json_reflection_Expander_DeSerialize( __VA_ARGS__ );																\
		return _generic;																									\
	}


//! \brief		unique reflection macro to be used by custom structures, when empty
#define JsonReflectionEmpty( _Type )																						\
																															\
	::Moth::JsonGeneric::Ptr operator << ( const ::Moth::Json &, const _Type & /*_data*/ )									\
	{																														\
		return ::Moth::Json::Object::Ptr{};																					\
	}																														\
																															\
	const ::Moth::JsonGeneric & operator >> ( const ::Moth::JsonGeneric & _generic, _Type & /*_data*/ )						\
	{																														\
		return _generic;																									\
	}


//! \brief		unique reflection macro to be used by custom templated structures
#define JsonReflectionT( _Type, ... )																						\
																															\
	template< typename _T >																									\
	::Moth::JsonGeneric::Ptr operator << ( const ::Moth::Json &, const _Type< _T > & _data )								\
	{																														\
		::Moth::Json::Object::Ptr object;																					\
		json_reflection_Expander_Serialize( __VA_ARGS__ );																	\
		return object;																										\
	}																														\
																															\
	template< typename _T >																									\
	const ::Moth::JsonGeneric & operator >> ( const ::Moth::JsonGeneric & _generic, _Type< _T > & _data )					\
	{																														\
		const auto & object{ _generic.As< ::Moth::Json::Object >() };														\
		json_reflection_Expander_DeSerialize( __VA_ARGS__ );																\
		return _generic;																									\
	}


//! \brief		unique reflection macro preprocessor definitions, to deal with pair of variable names
#define json_reflection_pair_Separator()						;
#define json_reflection_pair_Fnc_Serialize( _DATA1, _DATA2 )	*object << ::Moth::Json::Pair< decltype( _data._DATA1 ) >{ L""#_DATA2, _data._DATA1 }
#define json_reflection_pair_Fnc_DeSerialize( _DATA1, _DATA2 )	object >> ::Moth::Json::Pair< decltype( _data._DATA1 ) >{ L""#_DATA2, _data._DATA1 }
#define json_reflection_pair_Expander_Serialize( ... )			ExpanderPair( json_reflection_pair_Separator, json_reflection_pair_Fnc_Serialize, __VA_ARGS__ )
#define json_reflection_pair_Expander_DeSerialize( ... )		ExpanderPair( json_reflection_pair_Separator, json_reflection_pair_Fnc_DeSerialize, __VA_ARGS__ )


//! \brief		unique reflection macro to be used by custom structures
//! \note		use this one when you want to associate a different JSON storage name regarding the C++ variable name
#define JsonReflectionPair( _Type, ... )																					\
																															\
	::Moth::JsonGeneric::Ptr operator << ( const ::Moth::Json &, const _Type & _data )										\
	{																														\
		::Moth::Json::Object::Ptr object;																					\
		json_reflection_pair_Expander_Serialize( __VA_ARGS__ );																\
		return object;																										\
	}																														\
																															\
	const ::Moth::JsonGeneric & operator >> ( const ::Moth::JsonGeneric & _generic, _Type & _data )							\
	{																														\
		const auto & object{ _generic.As< ::Moth::Json::Object >() };														\
		json_reflection_pair_Expander_DeSerialize( __VA_ARGS__ );															\
		return _generic;																									\
	}



//! \brief		frienship with Json required to deal with private parts of structures
#define JsonFriendship( _Type )																								\
	friend std::unique_ptr< ::Moth::JsonGeneric > operator << ( const ::Moth::Json &, const _Type & );						\
	friend const ::Moth::JsonGeneric & operator >> ( const ::Moth::JsonGeneric & _generic, _Type & )
