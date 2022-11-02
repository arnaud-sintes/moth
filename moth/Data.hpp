// -----------------
// Specialized data object

template< typename _Type >
Data< _Type >::Data( _Type && _data, std::function< void( const _Type & _data ) > && _onUpdate )
	: m_data{ std::move( _data ) }
	, m_onUpdate{ std::move( _onUpdate ) }
{
	m_onUpdate( m_data );
}


template< typename _Type >
void Data< _Type >::Set( _Type && _data )
{
	m_data = std::move( _data );
	m_onUpdate( m_data );
}


template< typename _Type >
const _Type & Data< _Type >::Get() const
{
	return m_data;
}


template< typename _Type >
std::wstring Data< _Type >::Serialize() const
{
	return m_data;
}


template<>
std::wstring Data< bool >::Serialize() const;


template< typename _Type >
void Data< _Type >::DeSerialize( const std::wstring & _data )
{
	Set( std::wstring{ _data } );
}


template<>
void Data< bool >::DeSerialize( const std::wstring & _data );


// -----------------
// A pool of multiple registered data

template< typename _Type, typename... _Args >
void DataPool::Register( const std::wstring & _name, std::function< void( const _Type & _data ) > && _onUpdate, _Args&&... _args )
{
	System::Assert( m_data.find( _name ) == m_data.cend(), L"[DataPool::Register] data '" + _name + L"' already registered" );
	m_data.emplace( _name, new Data< _Type >{ _Type{ std::forward< _Args >( _args )... }, std::move( _onUpdate ) } );
}


template< typename _Type, typename... _Args >
void DataPool::Set( const std::wstring & _name, _Args&&... _args )
{
	auto itData{ m_data.find( _name ) };
	System::Assert( itData != m_data.cend(), L"[DataPool::Set] unknown data '" + _name + L"'" );
	static_cast< Data< _Type > & >( *itData->second ).Set( _Type{ std::forward< _Args >( _args )... } );
}


template< typename _Type >
const _Type & DataPool::Get( const std::wstring & _name ) const
{
	auto itData{ m_data.find( _name ) };
	System::Assert( itData != m_data.cend(), L"[DataPool::Get] unknown data '" + _name + L"'" );
	return static_cast< const Data< _Type > & >( *itData->second ).Get();
}