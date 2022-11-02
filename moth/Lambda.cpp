#include "Lambda.h"

// moth:
#include "System.h"


namespace Moth {
	

// -----------------
// Lambda manager, dealing with C++ lambda to be called by JavaScript

LambdaManager::~LambdaManager()
{
	std::lock_guard lock{ m_functorsMtx };
	System::Assert( m_functors.empty(), L"[LambdaManager::~LambdaManager] lambdas are not properly cleaned-up" );
}


LambdaManager & LambdaManager::GetInstance()
{
	static LambdaManager instance;
	return instance;
}


int LambdaManager::Add( std::function< void( std::wstring && ) > && _functor )
{
	std::lock_guard lock{ m_functorsMtx };
	static int index{ 0 };
	m_functors.emplace( ++index, std::move( _functor ) );
	return index;
}


void LambdaManager::Remove( const int _index )
{
	std::lock_guard lock{ m_functorsMtx };
	auto it{ m_functors.find( _index ) };
	System::Assert( it != m_functors.cend(), L"[LambdaManager::Remove] unknown function index" );
	m_functors.erase( it );
}


void LambdaManager::Call( const int _index, std::wstring && _data )
{
	std::lock_guard lock{ m_functorsMtx };
	auto it{ m_functors.find( _index ) };
	System::Assert( it != m_functors.cend(), L"[LambdaManager::Call] unknown function index" );
	it->second( std::move( _data ) );
}


// -----------------
// create a pool of lambda, that can be inherited when needed

LambdaPool::~LambdaPool()
{
	for( const int lambdaId : m_lambdaIds )
		LambdaManager::GetInstance().Remove( lambdaId );
}


const int LambdaPool::Add( std::function< void( std::wstring && ) > && _functor )
{
	const int lambdaId{ LambdaManager::GetInstance().Add( std::move( _functor ) ) };
	m_lambdaIds.emplace_back( lambdaId );
	return lambdaId;
}


} // Moth


// -----------------
//! \brief		exported code so a C++ lambda can be called from Js code using an index
extern "C" void CallCppLambda( const int _index, char * _data )
{
	Moth::LambdaManager::GetInstance().Call( _index, Moth::System::MbcsToWide( std::string{ _data } ) );
	free( _data );
}