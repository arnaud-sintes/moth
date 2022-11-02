#pragma once

// std library:
#include "StdIncludes.h"


namespace Moth {


struct System
{
    //! \return		a unique identifier as a string
	static std::wstring GenerateId();

	//! \brief		possible types of outputs:
	enum class eOutputType
	{
		info,		// informative
		warning,	// warning
		error,		// error
	};

	//! \brief		output a message to the JavaScript console
	//! \param[in]	_message	the message to write
	//! \param[in]	_type		[optional] the type of message (default: info)
	static void Output( const std::wstring & _message, const eOutputType _type = eOutputType::info );

	//! \brief		assert for a given condition
	//! \details	log an error in the JavaScript console in case of failure and abort execution
	//! \param[in]	_condition	the assertion condition to check
	//! \param[in]	_message	the error message to log in case of failure
	static void Assert( const bool _condition, const std::wstring & _message );

	//! \brief	Multibyte to Wide strings conversion
	static std::wstring MbcsToWide( const std::string & _utf8 );

	//! \brief	Wide to Multibyte strings conversion
	static std::string WideToMbcs( const std::wstring & _utf16 );

    struct Position
	{
		unsigned long long row{ 1 };
		unsigned long long column{ 1 };
	};

    //! \brief	convert an offset in a text file to row/column positionning
	static Position OffsetToPosition( const std::wstring & _data, const unsigned long long _offset );

	//! \brief		extract a single data from a string by consumption
	//! \param[in]	_data		the data to extract
	//! \remark		input data is consumed
	//! \return		the extracted data
	static std::wstring ExtractData( std::wstring & _data );
};


} // Moth
