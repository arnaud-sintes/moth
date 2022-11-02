// moth:
#include "../../moth/[moth].h"


namespace Moth {


// back-end data structure
struct BEData
{
	std::optional< std::wstring > message;
	std::optional< std::wstring > image;
};
JsonReflection( BEData, message, image );

// front-end data structure
struct FEData
{
	std::optional< std::wstring > message;
	std::optional< bool > stream;
};
JsonReflection( FEData, message, stream );


// -----------------
//! \brief	User page definition
class Page
{
// ctor/dtor:
private:
	//! \brief	ctor, create every static elements and stuff
	Page()
		: m_body{ L"body" }
		, m_title{ m_body, L"> moth demonstration" }
		, m_btnWSSend{ m_body, [ this ]{ _OnSendBtnClicked(); }, L"WebSocket send" }
		, m_tgbtnState{ m_body }
		, m_tgbtnStream{ m_body, [ this ]( const bool _toggled ){ _OnToggleStreamtBtn( _toggled ); }, L"Stream" }
		, m_imageFps{ m_body, L"fps: -" }
		, m_image{ m_body }
		, m_ws{ [ this ]{ _OnSocketReady(); }, [ this ]( auto && _buffer ){ _OnSocketReceive( std::move( _buffer ) ); }, [ this ]{ _OnSocketTerminated(); } }
	{
		// connect websocket:
		System::Assert( m_ws.Connect( L"ws://localhost:2000" ), L"Fail to connect to WebSocket server" );
	}
	
// public methods:
public:
	//! \return		a reference over the unique singleton instance of the page
	static Page & GetInstance()
	{
		static Page instance;
		return instance;
	}

// internal methods:
private:
	void _OnSendBtnClicked()
	{
		const FEData data{ L"(Front-End) clicking send button" };
		const auto sData{ System::WideToMbcs( Json::Object::Serialize( data ).Stringify() ) };
		System::Assert( !m_ws.Send( Buffer::From( sData ) ), L"Fail to send" );
	}

	void _UpdateFps()
	{
		static unsigned long long frameCount{ 0 };
		static auto timeStampBegin{ std::chrono::system_clock::now() };
		frameCount++;
		const auto timeStampCurrent{ std::chrono::system_clock::now() };
		const auto timeStampDiff{ timeStampCurrent - timeStampBegin };
		if( timeStampDiff < std::chrono::seconds{ 1 } )
			return;
		const auto fps{ static_cast< double >( frameCount ) * static_cast< double >( std::chrono::duration_cast< std::chrono::milliseconds >( timeStampDiff ).count() ) / 1000. };
		m_imageFps.SetTextContent( L"fps: " + std::to_wstring( fps ) );
		frameCount = 0;
		timeStampBegin = timeStampCurrent;
	}

	void _OnSocketReceive( Buffer && _buffer )
	{
		const auto statusAnd{ Json::Object::Parse( System::MbcsToWide( _buffer.To< std::string >() ) ) };
		System::Assert( static_cast< bool >( statusAnd ), L"receive badly formatted JSON data" );
		const auto data{ statusAnd.object->DeSerialize< BEData >() };
		// receive a message:
		if( data.message )		
			return System::Output( L"[Server] WebSocket receive '" + *data.message + L"'" );
		// receive an image:
		if( data.image ) {
			_UpdateFps();
			return m_image.SetSource( *data.image );
		}
	}

	void _OnSocketReady()
	{
		const FEData data{ L"(Front-End) WebSocket is opened and ready" };
		const auto sData{ System::WideToMbcs( Json::Object::Serialize( data ).Stringify() ) };
		System::Assert( !m_ws.Send( Buffer::From( sData ) ), L"Fail to send" );
		// default toggle:
		_OnToggleStreamtBtn( m_tgbtnStream.Toggled() );
	}
	
	void _OnSocketTerminated()
	{
		System::Output( L"[Server] WebSocket terminated" );
	}

	void _OnToggleStreamtBtn( const bool _toggled )
	{
		// update state:
		m_tgbtnState.SetTextContent( L"Image streaming: " + std::wstring{ _toggled ? L"active" : L"inactive" } );
		// change back-end stream state:
		const FEData data{ {}, _toggled };
		const auto sData{ System::WideToMbcs( Json::Object::Serialize( data ).Stringify() ) };
		System::Assert( !m_ws.Send( Buffer::From( sData ) ), L"Fail to send" );
	}
	
// members:
private:
	Html::Body m_body;
	Html::Label m_title;
	Html::PushButton m_btnWSSend;
	Html::Label m_tgbtnState;
	Html::ToggleButton m_tgbtnStream;
	Html::Label m_imageFps;
	Html::Image m_image;
	WebSocket m_ws; //!< client web socket
};


} // Moth


// -----------------
// main entry point
int main()
{
	// instantiate the page using singleton accessor:
	Moth::Page::GetInstance();
	return 0;
}