#include "Item.h"

// moth:
#include "System.h"


namespace Moth {
	

Item::Item( const std::optional< std::reference_wrapper< const Item > > & _parent )
	: m_parent{ _parent }
	, m_id{ System::GenerateId() }
{
	if( m_parent )
		m_parent->get().RegisterChild( *this );
}


Item::~Item()
{
	if( m_parent )
		m_parent->get().UnRegisterChild( *this );
}


const std::wstring & Item::GetId() const
{
	return m_id;
}


std::wstring Item::Serialize() const
{
	std::wstring data;
	data.append( m_id + L"," );
	data.append( DataPool::Serialize() );
	for( const auto & pair : m_children )
		data.append( pair.second.get().Serialize() );
	return data;
}


void Item::DeSerialize( std::wstring & _data )
{
	System::Assert( System::ExtractData( _data ) == m_id, L"[Item::DeSerialize] identifier is non-consistent" );
	DataPool::DeSerialize( _data );
	for( const auto & pair : m_children )
		pair.second.get().DeSerialize( _data );
}


void Item::RegisterChild( Item & _item ) const
{
	m_children.emplace( _item.m_id, _item );
}


void Item::UnRegisterChild( Item & _item ) const
{
	m_children.erase( m_children.find( _item.m_id ) );
}


} // Moth