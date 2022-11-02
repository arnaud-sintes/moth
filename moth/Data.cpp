#include "Data.h"


namespace Moth {


// -----------------
// Specialized data object

template<>
std::wstring Data< bool >::Serialize() const
{
	return std::to_wstring( m_data );
}


template<>
void Data< bool >::DeSerialize( const std::wstring & _data )
{
	Set( std::stoi( _data.c_str() ) == 1 );
}


// -----
// A pool of multiple registered data

std::wstring DataPool::Serialize() const
{
	std::wstring data;
	for( const auto & pair : m_data )
		data.append( pair.first + L"," + pair.second->Serialize() + L"," );
	return data;
}


void DataPool::DeSerialize( std::wstring & _data )
{
	for( const auto & pair : m_data ) {
		System::Assert( System::ExtractData( _data ) == pair.first, L"[DataPool::DeSerialize] name is non-consistent" );
		pair.second->DeSerialize( System::ExtractData( _data ) );
	}
}


} // Moth