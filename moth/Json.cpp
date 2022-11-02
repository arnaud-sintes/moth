#include "Json.h"


namespace Moth {


//! \brief  Json Boolean serialization
//! \param        Json        (just a placeholder for Json serialization streaming operation)
//! \param[in]    _data        a reference over the C++ Boolean to serialize from
//! \return        a Json Generic pointer
JsonGeneric::Ptr operator << ( const Json &, const bool & _data )
{
    return Json::Boolean::Ptr{ _data };
}


//! \brief  Json Boolean deserialization
//! \param[in]    _generic    the Json Generic type to use as a Json Boolean
//! \param[in,out]    _data        a reference over the C++ Boolean to deserialize to
//! \return        a reference over the input Json Generic
const JsonGeneric & operator >> ( const JsonGeneric & _generic, bool & _data )
{
    _data = _generic.As< Json::Boolean >().data;
    return _generic;
}


// ----------

//! \brief  Json String (UCS-2/UTF-16) serialization
//! \param        Json        (just a placeholder for Json serialization streaming operation)
//! \param[in]    _data        a reference over the C++ String to serialize from
//! \return        a Json Generic pointer
JsonGeneric::Ptr operator << ( const Json &, const std::wstring & _data )
{
    return Json::String::Ptr{ _data };
}


//! \brief  Json String (UCS-2/UTF-16) deserialization
//! \param[in]    _generic    the Json Generic type to use as a Json String
//! \param[in,out]    _data        a reference over the C++ String to deserialize to
//! \return        a reference over the input Json Generic
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::wstring & _data )
{
    _data = _generic.As< Json::String >().data;
    return _generic;
}


// ----------

//! \brief  Json String (ANSI) serialization
//! \param        Json        (just a placeholder for Json serialization streaming operation)
//! \param[in]    _data        a reference over the C++ String to serialize from
//! \return        a Json Generic pointer
JsonGeneric::Ptr operator << ( const Json &, const std::string & _data )
{
    return Json::String::Ptr{ System::MbcsToWide( _data ) };
}


//! \brief  Json String (ANSI) deserialization
//! \param[in]    _generic    the Json Generic type to use as a Json String
//! \param[in,out]    _data        a reference over the C++ String to deserialize to
//! \return        a reference over the input Json Generic
const JsonGeneric & operator >> ( const JsonGeneric & _generic, std::string & _data )
{
    _data = System::WideToMbcs( _generic.As< Json::String >().data );
    return _generic;
}


// ----------
//! \brief        Char is a container defining common characters and String sequences useful for stringification, parsing & path process
struct Char
{
    static constexpr const wchar_t backSpace{ L'\b' };
    static constexpr const wchar_t formFeed{ L'\f' };
    static constexpr const wchar_t backSlash{ L'\\' };
    static constexpr const wchar_t slash{ L'/' };
    static constexpr const wchar_t carriageReturn{ L'\r' };
    static constexpr const wchar_t lineFeed{ L'\n' };
    static constexpr const wchar_t comma{ L',' };
    static constexpr const wchar_t space{ L' ' };
    static constexpr const wchar_t negative{ L'-' };
    static constexpr const wchar_t positive{ L'+' };
    static constexpr const wchar_t exponentUpperCase{ L'E' };
    static constexpr const wchar_t exponentLowerCase{ L'e' };
    static constexpr const wchar_t dot{ L'.' };
    static constexpr const wchar_t tabulation{ L'\t' };
    static constexpr const wchar_t openCurlyBrace{ L'{' };
    static constexpr const wchar_t closeCurlyBrace{ L'}' };
    static constexpr const wchar_t colon{ L':' };
    static constexpr const wchar_t openSquareBracket{ L'[' };
    static constexpr const wchar_t closeSquareBracket{ L']' };
    static constexpr const wchar_t doubleQuote{ L'\"' };
    static constexpr const wchar_t singleQuote{ L'\'' };
    static constexpr const wchar_t dollar{ L'$' };

    static inline const std::wstring crlf{ L"\r\n" };
    static inline const std::wstring booleanTrue{ L"true" };
    static inline const std::wstring booleanFalse{ L"false" };
    static inline const std::wstring Null{ L"null" };
    static inline const std::wstring numbers{ L"0123456789" };
};


// ----------
//! \brief        Parser is an helper class use for stringification and parsing process
struct Parser final
{
    //! \brief    (un)escaping a text (adding or removing backslashes were needed)
    //! \param[in]    _text    the input text to process
    //! \param[in]    _escape    when set to true (default) process with escaping, unescape when set to false
    //! \return        an (un)escaped text
    static std::wstring Escape( const std::wstring & _text, const bool _escape = true );

    //! \brief    create a proper line indentation String considering the current tabulation String and depth
    //! \param[in]    _tab        the String to use to tabulate the content of the stringified Json data
    //! \param[in]    _depth        the current tabulation depth
    //! \return        a String that can be used for current line indentation prefix
    static std::wstring Indent( const std::wstring & _tab, unsigned long long _depth );

    //! \param[in]    _tab        the String to use to tabulate the content of the stringified Json data
    //! \return        return a String corresponding to an end of line sequence (CRLF) in case we're dealing with tabulations, or an empty String in case of minimal stringification
    static std::wstring EndLine( const std::wstring & _tab );

    //! \param[in]    _tab        the String to use to tabulate the content of the stringified Json data
    //! \return        return a String corresponding to a space character in case we're dealing with tabulations, or an empty String in case of minimal stringification
    static std::wstring Space( const std::wstring & _tab );

    //! \param[in]        _pos        current iterator position in text
    //! \param[in]        _end        end iterator position in text
    //! \return        the position of the next useful character in the String (avoid spaces, tabulations, line returns etc.)
    static std::wstring::const_iterator NextUsefulChar( std::wstring::const_iterator _pos, const std::wstring::const_iterator _end );

    //! \brief    find the corresponding closure position the text, using a counter
    //! \param[in]        _pos        current iterator position in text
    //! \param[in]        _end        end iterator position in text
    //! \param[in]        _open        opening character definition
    //! \param[in]        _close        closure character definition
    //! \return        the position of the closure character in the text
    static std::wstring::const_iterator FindClosure( std::wstring::const_iterator _pos, const std::wstring::const_iterator _end, const char _open, const char _close );

    //! \brief    find the corresponding text closure position the text, avoiding escaped double quotes
    //! \param[in]        _pos        current iterator position in text
    //! \param[in]        _end        end iterator position in text
    //! \return        the position of the closure character in the text
    static std::wstring::const_iterator FindTextClosure( std::wstring::const_iterator _pos, const std::wstring::const_iterator _end );

    //! \brief    Parse the text from the given position in order to find the next useful statement, determine the nature of the statement and call the proper parser
    //! \param[in,out]    _pos        current iterator position in text (will be updated at the end of the parsing)
    //! \param[in]        _end        end iterator position in text
    //! \return        a Json Generic pointer depending of the statement nature
    static Status::And< JsonGeneric::Ptr > ParseGeneric( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end );

    //! \brief    Parse the text from the given position in order to extract a Json Null type
    //! \param[in,out]    _pos        current iterator position in text (will be updated at the end of the parsing)
    //! \param[in]        _end        end iterator position in text
    //! \return        a Json Generic pointer over a Json Null
    static Status::And< JsonGeneric::Ptr > ParseNull( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end );

    //! \brief    Parse the text from the given position in order to extract a Json Number type
    //! \param[in,out]    _pos        current iterator position in text (will be updated at the end of the parsing)
    //! \param[in]        _end        end iterator position in text
    //! \return        a Json Generic pointer over a Json Number
    static Status::And< JsonGeneric::Ptr > ParseNumber( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end );

    //! \brief    Parse the text from the given position in order to extract a Json Boolean type
    //! \param[in,out]    _pos        current iterator position in text (will be updated at the end of the parsing)
    //! \param[in]        _end        end iterator position in text
    //! \return        a Json Generic pointer over a Json Boolean
    static Status::And< JsonGeneric::Ptr > ParseBoolean( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end );

    //! \brief    Parse the text from the given position in order to extract a Json String type
    //! \param[in,out]    _pos        current iterator position in text (will be updated at the end of the parsing)
    //! \param[in]        _end        end iterator position in text
    //! \return        a Json Generic pointer over a Json String
    static Status::And< JsonGeneric::Ptr > ParseString( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end );

    //! \brief    Parse the text from the given position in order to extract a Json Array type
    //! \param[in,out]    _pos        current iterator position in text (will be updated at the end of the parsing)
    //! \param[in]        _end        end iterator position in text
    //! \return        a Json Generic pointer over a Json Array
    static Status::And< JsonGeneric::Ptr > ParseArray( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end );

    //! \brief    Parse the text from the given position in order to extract a Json Object type
    //! \param[in,out]    _pos        current iterator position in text (will be updated at the end of the parsing)
    //! \param[in]        _end        end iterator position in text
    //! \return        a Json Generic pointer over a Json Object
    static Status::And< JsonGeneric::Ptr > ParseObject( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end );
};


std::wstring Json::Null::Stringify( const std::wstring & /*_tab*/, const unsigned long long /*_depth*/ ) const
{
    return Char::Null;
}


std::wstring Json::Number::Stringify( const std::wstring & /*_tab*/, const unsigned long long /*_depth*/ ) const
{
    auto s{ std::to_wstring( data ) };

    // remove useless trailing zeros:
    s.erase( s.find_last_not_of( Char::numbers.at( 0 ) ) + 1, std::wstring::npos );

    // remove also the dot is useless:
    if( s.back() == Char::dot )
        s.pop_back();

    return s;
}


std::wstring Json::Boolean::Stringify( const std::wstring & /*_tab*/, const unsigned long long /*_depth*/ ) const
{
    return data ? Char::booleanTrue : Char::booleanFalse;
}


std::wstring Json::String::Stringify( const std::wstring & /*_tab*/, const unsigned long long /*_depth*/ ) const
{
    return Char::doubleQuote + Parser::Escape( data ) + Char::doubleQuote;
}


std::wstring Json::Array::Stringify( const std::wstring & _tab, const unsigned long long _depth ) const
{
    std::wstring s;
    s += Char::openSquareBracket;
    s += Parser::EndLine( _tab );
    if( !data.empty() ) {
        for( auto it{ data.cbegin() }, itEnd{ data.cend() }, itLast{ std::prev( itEnd ) }; it != itEnd; ++it ) {
            s += Parser::Indent( _tab, _depth + 1 );
            s += ( *it )->Stringify( _tab, _depth + 1 );
            s += ( it != itLast ? std::wstring{ Char::comma } : L"" ) + Parser::EndLine( _tab );
        }
    }
    s += Parser::Indent( _tab, _depth );
    s += Char::closeSquareBracket;
    return s;
}


std::wstring Json::Object::Stringify(const std::wstring &_tab, const unsigned long long _depth) const
{
    std::wstring s;
    s += Char::openCurlyBrace;
    s += Parser::EndLine(_tab);
    if (!data.empty())
    {

        for (auto it{data.cbegin()}, itEnd{data.cend()}, itLast{itEnd.prev()}; it != itEnd; ++it)
        {
            // Do not parse null element
            if(dynamic_cast<Json::Null*>(it->second.get()) != nullptr)
            {
                // If last, remove the potential comma inserted during previous iteration
                const auto tail = std::wstring{ Char::comma } + Parser::EndLine(_tab);
                if (it == itLast && s.ends_with(tail))
                {
                    s.resize(s.size() - tail.size());
                    s += Parser::EndLine(_tab);
                }
                continue;
            }
            s += Parser::Indent(_tab, _depth + 1);
            s += Char::doubleQuote + it->first + Char::doubleQuote;
            s += Char::colon + Parser::Space(_tab);
            s += it->second->Stringify(_tab, _depth + 1);
            s += (it != itLast ? std::wstring{Char::comma} : L"") + Parser::EndLine(_tab);
        }
    }
    s += Parser::Indent(_tab, _depth);
    s += Char::closeCurlyBrace;
    return s;
}


std::wstring Parser::Escape( const std::wstring & _text, const bool _escape )
{
    // characters conversion table:
    static std::list< std::pair< wchar_t, wchar_t > > chars{
        { Char::backSpace, L'b' },
        { Char::formFeed, L'f' },
        { Char::lineFeed, L'n' },
        { Char::carriageReturn, L'r' },
        { Char::tabulation, L't' },
        { Char::doubleQuote, Char::doubleQuote },
        { Char::backSlash, Char::backSlash },
        { Char::slash, Char::slash },
    };

    std::wstring text;
    // escape the text:
    if( _escape )
        for( const auto c : _text ) {
            auto itChars{ std::find_if( chars.cbegin(), chars.cend(), [ c ]( const auto & _pair ){ return _pair.first == c; } ) };
            text +=( itChars == chars.cend() ? std::wstring{ c } : std::wstring{ Char::backSlash } + itChars->second );
        }
    // unescape the text:
    else
        for( auto it{ _text.cbegin() }, itEnd{ _text.cend() }; it != itEnd; ++it )
            text += *it == Char::backSlash ? std::find_if( chars.cbegin(), chars.cend(), [ c{ *++it } ]( const auto & _pair ){ return _pair.second == c; } )->first : *it;
    return text;
}


std::wstring Parser::Indent( const std::wstring & _tab, unsigned long long _depth )
{
    std::wstring indentation;
    while( _depth-- != 0 )
        indentation.append( _tab );
    return indentation;
}


std::wstring Parser::EndLine( const std::wstring & _tab )
{
    return _tab.empty() ? L"" : Char::crlf;
}


std::wstring Parser::Space( const std::wstring & _tab )
{
    return _tab.empty() ? L"" : std::wstring{ Char::space };
}


std::wstring::const_iterator Parser::NextUsefulChar( std::wstring::const_iterator _pos, const std::wstring::const_iterator _end )
{
    for( ; _pos != _end; ++_pos )
        if( *_pos != Char::carriageReturn && *_pos != Char::lineFeed && *_pos != Char::space && *_pos != Char::tabulation )
            return _pos;
    return _end;
}


std::wstring::const_iterator Parser::FindClosure( std::wstring::const_iterator _pos, const std::wstring::const_iterator _end, const char _open, const char _close )
{
    unsigned long long count{ 1 };
    bool insideString{ false };
    for( ; _pos != _end && count > 0; ++_pos ) {
        insideString = *_pos == Char::doubleQuote ? !insideString : insideString;
        if( !insideString )
            count += *_pos == _open ? 1 : ( *_pos == _close ? -1 : 0 );
    }
    return _pos;
}


std::wstring::const_iterator Parser::FindTextClosure( std::wstring::const_iterator _pos, const std::wstring::const_iterator _end )
{
    bool backSlashPrefix{ false };
    for( ; _pos != _end; ++_pos ) {
        if( *_pos == Char::doubleQuote && !backSlashPrefix )
            break;
        if( *_pos == Char::backSlash ) {
            backSlashPrefix    = !backSlashPrefix;
            continue;
        }
        backSlashPrefix = false;
    }
    return _pos;
}


Status::And< JsonGeneric::Ptr > Parser::ParseGeneric( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end )
{
    _pos = NextUsefulChar( _pos, _end );
    if( *_pos == Char::openCurlyBrace )
        return ParseObject( _data, _pos, _end );
    if( *_pos == Char::openSquareBracket )
        return ParseArray( _data, _pos, _end );
    if( *_pos == Char::doubleQuote )
        return ParseString( _data, _pos, _end );
    if( *_pos == Char::Null.at( 0 ) )
        return ParseNull( _data, _pos, _end );
    if( *_pos == Char::booleanTrue.at( 0 ) || *_pos == Char::booleanFalse.at( 0 ) )
        return ParseBoolean( _data, _pos, _end );
    return ParseNumber( _data, _pos, _end );
}


Status::And< JsonGeneric::Ptr > Parser::ParseNull( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end )
{
    std::advance( _pos, Char::Null.length() );
    if( _pos == _end ) {
        const auto position{ System::OffsetToPosition( _data, static_cast< unsigned long long >( _pos - _data.cbegin() ) ) };
        return { StatusFailure( FailToParse, L"error while parsing null value", std::to_wstring( position.row ), std::to_wstring( position.column ) ) };
    }
    return { StatusOk, Json::Null::Ptr{} };
}


Status::And< JsonGeneric::Ptr > Parser::ParseNumber( const std::wstring & /*_data*/, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end )
{
    std::wstring value;
    for( ; _pos != _end && ( *_pos == Char::negative || *_pos == Char::positive || *_pos == Char::dot || *_pos == Char::exponentLowerCase || *_pos == Char::exponentUpperCase ||
         ( *_pos >= Char::numbers.front() && *_pos <= Char::numbers.back() ) ); ++_pos )
        value += *_pos;

    // note stod deals properly with exponents notations:
    return { StatusOk, Json::Number::Ptr{ std::stod( value ) } };
}


Status::And< JsonGeneric::Ptr > Parser::ParseBoolean( const std::wstring & /*_data*/, std::wstring::const_iterator & _pos, const std::wstring::const_iterator /*_end*/ )
{
    const bool isTrue{ *_pos == Char::booleanTrue.at( 0 ) };
    std::advance( _pos, isTrue ? Char::booleanTrue.length() : Char::booleanFalse.length() );
    return { StatusOk, Json::Boolean::Ptr{ isTrue } };
}


Status::And< JsonGeneric::Ptr > Parser::ParseString( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end )
{
    _pos = std::next( _pos );
    const auto end{ FindTextClosure( _pos, _end ) };
    if( _pos == _end ) {
        const auto position{ System::OffsetToPosition( _data, static_cast< unsigned long long >( _pos - _data.cbegin() ) ) };
        return { StatusFailure( FailToParse, L"error while parsing string value", std::to_wstring( position.row ), std::to_wstring( position.column ) ) };
    }
    std::wstring text{ _pos, end };
    _pos = std::next( end );
    return { StatusOk, Json::String::Ptr{ Escape( text, false ) } };
}


Status::And< JsonGeneric::Ptr > Parser::ParseArray( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end )
{
    Json::Array::Ptr arr;
    _pos = std::next( _pos );
    const auto end{ FindClosure( _pos, _end, Char::openSquareBracket, Char::closeSquareBracket ) };
    while( _pos != end ) {
        if( *NextUsefulChar( _pos, _end ) == Char::closeSquareBracket ) // no data in the Array
            break;
        auto statusAnd{ ParseGeneric( _data, _pos, end ) };
        if( !statusAnd )
            return statusAnd;
        arr.ptr->data.emplace_back( std::move( *statusAnd.object ) );
        auto commaIt{ std::find( _pos, end, Char::comma ) };
        _pos = commaIt == end ? end : std::next( commaIt );
    }
    return { StatusOk, std::move( arr ) };
}


Status::And< JsonGeneric::Ptr > Parser::ParseObject( const std::wstring & _data, std::wstring::const_iterator & _pos, const std::wstring::const_iterator _end )
{
    Json::Object::Ptr object;
    _pos = std::next( _pos );
    const auto end{ FindClosure( _pos, _end, Char::openCurlyBrace, Char::closeCurlyBrace ) };
    while( _pos != end ) {
        auto openDoubleQuoteIt{ std::find( _pos, end, Char::doubleQuote ) };
        if( openDoubleQuoteIt == end ) // no data in the Object
            break;
        auto closeDoubleQuoteIt{ std::find( std::next( openDoubleQuoteIt ), end, Char::doubleQuote ) };
        const std::wstring name{ std::next( openDoubleQuoteIt ), closeDoubleQuoteIt };
        if( object.ptr->data.find( name ) != object.ptr->data.cend() ) {
            const auto position{ System::OffsetToPosition( _data, static_cast< unsigned long long >( _pos - _data.cbegin() ) ) };
            return { StatusFailure( FailToParse, L"key '" + name + L"' already exists", std::to_wstring( position.row ), std::to_wstring( position.column ) ) };
        }
        if( closeDoubleQuoteIt == _end ) {
            const auto position{ System::OffsetToPosition( _data, static_cast< unsigned long long >( _pos - _data.cbegin() ) ) };
            return { StatusFailure( FailToParse, L"double quote opening/closure mismatch", std::to_wstring( position.row ), std::to_wstring( position.column ) ) };
        }
        auto colonIt{ std::find( std::next( closeDoubleQuoteIt ), end, Char::colon ) };
        _pos = std::next( colonIt );
        auto statusAnd{ ParseGeneric( _data, _pos, end ) };
        if( !statusAnd )
            return statusAnd;
        object.ptr->data.emplace( name, std::move( *statusAnd.object ) );
        auto commaIt{ std::find( _pos, end, Char::comma ) };
        _pos = commaIt == end ? end : std::next( commaIt );
    }
    return { StatusOk, std::move( object ) };
}


// ----------

Status::And< JsonGeneric::Ptr > JSON::Parse( const std::wstring & _data )
{
    auto pos{ _data.cbegin() };
    return Parser::ParseGeneric( _data, pos, _data.cend() );
}


// ----------
//! \brief        Path is an helper class use for Json path search
struct PathParser final
{
    //! \param[in]    _c        the character to test
    //! \param[in]        _includeNumbers        when set to true, add numbers to the list of possible delimiters (default is false)
    //! \brief        return true if the given character is a proper delimiter character (dollar, dot, quote or open/close square bracket)
    static bool IsDelimiter( const wchar_t _c, const bool _includeNumbers = false );

    //! \brief    find the position of the next path delimiter character
    //! \param[in]        _pos                current iterator position in path String
    //! \param[in]        _end                end iterator position in path String
    //! \param[in]        _includeNumbers        when set to true, add numbers to the list of possible delimiters (default is false)
    //! \return        the position of the next delimiter character in the path String
    static std::wstring::const_iterator NextDelimiter( std::wstring::const_iterator _pos, const std::wstring::const_iterator _end, const bool _includeNumbers = false );

    //! \brief    convert current Generic as a Json Object and return the identifier element
    //!    \param[in]    _generic        a reference over the Generic type to search
    //!    \param[in]    _identifier        the identifier of the element to extract in the Object
    //! \param[in]    _pos            current iterator position in path String, used to identified the sub-String to use to continue the parsing
    //! \param[in]    _end            end iterator position in path stringobject
    //! \return        a reference over a Generic type
    static Status::And< std::reference_wrapper< const JsonGeneric > > Object( const JsonGeneric & _generic, const std::wstring & _identifier, const std::wstring::const_iterator _pos, const std::wstring::const_iterator _end );

    //! \brief    convert current Generic as a Json Array and return the element at a given index
    //!    \param[in]    _generic        a reference over the Generic type to search
    //!    \param[in]    _index            the index of the element to extract in the Array
    //! \param[in]    _pos            current iterator position in path String, used to identified the sub-String to use to continue the parsing
    //! \param[in]    _end            end iterator position in path stringobject
    //! \return        a reference over a Generic type
    static Status::And< std::reference_wrapper< const JsonGeneric > > Array( const JsonGeneric & _generic, const unsigned long long _index, const std::wstring::const_iterator _pos, const std::wstring::const_iterator _end );
};


bool PathParser::IsDelimiter( const wchar_t _c, const bool _includeNumbers )
{
    return _c == Char::dollar || _c == Char::dot || _c == Char::singleQuote || _c == Char::openSquareBracket || _c == Char::closeSquareBracket || ( _includeNumbers && _c >= Char::numbers.front() && _c <= Char::numbers.back() );
}


std::wstring::const_iterator PathParser::NextDelimiter( std::wstring::const_iterator _pos, const std::wstring::const_iterator _end, const bool _includeNumbers )
{
    while( _pos != _end && !IsDelimiter( *_pos, _includeNumbers ) )
        ++_pos;
    return _pos;
}


Status::And< std::reference_wrapper< const JsonGeneric > > PathParser::Object( const JsonGeneric & _generic, const std::wstring & _identifier, const std::wstring::const_iterator _pos, const std::wstring::const_iterator _end )
{
    const auto & obj{ _generic.As< Json::Object >() };
    auto itObj{ obj.data.find( _identifier ) };
    if( itObj == obj.data.cend() )
        return { StatusFailure( PathErrorUnknownObjectIdentifier, _identifier ) };
    return JSON::Path( *itObj->second, std::wstring{ _pos, _end } );
}


Status::And< std::reference_wrapper< const JsonGeneric > > PathParser::Array( const JsonGeneric & _generic, const unsigned long long _index, const std::wstring::const_iterator _pos, const std::wstring::const_iterator _end )
{
    const auto & arr{ _generic.As< Json::Array >() };
    auto itData{ arr.data.cbegin() };
    if( _index >= arr.data.size() )
        return { StatusFailure( PathErrorOutOfArrayBoundaries, std::to_wstring( _index ), std::to_wstring( arr.data.size() ) ) };
    std::advance( itData, _index );
    return JSON::Path( **itData, std::wstring{ _pos, _end } );
}


Status::And< std::reference_wrapper< const JsonGeneric > > JSON::Path( const JsonGeneric & _generic, std::wstring _expression )
{
    // end of search:
    if( _expression.empty() )
        return { StatusOk, _generic };

    // first useful character:
    auto pos{ PathParser::NextDelimiter( _expression.cbegin(), _expression.cend() ) };

    // root element to query:
    if( *pos == Char::dollar ) {
        pos = PathParser::NextDelimiter( std::next( pos ), _expression.cend() );
        if( pos == _expression.cend() ) // expressions containing only dollar
            return { StatusOk, _generic };
    }

    // dot-notated child:
    if( *pos == Char::dot ) {
        const auto identifierBegin{ std::next( pos ) };
        pos = PathParser::NextDelimiter( identifierBegin, _expression.cend() );
        return PathParser::Object( _generic, std::wstring{ identifierBegin, pos }, pos, _expression.cend() );
    }

    // open square bracket: can be both bracket-notated child or Array index:
    if( *pos != Char::openSquareBracket )
        return { StatusFailure( PathErrorMissingOpenSquareBracket ) };
    pos = PathParser::NextDelimiter( std::next( pos ), _expression.cend(), true );

    // bracket notation child:
    if( *pos == Char::singleQuote ) {
        const auto identifierBegin{ std::next( pos ) };
        pos = PathParser::NextDelimiter( identifierBegin, _expression.cend() );
        const std::wstring identifier{ identifierBegin, pos };
        pos = PathParser::NextDelimiter( std::next( pos ), _expression.cend() );
        return PathParser::Object( _generic, identifier, std::next( pos ), _expression.cend() );
    }

    // Array index:
    const auto indexEnd{ PathParser::NextDelimiter( pos, _expression.cend() ) };
    return PathParser::Array( _generic, static_cast< unsigned long long >( std::stoull( std::wstring{ pos, indexEnd } ) ), std::next( indexEnd ), _expression.cend() );
}


} // Moth