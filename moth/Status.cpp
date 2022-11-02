#include "Status.h"

// moth:
#include "System.h"


namespace Moth {


Status::Status( const bool _success, std::wstring && _identifier, const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, std::wstring && _message )
	: m_armed{ true }
	, m_success{ _success }
	, m_identifier{ std::move( _identifier ) }
	, m_originFile{ System::MbcsToWide( _originFile ) }
	, m_originLine{ _originLine }
	, m_originFunction{ System::MbcsToWide( _originFunction ) }
	, m_message{ std::move( _message ) }
{}


Status::~Status()
{
	if( !m_armed )
		return;

	// dump status stack:
	System::Output( L"STATUS NOT DISARMED !", System::eOutputType::error );
	System::Output( L"> Status stack:" );
	Dump( true, false );

	// abort execution:
	std::abort();
}


void Status::Swap( Status && _a, Status && _b )
{
	std::swap( _a.m_armed, _b.m_armed );
	std::swap( _a.m_success, _b.m_success );
	std::swap( _a.m_identifier, _b.m_identifier );
	std::swap( _a.m_originFile, _b.m_originFile );
	std::swap( _a.m_originLine, _b.m_originLine );
	std::swap( _a.m_originFunction, _b.m_originFunction );
	std::swap( _a.m_message, _b.m_message );
	std::swap( _a.m_child, _b.m_child );
}


Status::Status( Status && _other )
{
	Swap( std::move( *this ), std::move( _other ) );
}


Status && Status::operator =( Status && _other )
{
	Swap( std::move( *this ), std::move( _other ) );
	return std::move( *this );
}


void Status::Disarm() const
{
	m_armed = false;
}


Status::operator bool() const
{
	// checking the status disarm it:
	Disarm();
	return m_success;
}


Status && Status::operator <<( Status && _other )
{
	System::Assert( !m_child, L"a child status was already stacked" );

	// stacking the status disarm it:
	_other.Disarm();
	m_child.reset( new Status{ std::move( _other ) } );
	return std::move( *this );
}


void Status::Dump( const bool _showOrigin, const bool _dumpChildren ) const
{
	System::Output( L"\t" + std::wstring{ m_success ? L"success" : L"FAILURE" } + L": [" + m_identifier +  L"] " + m_message );
	if( _showOrigin )
		System::Output( L"\t[" + m_originFunction + L"] " + m_originFile + L"(" + std::to_wstring( m_originLine ) + L")" );
	if( !_dumpChildren )
		return;
	if( m_child )
		m_child->Dump( _showOrigin );
}


bool Status::Is( const std::wstring & _identifier ) const
{
	// checking the status disarm it:
	Disarm();
	return m_identifier == _identifier;
}


bool Status::Contains( const std::wstring & _identifier ) const
{
	if( Is( _identifier ) )
		return true;
	if( m_child && m_child->Contains( _identifier ) )
		return true;
	return false;
}

const std::wstring & Status::Identifier() const
{
    return m_identifier;
}

Status Status::_CreateStatus( const bool _success, std::wstring && _type, const std::string & _originFile, const unsigned long long _originLine, const std::string & _originFunction, std::wstring && _message, std::list< std::wstring > && _arguments )
{
	const std::wstring token{ L"%" };
	for( auto && argument : _arguments ) {
		const auto tokenPosition{ _message.find( token ) };
		System::Assert( tokenPosition != std::wstring::npos, L"too many arguments regarding message's token count" );
		_message.replace( tokenPosition, token.length(), std::move( argument ) );
	}
	return Status{ _success, std::move( _type ), _originFile, _originLine, _originFunction, std::move( _message ) };
}


} // Moth