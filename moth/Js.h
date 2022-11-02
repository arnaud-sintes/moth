#pragma once

// std library:
#include "StdIncludes.h"


namespace Moth::Js {


// -----------------
//! \brief		Javascript console wrapper
struct Console
{
	//! \brief	add informational log in the JavaScript console
	//! \param[in]	_message	the message to log
	static void Log( const std::wstring & _message );
	
	//! \brief	add a warning log in the JavaScript console
	//! \param[in]	_message	the warning message to log
	static void Warning( const std::wstring & _message );
	
	//! \brief	add an error log in the JavaScript console
	//! \param[in]	_message	the error message to log
	static void Error( const std::wstring & _message );
};


// -----------------
//! \brief		Javascript DOM manipulation
struct Dom
{
	//! \brief		static DOM initializer object
	struct Initializer;
	
	//! \brief	create a new DOM element
	//! \param[in]	_id			the element identifier in the DOM
	//! \param[in]	_type		[optional] the element type (default: div)
	//! \param[in]	_parentId	[optional] the element parent identifier (default: none)
	static void CreateElement( const std::wstring & _id, const std::wstring & _type = L"div", const std::wstring & _parentId = {} );

	//! \brief	remove a new DOM element
	//! \param[in]	_id			the DOM element identifier to remove
	static void RemoveElement( const std::wstring & _id );
	
	//! \brief	add a new CSS class name to a DOM element
	//! \param[in]	_id			the DOM element identifier
	//! \param[in]	_className	the CSS class name to add
	static void AddClass( const std::wstring & _id, const std::wstring & _className );
	
	//! \brief	remove a new CSS class name from a DOM element
	//! \param[in]	_id			the DOM element identifier
	//! \param[in]	_className	the CSS class name to remove
	static void RemoveClass( const std::wstring & _id, const std::wstring & _className );
	
	//! \brief	toggle a CSS class name on a given DOM element
	//! \param[in]	_id			the DOM element identifier
	//! \param[in]	_className	the CSS class name to toggle
	//! \param[in]	_active		the CSS toggle activation flag
	static void ToggleClass( const std::wstring & _id, const std::wstring & _className, const bool _active );
	
	//! \brief	set a new HTML attribute on a DOM element
	//! \param[in]	_id				the DOM element identifier
	//! \param[in]	_attributeName	the attribute name to add
	//! \param[in]	_value			[optional] the attribute value to associate (default: none)
	static void SetAttribute( const std::wstring & _id, const std::wstring & _attributeName, const std::wstring & _value = {} );
	
	//! \brief	remove an HTML attribute from a DOM element
	//! \param[in]	_id				the DOM element identifier
	//! \param[in]	_attributeName	the attribute name to remove
	static void RemoveAttribute( const std::wstring & _id, const std::wstring & _attributeName );
	
	//! \brief	toggle an HTML attribute on a given DOM element
	//! \param[in]	_id				the DOM element identifier
	//! \param[in]	_attributeName	the attribute name to toggle
	//! \param[in]	_active			the attribute toggle activation flag
	static void ToggleAttribute( const std::wstring & _id, const std::wstring & _attributeName, const bool _active );
	
	//! \brief	set the text content of a given DOM element
	//! \param[in]	_id				the DOM element identifier
	//! \param[in]	_textContent	the text content to set
	static void SetTextContent( const std::wstring & _id, const std::wstring & _textContent );
	
	//! \brief	attach a "onClick" event to a given DOM element
	//! \param[in]	_id				the DOM element identifier
	//! \param[in]	_lambdaId		the identifier of the lambda to be called when clicking the DOM element
	static void SetOnClick( const std::wstring & _id, const int _lambdaId );

	//! \brief	set the image source content of a given DOM element
	//! \param[in]	_id				the DOM element identifier
	//! \param[in]	_data			the image source data to set
	static void SetImageSource( const std::wstring & _id, const std::wstring & _data );
};


// -----------------
//! \brief		static DOM initializer object, used to set the page title and icon
struct Dom::Initializer
{
	//! \brief		ctor
	Initializer();
};


} // Moth::Js