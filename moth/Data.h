#pragma once

// moth:
#include "AnsiToWideMacros.h"
#include "System.h"

// std library:
#include "StdIncludes.h"


namespace Moth {


// -----------------
//! \brief		Generic data interface
class IData
{
	// can be destroyed through this interface when stored in a unique pointer
	friend std::unique_ptr< IData >::deleter_type;

// ctor/dtor:
protected:
	//! \brief		default ctor to be called by inherited class
	IData() = default;
	
	//! \brief		dtor that will be called through unique pointer destruction
	virtual ~IData() = default;

// public interface methods:
public:
	//! \brief		serialize a single data
	//! \return		the serialized data as a string
	virtual std::wstring Serialize() const = 0;
	
	//! \brief		de-serialize a single data
	//! \para[in]	_data	the string to de-serialize the data from
	virtual void DeSerialize( const std::wstring & _data ) = 0;
};


// -----------------
//! \brief		Specialized data object
//! \tparam		_Type	the data type
template< typename _Type >
class Data
	: public IData
{
	// Data is accessible by DataPool
	friend class DataPool;

// ctor/dtor:
private:
	//! \brief		ctor
	//! \param[in]	_data		the initial data to set
	//! \param[in]	_onUpdate	update method to be called when setting a data value
	Data( _Type && _data, std::function< void( const _Type & _data ) > && _onUpdate );

	//! \brief		dtor
	~Data() = default;

// public methods:
public:
	//! \brief		update the current data value
	//! \param[in]	_data	the data used to replace the current value
	void Set( _Type && _data );
	
	//! \return		a constant reference over the current data value
	const _Type & Get() const;

// IData:
private:
	std::wstring Serialize() const final;
	void DeSerialize( const std::wstring & _data ) final;

// members:
private:
	_Type m_data; //!< the data itself
	std::function< void( const _Type & _data ) > m_onUpdate; //!< update method
};


// -----------------
//! \brief		A pool of multiple registered data
class DataPool
{
// ctor/dtor
public:
	//! \brief		default ctor
	DataPool() = default;

	//! \brief		dctor
	~DataPool() = default;
	

// public methods:
public:
	//! \brief		register a new data in the poll
	//! \tparam		_Type		the data type to register
	//! \tparam		_Args		variadic parameters used to init the data
	//! \param[in]	_name		the data name to register
	//! \param[in]	_onUpdate	update method to be called when setting a data value
	//! \param[in]	_args		data creation variadic arguments
	//! \warning	can raise an assertion if the data was already registered
	template< typename _Type, typename... _Args >
	void Register( const std::wstring & _name, std::function< void( const _Type & _data ) > && _onUpdate, _Args&&... _args );
	
	//! \brief		update a given data value
	//! \tparam		_Type		the data type to update
	//! \tparam		_Args		variadic parameters used to create the data
	//! \param[in]	_name		the data name to update
	//! \param[in]	_args		data creation variadic arguments
	//! \warning	can raise an assertion if the data is unknown
	template< typename _Type, typename... _Args >
	void Set( const std::wstring & _name, _Args&&... _args );
	
	//! \return		a constant reference over the current value of a data
	//! \tparam		_Type		the data type to get
	//! \param[in]	_name		the data name to get
	//! \warning	can raise an assertion if the data is unknown
	template< typename _Type >
	const _Type & Get( const std::wstring & _name ) const;
	

// protected methods:
protected:
	//! \brief		serialize the pool of data
	//! \return		the serialized data pool as a string
	std::wstring Serialize() const;
	
	//! \brief		de-serialize a pool of data
	//! \para[in,out]	_data	the string to de-serialize the data pool from
	//! \remark			input data is consumed by the de-serialization algorithm
	//! \warning		can raise an assertion if the expected de-serialized name is unconsistent
	void DeSerialize( std::wstring & _data );
	

// members:
private:
	std::map< std::wstring, std::unique_ptr< IData > > m_data; //!< the pool of data
};


// -----------------
//! \brief		Usage macros definitions

//! \brief	easy data pool registration
#define DataPoolRegister( _Object, _Type, _Name, ... )								\
	static_cast< DataPool & >( _Object ).Register< _Type >( A2W( #_Name ), __VA_ARGS__ )


//! \brief	easy data pool accessors definition
#define DataPoolAccess( _Object, _Type, _Name, _Getter, _Setter )					\
	const _Type & _Getter() const													\
	{																				\
		return static_cast< const DataPool & >( _Object ).Get< _Type >( A2W( #_Name ) );	\
	}																				\
																					\
	void _Setter( const _Type & _data )												\
	{																				\
		static_cast< DataPool & >( _Object ).Set< _Type >( A2W( #_Name ), _data );			\
	}


// -----------------
//! \brief		templated methods definition
#include "Data.hpp"


} // Moth