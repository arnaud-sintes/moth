#include "Js.h"

// emscripten:
#include <emscripten.h>


namespace Moth::Js {

	
// -----------------
// Javascript console wrapper

void Console::Log( const std::wstring & _message )
{
	const auto message{ System::WideToMbcs( _message ) };
	EM_ASM( {
		console.log( UTF8ToString( $0 ) );
	}, message.c_str() );
}


void Console::Warning( const std::wstring & _message )
{
	const auto message{ System::WideToMbcs( _message ) };
	EM_ASM( {
		console.warning( UTF8ToString( $0 ) );
	}, message.c_str() );
}


void Console::Error( const std::wstring & _message )
{
	const auto message{ System::WideToMbcs( _message ) };
	EM_ASM( {
		console.error( UTF8ToString( $0 ) );
	}, message.c_str() );
}


// -----------------
// Javascript DOM manipulation

void Dom::CreateElement( const std::wstring & _id, const std::wstring & _type, const std::wstring & _parentId )
{
	const auto id{ System::WideToMbcs( _id ) };
	const auto type{ System::WideToMbcs( _type ) };
	const auto parentId{ System::WideToMbcs( _parentId ) };
	static Initializer instance;
	EM_ASM( {
		const element = document.createElement( UTF8ToString( $1 ) );
		element.id = UTF8ToString( $0 );
		const parentId = UTF8ToString( $2 );
		if( parentId == "" )
			document.body.appendChild( element );
		else
			document.getElementById( parentId ).appendChild( element );
	}, id.c_str(), type.c_str(), parentId.c_str() );
}


void Dom::RemoveElement( const std::wstring & _id )
{
	const auto id{ System::WideToMbcs( _id ) };
	EM_ASM( {
		document.getElementById( UTF8ToString( $0 ) ).remove();
	}, id.c_str() );
}


void Dom::AddClass( const std::wstring & _id, const std::wstring & _className )
{
	const auto id{ System::WideToMbcs( _id ) };
	const auto className{ System::WideToMbcs( _className ) };
	EM_ASM( {
		document.getElementById( UTF8ToString( $0 ) ).classList.add( UTF8ToString( $1 ) );
	}, id.c_str(), className.c_str() );
}


void Dom::RemoveClass( const std::wstring & _id, const std::wstring & _className )
{
	const auto id{ System::WideToMbcs( _id ) };
	const auto className{ System::WideToMbcs( _className ) };
	EM_ASM( {
		document.getElementById( UTF8ToString( $0 ) ).classList.remove( UTF8ToString( $1 ) );
	}, id.c_str(), className.c_str() );
}


void Dom::ToggleClass( const std::wstring & _id, const std::wstring & _className, const bool _active )
{
	if( _active )
		AddClass( _id, _className );
	else
		RemoveClass( _id, _className );
}


void Dom::SetAttribute( const std::wstring & _id, const std::wstring & _attributeName, const std::wstring & _value )
{
	const auto id{ System::WideToMbcs( _id ) };
	const auto attributeName{ System::WideToMbcs( _attributeName ) };
	const auto value{ System::WideToMbcs( _value ) };
	EM_ASM( {
		document.getElementById( UTF8ToString( $0 ) ).setAttribute( UTF8ToString( $1 ), UTF8ToString( $2 ) );
	}, id.c_str(), attributeName.c_str(), value.c_str() );
}


void Dom::RemoveAttribute( const std::wstring & _id, const std::wstring & _attributeName )
{
	const auto id{ System::WideToMbcs( _id ) };
	const auto attributeName{ System::WideToMbcs( _attributeName ) };
	EM_ASM( {
		document.getElementById( UTF8ToString( $0 ) ).removeAttribute( UTF8ToString( $1 ) );
	}, id.c_str(), attributeName.c_str() );
}


void Dom::ToggleAttribute( const std::wstring & _id, const std::wstring & _attributeName, const bool _active )
{
	if( _active )
		SetAttribute( _id, _attributeName );
	else
		RemoveAttribute( _id, _attributeName );
}


void Dom::SetTextContent( const std::wstring & _id, const std::wstring & _textContent )
{
	const auto id{ System::WideToMbcs( _id ) };
	const auto textContent{ System::WideToMbcs( _textContent ) };
	EM_ASM( {
		document.getElementById( UTF8ToString( $0 ) ).textContent = UTF8ToString( $1 );
	}, id.c_str(), textContent.c_str() );
}


void Dom::SetOnClick( const std::wstring & _id, const int _lambdaId )
{
	const auto id{ System::WideToMbcs( _id ) };
	EM_ASM( {
		document.getElementById( UTF8ToString( $0 ) ).onclick = _ => _CallCppLambda( $1, DataToWasm( "{}" ) );
	}, id.c_str(), _lambdaId );
}


void Dom::SetImageSource( const std::wstring & _id, const std::wstring & _data )
{
	const auto id{ System::WideToMbcs( _id ) };
	const auto data{ System::WideToMbcs( _data ) };
	EM_ASM( {
		document.getElementById( UTF8ToString( $0 ) ).src = UTF8ToString( $1 );
	}, id.c_str(), data.c_str() );
}


// -----------------
// Javascript Javascript DOM initializer

Dom::Initializer::Initializer()
{
	EM_ASM( {
		document.title = "moth";
		document.head.appendChild( Object.assign( document.createElement( "link" ),
		{
			rel: "icon",
			href: "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='80'>🦋</text></svg>"
		} ) );
	} );
}

	
// create global DataToWasm JavaScript conversion method:
EM_JS( char *, DataToWasm, ( char * _data ), {
	const dataLen = lengthBytesUTF8( _data ) + 1;
	var dataWasm = _malloc( dataLen ); // requires a call to free
	stringToUTF8( _data, dataWasm, dataLen );
	return dataWasm;
} );


} // Moth::Js