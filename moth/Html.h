#pragma once

// moth:
#include "Item.h"
#include "Data.h"
#include "Lambda.h"

// std library:
#include "StdIncludes.h"


namespace Moth::Html {


// -----------------
//! \brief		Root HTML element
class Element
	: public Item
{
// ctor/dtor:
protected:	
	//! \brief		ctor
	//! \param[in]	_type		the DOM type of the element (div, span...)
	//! \param[in]	_parent		if any, the parent's item of this new item
	//! \param[in]	_className	[optional] the CSS class name to be associated to this element (default: none)
	//! \param[in]	_enabled	[optional] the CSS element enabling state (default: enabled)
	Element( const std::wstring & _type, const std::optional< std::reference_wrapper< const Item > > & _parent, const std::wstring & _className = {}, const bool _enabled = true );
	
	//! \brief		dtor
	virtual ~Element();

// data pool:
public:
	//! \brief		enabled state data
	DataPoolAccess( *this, bool, enabled, Enabled, Enable );
};


// -----------------
//! \brief		"body" HTML element (stored as a <div>)
class Body
	: public Element
{
// ctor/dtor:
public:
	//! \brief		ctor
	//! \remark		a body element got no parent
	//! \param[in]	_className	[optional] the CSS class name to be associated to this element (default: none)
	//! \param[in]	_enabled	[optional] the CSS element enabling state (default: enabled)
	Body( const std::wstring & _className = {}, const bool _enabled = true );
};


// -----------------
//! \brief		<div> HTML element
class Division
	: public Element
{
// ctor/dtor:
public:
	//! \brief		ctor
	//! \param[in]	_parent		the parent's item of this new item
	//! \param[in]	_className	[optional] the CSS class name to be associated to this element (default: none)
	//! \param[in]	_enabled	[optional] the CSS element enabling state (default: enabled)
	Division( const Item & _parent, const std::wstring & _className = {}, const bool _enabled = true );
};


// -----------------
//! \brief		Indicates the object came with text content data
class WithTextContent
{
// ctor/dtor:
protected:
	//! \brief		ctor
	//! \param[in]	_item			a reference over the item that got text content
	//! \param[in]	_textContent	[optional] the text content default value (default: none)
	WithTextContent( Item & _item, const std::wstring & _textContent = {} );
	
// data pool:
public:
	//! \brief		textContent string data
	DataPoolAccess( m_item, std::wstring, textContent, GetTextContent, SetTextContent );
	
// members:
private:
	Item & m_item; //!< a reference over the item with text content
};


// -----------------
//! \brief		<span> HTML element
class Label
	: public Element
	, public WithTextContent
{
// ctor/dtor:
public:
	//! \brief		ctor
	//! \param[in]	_parent			the parent's item of this new item
	//! \param[in]	_textContent	[optional] the text content default value (default: none)
	//! \param[in]	_className		[optional] the CSS class name to be associated to this element (default: none)
	//! \param[in]	_enabled		[optional] the CSS element enabling state (default: enabled)
	Label( const Item & _parent, const std::wstring & _textContent = {}, const std::wstring & _className = {}, const bool _enabled = true );
};


// -----------------
//! \brief		<img> HTML element
class Image
	: public Element
{
// ctor/dtor:
public:
	//! \brief		ctor
	//! \param[in]	_parent			the parent's item of this new item
	//! \param[in]	_className		[optional] the CSS class name to be associated to this element (default: none)
	//! \param[in]	_enabled		[optional] the CSS element enabling state (default: enabled)
	Image( const Item & _parent, const std::wstring & _className = {}, const bool _enabled = true );

// methods:
public:
	// !\brief	set the image source data
	//! \param[in]	_data			the image source data to set
	void SetSource( const std::wstring & _data );
};


// -----------------
//! \brief		<button> HTML element
class PushButton
	: public Element
	, public WithTextContent
	, LambdaPool
{
// ctor/dtor:
public:
	//! \brief		ctor
	//! \param[in]	_parent			the parent's item of this new item
	//! \param[in]	_onClick		the callback to trigger when we click on the button
	//! \param[in]	_textContent	[optional] the text content default value (default: none)
	//! \param[in]	_className		[optional] the CSS class name to be associated to this element (default: none)
	//! \param[in]	_enabled		[optional] the CSS element enabling state (default: enabled)
	PushButton( const Item & _parent, std::function< void() > && _onClick, const std::wstring & _textContent = {}, const std::wstring & _className = {}, const bool _enabled = true );
};


// -----------------
//! \brief		a togglable button HTML element
class ToggleButton
	: public PushButton
{
// ctor/dtor:
public:
	//! \brief		ctor
	//! \param[in]	_parent			the parent's item of this new item
	//! \param[in]	_onToggled		the callback to trigger when the button was toggled
	//! \param[in]	_textContent	[optional] the text content default value (default: none)
	//! \param[in]	_className		[optional] the CSS class name to be associated to this element (default: none)
	//! \param[in]	_enabled		[optional] the CSS element enabling state (default: enabled)
	ToggleButton( const Item & _parent, std::function< void( const bool _toggled ) > && _onToggled, const std::wstring & _textContent, const std::wstring & _className = {}, const bool _enabled = true );
	
// data pool:
public:
	//! \brief		toggled flag data
	DataPoolAccess( *this, bool, toggled, Toggled, Toggle );
};


} // Moth::Html