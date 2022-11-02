#pragma once

// moth:
#include "Data.h"

// std library:
#include "StdIncludes.h"


namespace Moth {
	

// -----------------
//! \brief		Generic item, with parent/child hierarchy
class Item
	: public DataPool
{
// ctor/dtor:
protected:
	//! \brief	ctor
	//! \param[in]	_parent		an optional reference over a parent item
	Item( const std::optional< std::reference_wrapper< const Item > > & _parent );
	
	//! \brief dtor
	virtual ~Item();
	
// public methods:
public:
	//! \return	the current identifier of the item
	const std::wstring & GetId() const;
	
	//! \brief		serialize the item with its children, recursively
	//! \return		the serialized data as a string
	std::wstring Serialize() const;
	
	//! \brief		de-serialize the item with its children
	//! \param[in]	_data	the string to de-serialize the data from
	//! \warning		can raise an assertion if the expected de-serialized name is unconsistent
	void DeSerialize( std::wstring & _data );
	
// internal methods:
private:
	//! \brief	register a given child item (called by child on creation)
	void RegisterChild( Item & _item ) const;
	
	//! \brief	un-register a given child item (called by child on descreation)
	void UnRegisterChild( Item & _item ) const;
	
// members:
private:
	const std::optional< std::reference_wrapper< const Item > > m_parent; //!< the potential parent's item reference
	const std::wstring m_id; //!< current item unique identifier
	mutable std::map< std::wstring, std::reference_wrapper< Item > > m_children; //!< children list
};


} // Moth