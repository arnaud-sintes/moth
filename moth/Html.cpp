#include "Html.h"

// moth:
#include "Js.h"


namespace Moth::Html {
	
	
// -----------------
// Root HTML element

Element::Element( const std::wstring & _type, const std::optional< std::reference_wrapper< const Item > > & _parent, const std::wstring & _className, const bool _enabled )
	: Item{ _parent }
{
	Js::Dom::CreateElement( GetId(), _type, _parent ? _parent->get().GetId() : L"" );
	if( !_className.empty() )
		Js::Dom::AddClass( GetId(), _className );
	DataPoolRegister( *this, bool, enabled, [ this ]( const auto & _enabled )
	{
		Js::Dom::ToggleAttribute( GetId(), L"disabled", !_enabled );
	}, _enabled );
}


Element::~Element()
{
	Js::Dom::RemoveElement( GetId() );
}


// -----------------
// "body" HTML element (stored as a <div>)

Body::Body( const std::wstring & _className, const bool _enabled )
	: Element{ L"div", {}, _className, _enabled }
{}


// -----------------
// <div> HTML element

Division::Division( const Item & _parent, const std::wstring & _className, const bool _enabled )
	: Element{ L"div", _parent, _className, _enabled }
{}


// -----------------
// Indicates the object came with text content data

WithTextContent::WithTextContent( Item & _item, const std::wstring & _textContent )
	: m_item{ _item }
{
	DataPoolRegister( m_item, std::wstring, textContent, [ this ]( const auto & _textContent )
	{
		Js::Dom::SetTextContent( m_item.GetId(), _textContent );
	}, _textContent );
}


// -----------------
// <span> HTML element

Label::Label( const Item & _parent, const std::wstring & _textContent, const std::wstring & _className, const bool _enabled )
	: Element{ L"span", _parent, _className, _enabled }
	, WithTextContent{ *this, _textContent }
{}


// -----------------
// <img> HTML element

Image::Image( const Item & _parent, const std::wstring & _className, const bool _enabled )
	: Element{ L"img", _parent, _className, _enabled }
{}


void Image::SetSource( const std::wstring & _data )
{
	Js::Dom::SetImageSource( GetId(), _data );
}


// -----------------
// <button> HTML element

PushButton::PushButton( const Item & _parent, std::function< void() > && _onClick, const std::wstring & _textContent, const std::wstring & _className, const bool _enabled )
	: Element{ L"button", _parent, _className, _enabled }
	, WithTextContent{ *this, _textContent }
{
	Js::Dom::SetOnClick( GetId(), LambdaPool::Add( [ _onClick{ std::move( _onClick ) } ]( std::wstring && )
	{
		_onClick();
	} ) );
}


// -----------------
// a togglable button HTML element

ToggleButton::ToggleButton( const Item & _parent, std::function< void( const bool _toggled ) > && _onToggled, const std::wstring & _textContent, const std::wstring & _className, const bool _enabled )
	: PushButton{ _parent, [ this, _onToggled{ std::move( _onToggled ) } ] 
		{
			const auto toggled{ !Toggled() };
			Toggle( toggled );
			_onToggled( toggled );
		}, _textContent, _className, _enabled }
{
	Js::Dom::AddClass( GetId(), L"toggle" );
	DataPoolRegister( *this, bool, toggled, [ this ]( const auto & _toggled )
	{
		Js::Dom::ToggleAttribute( GetId(), L"toggled", _toggled );
	}, false );
}


} // Moth::Html