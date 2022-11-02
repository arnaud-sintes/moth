#include "System.h"


namespace Moth {


std::wstring System::GenerateId()
{
	static std::atomic< int > id{ 0 };
	return L"id" + std::to_wstring( id++ );
}


void System::Output( const std::wstring & _message, const eOutputType _type )
{
	switch( _type ) {
		case eOutputType::info:
			Js::Console::Log( _message );
			break;
		case eOutputType::warning:
			Js::Console::Warning( _message );
			break;
		case eOutputType::error:
			Js::Console::Error( _message );
			break;
	}
}


void System::Assert( const bool _condition, const std::wstring & _message )
{
	if( _condition )
		return;
	Output( L"[ASSERTION FAILED] " + _message, eOutputType::error );
	std::abort();
}


std::wstring System::MbcsToWide( const std::string & _utf8 )
{
    static std::wstring_convert< std::codecvt_utf8< wchar_t >, wchar_t > converter;
    return converter.from_bytes( _utf8 );
}


std::string System::WideToMbcs( const std::wstring & _utf16 )
{
    static std::wstring_convert< std::codecvt_utf8< wchar_t >,  wchar_t > converter;
    return converter.to_bytes( _utf16 );
}


System::Position System::OffsetToPosition( const std::wstring & _data,const unsigned long long _offset )
{
    Assert( _offset < _data.size(), L"offset outside data boundaries" );
    auto it{ _data.cbegin() };
    unsigned long long offset{ 0 };
    Position position;
    for( ; offset != _offset; offset++, ++it ) {
        if( *it == L'\r' )
            continue;
        if( *it != L'\n' ) {
            position.column++;
            continue;
        }
        position.row++;
        position.column = 1;
    }
    return position;
}


std::wstring System::ExtractData( std::wstring & _data )
{
	const auto commaPos{ _data.find( L"," ) };
	const auto data{ _data.substr( 0, commaPos ) };
	_data = _data.substr( commaPos + 1, _data.length() - commaPos - 1 );
	return data;
}


} // Moth