#pragma once

// std library:
#include "StdIncludes.h"


namespace Moth {


// -----------------
//! \brief		Lambda manager, dealing with C++ lambda to be called by JavaScript
class LambdaManager
{
// ctor/dtor:
private:
	//! \brief		default ctor
	LambdaManager() = default;
	
	//! \brief		dtor, check if all lambdas are properly unregistered
	//! \warning	can raise an assertion if some lambdas remains
	~LambdaManager();
	
// singleton accessor:
public:
	//! \brief		return a reference over the unique singleton instance
	static LambdaManager & GetInstance();
	
// public methods:
public:
	//! \brief		add a new closure
	//! \param[in]	_functor	the lambda to be added in the manager
	//! \return		the index of the lambda to be called
	int Add( std::function< void( std::wstring && _data ) > && _functor );
	
	//! \brief		remove an existing closure
	//! \param[in]	_index		the index of the lambda to remove
	//! \warning	can raise an assertion if the lambda is unknown
	void Remove( const int _index );
	
	//! \brief		call an existing closure
	//! \param[in]	_index		the index of the lambda to call
	//! \warning	can raise an assertion if the lambda is unknown
	void Call( const int _index, std::wstring && _data );
	
// members:
private:
	std::mutex m_functorsMtx; //!< functors mutex
	std::unordered_map< int, std::function< void( std::wstring && ) > > m_functors; //!< functors list, per identifier
};


// -----------------
//! \brief		create a pool of lambda, that can be inherited when needed
class LambdaPool
{
// ctor/dtor:
protected:
	//! \brief		default ctor
	LambdaPool() = default;
	
	//! \brief		dtor, will remove all the lambdas known by the pool from the manager
	virtual ~LambdaPool();
	
// methods to be called by sub-classes:
protected:
	//! \brief		add a new closure in the pool, with an implicit registration in the manager
	//! \param[in]	_functor	the lambda to be added in the manager
	//! \return		the index of the lambda to be called
	const int Add( std::function< void( std::wstring && ) > && _functor );
	
// members:
private:
	std::list< int > m_lambdaIds; //!< the list of lambda identifiers attached to the pool
};


} // Moth