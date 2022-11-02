#pragma once

// std library:
#include "StdIncludes.h"


namespace Moth {


// -----------------
//! \brief  basically an unordered_map, respecting the emplacing order
template< typename _Key, typename _Type >
class IndexedMap
{
public:
	IndexedMap() = default;
	~IndexedMap() = default;

	IndexedMap( const IndexedMap& _other ) { *this = _other; }
	IndexedMap( IndexedMap&& _other ) { *this = std::move( _other ); }

	IndexedMap& operator=( const IndexedMap& _other )
	{
		m_data = _other.m_data;
		_Remap();
		return *this;
	}

	IndexedMap& operator=( IndexedMap&& _other )
	{
		std::swap( m_data, _other.m_data );
		_Remap();
		return *this;
	}

public:
	using _It = typename std::list< std::pair< _Key, _Type > >::iterator;
	using _ConstIt = typename std::list< std::pair< _Key, _Type > >::const_iterator;

	template< typename _Ptr >
	struct _GenericIterator;

	using iterator = IndexedMap< _Key, _Type >::_GenericIterator< _It >;
	using const_iterator = IndexedMap< _Key, _Type >::_GenericIterator< _ConstIt >;

public:
	bool empty() const { return m_data.empty(); }
	unsigned long long size() const { return m_data.size(); }

	void clear()
	{
		m_data.clear();
		m_dataMap.clear();
	}

	iterator emplace( const _Key & _key, _Type&& _type )
	{
		m_data.emplace_back( std::pair< _Key, _Type >{ std::move( _key ), std::move( _type ) } );
		auto end{ std::prev( m_data.end() ) };
		return m_dataMap.emplace( end->first, end ).first->second;
	}

	iterator find( const _Key& _key ) { auto it{ m_dataMap.find( _key ) }; return it == m_dataMap.cend() ? end() : it->second; }
	const_iterator find( const _Key& _key ) const { auto it{ m_dataMap.find( _key ) }; return it == m_dataMap.cend() ? cend() : static_cast< const_iterator >( it->second ); }

	iterator erase( iterator _it )
	{
		auto it{ find( _it.ptr->first ) };
		m_data.erase( _it );
		return m_dataMap.erase( it.ptr );
	}

	iterator begin() { return m_data.begin(); }
	iterator end() { return m_data.end(); }
	const_iterator cbegin() { return m_data.cbegin(); }
	const_iterator cend() { return m_data.cend(); }

	const_iterator begin() const { return m_data.begin(); }
	const_iterator end() const { return m_data.end(); }
	const_iterator cbegin() const { return m_data.cbegin(); }
	const_iterator cend() const { return m_data.cend(); }

private:
	void _Remap()
	{
		m_dataMap.clear();
		for ( auto it{ m_data.begin() }, itEnd{ m_data.end() }; it != itEnd; ++it )
			m_dataMap.emplace( it->first, it );
	}

private:
	std::list< std::pair< _Key, _Type > > m_data;
	std::unordered_map< _Key, typename std::list< std::pair< _Key, _Type > >::iterator > m_dataMap;
};


// -----------------
//! \brief  specific IndexedMap iterator object
template< typename _Key, typename _Type >
template< typename _Ptr >
struct IndexedMap< _Key, _Type >::_GenericIterator
{
	friend class IndexedMap< _Key, _Type >;

	_GenericIterator() = default;
	~_GenericIterator() = default;

	_GenericIterator( _Ptr _ptr )
		: ptr{ _ptr }
	{}

	_GenericIterator( const _GenericIterator& ) = default;
	_GenericIterator& operator=( const _GenericIterator& ) = default;
	_GenericIterator( _GenericIterator&& ) = default;
	_GenericIterator& operator=( _GenericIterator&& ) = default;

	using difference_type = std::ptrdiff_t;

	using Pair = std::pair< _Key, _Type >;
	Pair & operator* () { return const_cast< Pair & >( *ptr ); }
	const Pair & operator* () const { return *ptr; }
	_Ptr operator-> () { return ptr; }

	operator bool() const { return static_cast< bool >( ptr ); }
	bool operator== ( const auto& _b ) const { return ptr == _b.ptr; };
	bool operator!= ( const auto& _b ) const { return ptr != _b.ptr; };

	auto & operator+= ( const difference_type & _movement ) { ptr += _movement; return *this; }
    auto & operator-= ( const difference_type & _movement ) { ptr -= _movement; return *this; }
    auto & operator++ () { ++ptr; return *this; }
    auto & operator-- () { --ptr; return *this; }
    auto operator++ ( int ){ auto temp{ *this }; ++ptr; return temp; }
    auto operator-- ( int ){ auto temp{ *this }; --ptr; return temp; }
    auto operator+ ( const difference_type & _movement ) { auto oldPtr{ ptr }; ptr += _movement; auto temp{ *this }; ptr = oldPtr; return temp; }
    auto operator- ( const difference_type & _movement ) { auto oldPtr{ ptr }; ptr -= _movement; auto temp{ *this }; ptr = oldPtr; return temp; }

    difference_type operator- ( const auto & _other ){ return std::distance( _other.ptr->second, ptr->second ); }

	_GenericIterator next() { return std::next( ptr ); }
	_GenericIterator prev() { return std::prev( ptr ); }

private:
	_Ptr ptr;
};



} // Moth