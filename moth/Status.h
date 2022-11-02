#pragma once

// moth:
#include "Ctti.h"

// std library:
#include "StdIncludes.h"


namespace Moth {


// -----------------
//! \brief	Generic status object used for error management
class Status
{
	//! \brief	the content is accessible by Json for serialization purpose
	//! \remark	cannot access JsonFriendship( Status ); here
	friend std::unique_ptr< struct JsonGeneric > operator << ( const struct Json &, const Status & );
	friend const JsonGeneric & operator >> ( const JsonGeneric &, Status & );

// ctor/dtors
public:
	// default neutral status (placeholder for move):
	Status() = default;

	// proper ctor:
	Status( const bool _success, std::wstring && _identifier, const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, std::wstring && _message );

	//! \brief dtor: verify the status was previously disarmed
	~Status();

	// move:
	void Swap( Status && _a, Status && _b );
	Status( Status && _other );
	Status && operator =( Status && _other );

// public methods
public:
	//! \brief	disarm the current status (better be checked anyway...)
	void Disarm() const;

	//! \return	true if it's a success status, false if it's a failure
	explicit operator bool() const;

	//! \brief	create a success status (used by the StatusSuccess macro)
	template< typename _Type, typename... _Args >
	static Status Success( const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, _Args&&... _args );

	//! \brief	create a failure status (used by the StatusFailure macro)
	template< typename _Type, typename... _Args >
	static Status Failure( const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, _Args&&... _args );

	//! \brief	generic success used by the StatusOk macro
	struct GenericSuccess;

	//! \brief	stacking operator
	Status && operator <<( Status && _other );

	//! \brief	dump the status
	//! \param[ in ]	_showOrigin		dump the creation origin (file, line and function name) of the status
	//! \param[ in ]	_dumpChildren	dump also the stacked status children if any
	void Dump( const bool _showOrigin = false, const bool _dumpChildren = true ) const;

	const std::wstring & Identifier() const;
	explicit operator std::wstring() const { return m_identifier; }

	//! \return true if the status is of the given type
	bool Is( const std::wstring & _identifier ) const;

	//! \return true if the status is of the given type
	template< typename _Type >
	bool Is() const { return Is( std::wstring{ Ctti::ShortName< _Type >() } ); }

	// comparison operators:
	bool operator ==( const Status & _other ) const { return Is( _other.m_identifier ); }
	bool operator !=( const Status & _other ) const { return !operator ==( _other ); }

	//! \return	true if the status contains a given type
	bool Contains( const std::wstring & _identifier ) const;

	//! \return	true if the status contains a given type
	template< typename _Type >
	bool Contains() const { return Contains( std::wstring{ Ctti::ShortName< _Type >() } ); }

	//! \brief allows to return a status and an object tied together
	template< typename _ObjectType >
	struct And;

// internal methods:
private:
	//! \brief	create a status (can be both a success or failure)
	template< typename _Type, typename... _Args >
	static Status _Status( const bool _success, const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, _Args&&... _args );

	//! \brief	create a status (can be both a success or failure), non-templated call:
	static Status _CreateStatus( const bool _success, std::wstring && _type, const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, std::wstring && _message, std::list< std::wstring > && _arguments );

// members:
private:
	mutable bool m_armed{ false };
	mutable bool m_success{ false };
	mutable std::wstring m_identifier;
	mutable std::wstring m_originFile;
	mutable unsigned long long m_originLine;
	mutable std::wstring m_originFunction;
	mutable std::wstring m_message;
	std::unique_ptr< Status > m_child;
};


template< typename _Type, typename... _Args >
Status Status::Success( const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, _Args&&... _args )
{
	return _Status< _Type >( true, _originFile, _originLine, _originFunction, std::forward< _Args >( _args )... );
}


template< typename _Type, typename... _Args >
Status Status::Failure( const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, _Args&&... _args )
{
	return _Status< _Type >( false, _originFile, _originLine, _originFunction, std::forward< _Args >( _args )... );
}


struct Status::GenericSuccess
{
	static constexpr const wchar_t * message{ L"ok" };
};


template< typename _ObjectType >
struct Status::And
{
	Status status;
	std::optional< _ObjectType > object;

	explicit operator bool() const { return static_cast< bool >( status ); }
	operator Status &&() { return std::move( status ); }
	void Disarm() const { status.Disarm(); }
};


template< typename _Type, typename... _Args >
Status Status::_Status( const bool _success, const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, _Args&&... _args )
{
	constexpr const wchar_t * messageCtti{ _Type::message };
	return _CreateStatus( _success, System::MbcsToWide( std::string{ Ctti::ShortName< _Type >() } ), _originFile, _originLine, _originFunction, std::wstring{ messageCtti }, std::list< std::wstring >{ std::forward< _Args >( _args )... } );
}


} // Moth


//! \brief  macros exposition
#include "StatusMacros.h"