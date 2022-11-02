// generic code generator from multiple arguments (up to 40) with specific separator
#define _EXPANDER_01( _SEP, _FNC, _VAL )		_FNC( _VAL ) _SEP()
#define _EXPANDER_02( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_01( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_03( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_02( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_04( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_03( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_05( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_04( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_06( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_05( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_07( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_06( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_08( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_07( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_09( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_08( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_10( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_09( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_11( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_10( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_12( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_11( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_13( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_12( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_14( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_13( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_15( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_14( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_16( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_15( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_17( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_16( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_18( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_17( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_19( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_18( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_20( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_19( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_21( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_20( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_22( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_21( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_23( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_22( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_24( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_23( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_25( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_24( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_26( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_25( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_27( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_26( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_28( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_27( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_29( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_28( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_30( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_29( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_31( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_30( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_32( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_31( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_33( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_32( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_34( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_33( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_35( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_34( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_36( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_35( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_37( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_36( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_38( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_37( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_39( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_38( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_40( _SEP, _FNC, _VAL, ... )	_EXPANDER_01( _SEP, _FNC, _VAL ) _EXPANDER_39( _SEP, _FNC, __VA_ARGS__ )

#define _COUNTARG_( X, X40, X39, X38, X37, X36, X35, X34, X33, X32, X31, X30, X29, X28, X27, X26, X25, X24, X23, X22, X21, X20, X19, X18, X17, X16, X15, X14, X13, X12, X11, X10, X09, X08, X07, X06, X05, X04, X03, X02, X01, N, ... ) N
#define _COUNTARG( ... ) _COUNTARG_( 0, __VA_ARGS__, 40, 39, 38, 37, 36, 35, 34, 33, 32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 09, 08, 07, 06, 05, 04, 03, 02, 01, 00 )

#define _EXPANDER__( _SEP, _FNC, _VAL, ... )	_EXPANDER_ ## _VAL( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_( _SEP, _FNC, _VAL, ... )		_EXPANDER__( _SEP, _FNC, _VAL, __VA_ARGS__ )
#define Expander( _SEP, _FNC, ... )				_EXPANDER_( _SEP, _FNC, _COUNTARG( __VA_ARGS__ ), __VA_ARGS__ )


// generic code generator from multiple argument pairs (up to 20) with specific separator
#define _EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 )		_FNC( _VAL1, _VAL2 ) _SEP()
#define _EXPANDER_PAIR_04( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_02( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_06( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_04( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_08( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_06( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_10( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_08( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_12( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_10( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_14( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_12( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_16( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_14( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_18( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_16( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_20( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_18( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_22( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_20( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_24( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_22( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_26( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_24( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_28( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_26( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_30( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_28( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_32( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_30( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_34( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_32( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_36( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_34( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_38( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_36( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_40( _SEP, _FNC, _VAL1, _VAL2, ... )	_EXPANDER_PAIR_02( _SEP, _FNC, _VAL1, _VAL2 ) _EXPANDER_PAIR_38( _SEP, _FNC, __VA_ARGS__ )

#define _EXPANDER_PAIR__( _SEP, _FNC, _VAL, ... )			_EXPANDER_PAIR_ ## _VAL( _SEP, _FNC, __VA_ARGS__ )
#define _EXPANDER_PAIR_( _SEP, _FNC, _VAL, ... )			_EXPANDER_PAIR__( _SEP, _FNC, _VAL, __VA_ARGS__ )
#define ExpanderPair( _SEP, _FNC, ... )						_EXPANDER_PAIR_( _SEP, _FNC, _COUNTARG( __VA_ARGS__ ), __VA_ARGS__ )


// --------------------------------------------------------------------------------------------------------------
// --------------------------------------------------------------------------------------------------------------


#if 0
// *** use this way:

// define custom separator and function:
#define _Custom_Sep()								;
#define _Custom_Fnc( _DATA )						Register< _DATA >()

// define custom expander macro
#define _Custom( ... )								Expander( _Custom_Sep, _Custom_Fnc, __VA_ARGS__ ) 


// then call:
_Custom( Struct1, Struct2, Struct3 );


// it will generate then:
Register< Struct1 >();
Register< Struct2 >();
Register< Struct3 >();

#endif


#if 0
// *** use this way:

// define custom separator and function:
#define _CustomPair_Sep()							;
#define _CustomPair_Fnc( _DATA1, _DATA2 )			Register< _DATA1, _DATA2 >()

// define custom expander macro
#define _CustomPair( ... )							ExpanderPair( _CustomPair_Sep, _CustomPair_Fnc, __VA_ARGS__ ) 


// then call:
_CustomPair( Struct1, Struct2, Struct3, Struct4 );


// it will generate then:
Register< Struct1, Struct2 >();
Register< Struct3, Struct4 >();

#endif