#pragma once

//! \brief ANSI to Wide compile time conversion:
#define CONCAT2( X, Y ) X##Y
#define CONCAT( X, Y ) CONCAT2( X, Y )
#define STRINGIFY2( X ) #X
#define STRINGIFY( X ) STRINGIFY2( X )
#define A2W( X ) CONCAT( L, STRINGIFY( X ) )