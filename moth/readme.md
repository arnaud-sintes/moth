# 🦋 *moth* library

## Purpose

This folder provides multiple C++20/Clang compatible source files to properly create HTML structured web-pages using *emscripten*.

## Content

### [moth]

Single *header* to ease *moth* inclusions, with a single *source* file to ease the *emcc* compilation.

### AnsiToWideMacros

Macros to convert ANSI string declarations to Wide chars (UCS-2), adding an "*L*" prefix.

### Base64

*Base64* encoding/decoding (*unused yet*).

```c++
const auto b64data{ Base64::Encode( rawData ) }; // rawData is a Buffer object
const auto rawData{ Base64::Decode( b64Data ) }; // rawData is a std::vector
```

### Buffer

Dealing with raw memory buffers (weak possession container), allowing *to/from* conversions.

```c++
Buffer buffer{ pData, 128 }; // map a memory zone
auto data{ buffer.To< std::string >() };
auto data{ buffer.To< std::vector< unsigned char > >() };
```

```c++
auto buffer{ Buffer::From( data ) }; // data is a std::string
```

### Ctti

*Compile-Time Type Information* helpers, typically to compute *class names* and *FNV1-a hashes* at compile time.

```c++
struct Test{};
const std::string test{ Ctti::ShortName< Test >() }; // test == "Test"
```

- *Name* returns the complete name of the object
- *ShortName* returns the short name (no namespace/class prefix) of the object
- *FNV1a* returns the FNV1a hash of the object name

### Data

*moth* HTML components *Data* object, used typically for generic serialization/deserialization, providing also a *DataPool* object (which can be seen as the *Model* part of the *MVC* pattern for a single isolated component).

### Html

Provides C++ equivalences of the following HTML components:

- **Body**, a "*div*" element without any parent
- **Division**, all-purposes "*div*" element
- **Label**, a "*span*" element
- **Image**, an "*img*" element
- **PushButton**, a "*button*" element
- **ToggleButton**, a "*button*" element with toggling capabilities

  > Note *ToggleButton* can rely over a "*toggled*" specific attribute

> Note every elements can rely over a "*disabled*" specific attribute

### IndexedMap

An *unordered_map* that respects the emplacing order.

### Item

*moth* hierarchized (parent/child) item internal system.

### Js

*JavaScript* calls wrapper, covering:

- *Console* accesses (*Log, Warning, Error*)
- *DOM* manipulation (*create/remove element, add/remove/toggle class, set/remove/toggle attribute, set text content, set on click event, set image source*)
- *initialization* (setting page title and icon)

### Json

#### Purpose

The main purpose is to provide an easy way to perform bilateral <b>C++ reflection</b> in JSON format, based over:

- a C++ to JSON types (<i>Null, numbers, Boolean, String, Array & Object</i>) conversion via a <b>serialization</b> and <b>deserialization</b> process
- a Json types textual <b>stringification</b> and <b>parsing</b> process, with optional <i>beautification</i>

> **Note** this JSON library provides also a partial support for [JsonPath expressions](https://docs.oracle.com/cd/E60058_01/PDF/8.0.8.x/8.0.8.0.0/PMF_HTML/JsonPath_Expressions.htm) (refer to the dedicated section at the end).

#### Files

##### JsonTypes.hpp

Specific stream <b>serialization</b> & <b>deserialization</b> operators were then declared in <b>JsonTypes.hpp</b> to properly offers:

- a conversion from any C++ type to a <b>Json::Number</b>
- a conversion from a C++ bool to a <b>Json::Boolean</b>
- a conversion from a C++ String (<i>ANSI</i> or <i>UCS-2/UTF-16</i>) to a <b>Json::String</b>
- a Generic way to stack and unstack any C++ Object in a <b>Json::Array</b>
- a Generic way to insert and retrieve any C++ Object in a <b>Json::Object</b>, using a <b>Json::Pair</b> intermediate

##### JsonStdObjects.hpp

The <b>JsonStdObjects.hpp</b> file groups the specific <b>serialization</b> & <b>deserialization</b> operators related to the following <a href="https://en.cppreference.com/w/cpp/header">standard library</a> objects:

- <a href="https://en.cppreference.com/w/cpp/atomic/atomic">std::atomic</a>, with JSON Storage depending of its subtype
- <a href="https://en.cppreference.com/w/cpp/utility/functional/reference_wrapper">std::reference_wrapper</a>, with JSONStorage depending of its subtype
- <a href="https://en.cppreference.com/w/cpp/memory/unique_ptr">std::unique_ptr</a>, with JSONStorage depending of its subtype or <b>Json::Null</b> if empty
- <a href="https://en.cppreference.com/w/cpp/utility/optional">std::optional</a>, with JSONStorage depending of its subtype or <b>Json::Null</b> if empty
- <a href="https://en.cppreference.com/w/cpp/utility/Pair>">std::pair</a>, using a <b>Json::Array</b> Storage of two variable values
- <a href="https://en.cppreference.com/w/cpp/chrono/duration/duration>">std::chrono::duration</a>, using a <b>Json::Number</b> Storage

##### JsonStdContainers.hpp

The <b>JsonStdContainers.hpp</b> file groups the specific <b>serialization</b> & <b>deserialization</b> operators related to the following standard library containers:

- <a href="https://en.cppreference.com/w/cpp/container/list">std::list</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/vector">std::vector</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/Array">std::array</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/forward_list">std::forward_list</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/deque">std::deque</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/stack">std::stack</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/queue">std::queue</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/priority_queue">std::priority_queue</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/set">std::set</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/unordered_set">std::unordered_set</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/multiset>">std::multiset</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/unordered_multiset">std::unordered_multiset</a>, using a <b>Json::Array</b> Storage
- <a href="https://en.cppreference.com/w/cpp/container/map/find">std::map</a>, using a <b>Json::Array</b> Storage of <b>data pairs</b> (<i>key,value</i>) by default, but switching to a more natural <b>Json::Object</b> Storage if the map key is a <b>String</b>
- <a href="https://en.cppreference.com/w/cpp/container/unordered_map">std::unordered_map</a>, using a <b>Json::Array</b> Storage of <b>data pairs</b> (<i>key,value</i>) by default, but switching to a more natural <b>Json::Object</b> Storage if the map key is a <b>String</b>
- <a href="https://en.cppreference.com/w/cpp/container/multimap">std::multimap</a> using a <b>Json::Array</b> Storage of <b>data pairs</b> (<i>key,value</i>)
- <a href="https://en.cppreference.com/w/cpp/container/unordered_multimap">std::unordered_multimap</a> using a <b>Json::Array</b> Storage of <b>data pairs</b> (<i>key,value</i>)

##### JsonCoreObjects.hpp

The <b>JsonCoreObjects.hpp</b> file will group the specific <b>serialization</b> & <b>deserialization</b> operators related to the *moth* core objects.

##### JsonReflectionMacros.h

Finally, the <b>JsonReflectionMacros.h</b> file centralizes high-level <b>preprocessor definitions</b> that can be used to ease the C++ JSON reflection:

- <i>JsonReflection</i>, to easily defines <b>serialization</b> & <b>deserialization</b> related to a <b>custom user structure or class</b>
- *JsonReflectionT*, to easily defines <b>serialization</b> & <b>deserialization</b> related to a <b>custom user templated structure or class</b>

#### Link time resolution

Note the resolution of user custom types will happen at **link time**, allowing the user to centralize every serialization declarations in a unique file.

#### Usage

##### Serialization and deserialization

The <b>serialization</b> and <b>deserialization</b> process is very easy, using <b>Json::Array</b> or <b>Json::Object</b> with stream operators:

```cpp
// Json Array declaration:
Json::Array arr;

// C++ type stacking in the Json Array:
const int a{ 42 };
arr << a;

// C++ type unstacking from the Json Array:
int a;
arr >> a; // a == 42
```


Note <b>Json::Object</b> requires the usage of an intermediate <b>Json::Pair</b> in order to properly identify the element in the Object:

```cpp
// Json Object declaration:
Json::Object obj;

// C++ type insertion in the Json Object, as element 'a':
const int a{ 42 };
obj << Json::Pair{ L"a", a };

// C++ type get from the Json Object:
int a;
obj >> Json::Pair{ L"a", a }; // a == 42
```


Note also you can also manipulate <i>arrays</i> and <i>objects</i> via their <b>pointer definition</b>, which is very useful when you need to store, return or forward them using their abstract representation (JsonGeneric::Ptr):

```cpp
JsonGeneric::Ptr GetObjectAsGeneric()
{
	// Json Object pointer declaration:
	Json::Object::Ptr obj;

	// C++ type insertion in the Json Object, as element 'a':
	const int a{ 42 };
	*obj << Json::Pair{ L"a", a };

	// we can safely implicitely cast the pointer as a Generic and return it
	return obj;
}
```

##### Stringification and parsing

Getting the <b>String representation</b> of a Json::Object (or Json::Array) is very easy using the <b>Stringify()</b> method:

```cpp
// Json Object declaration:
Json::Object obj;

// C++ constant declarations:
const int a{ 42 };
const double b{ 8.7 };
const std::wstring c{ L"hello" };

// variables serialization in Object:
obj << Json::Pair{ L"a", a };
obj << Json::Pair{ L"b", b };
obj << Json::Pair{ L"c", c };

// stringification:
const auto compact{ obj.Stringify() };
const auto readable{ obj.Stringify( L"\t" ) };
```

the "compact" representation being:

```json
{"a":42,"b":8.7,"c":"hello"}
```

and the "readable" (<i>beautified</i>) representation being:

```json
{
	"a": 42,
	"b": 8.7,
	"c": "hello"
}
```


Of course, building a Json::Object or Json::Array from a valid JSON stringified text is also easy using the <b>Parse()</b> method:

```cpp
auto statusAndObj{ Json::Object::Parse( compact ) };
```

> **Note** that a stringification process can lead to the introduction of <b>Null</b> values in the output String, e.g. when using <i>std::unique_ptr</i> or <i>std::optional</i> with empty content.

> **Note** also that the parsing process takes properly care of these Null values, alongside possible <b>missing values</b> in Json::Object (we can typically omit completely empty values).

> **Note** *Parse()* returns a **status** and the value, taking care properly of errors


Of course, you may want to <b>serialize</b> directly the content of a C++ std <b>container</b> into a <b>Json::Array</b>, which is very easy using the <b>Serialize()</b> static method:

```cpp
const std::list< int > a{ 1, 2, 3, 4 };
auto arr{ Json::Array::Serialize( a ) };
```


the <b>deserialization</b> process being as easy using the <b>DeSerialize()</b> method:

```cpp
const std::list< int > a{ 1, 2, 3, 4 };
const auto arr{ Json::Array::Serialize( a ) };
const auto b{ arr.DeSerialize< decltype( a ) >() }; // b={1, 2, 3, 4}
```


You can of course cumulate both serialization and stringify process (for <i>marshalling</i> operations):

```cpp
const std::list< int > a{ 1, 2, 3, 4 };
auto s{ Json::Array::Serialize( a ).Stringify() }; // s="[1,2,3,4]"
//...
const auto statusAndS{ Json::Array::Parse( s ) };
System::Assert( statusAndS, L"JSON parsing issue" );
const auto b{ statusAndS.object->DeSerialize< std::list< int > >() }; // b={1, 2, 3, 4}
```

##### Complex structures

The main advantage of the JSON lib is the reflection of <b>complex structures</b>.

Considering the following custom structure:

```cpp
struct MyStruct
{
	int a;
	double b;
};
```

What you need to do is to define how the stream <b>serialization</b> and <b>deserialization</b> process will work with this specific structure.

This can be done "the hard way", by writing down manually both serialization and deserialization stream methods:

```cpp
// serialization:
JsonGeneric::Ptr operator << ( const Json &, const MyStruct & _data )
{
	Json::Object::Ptr Object; // we create a new Json::Object via pointer, before inserting the data
	*Object << Json::Pair{ L"a", _data.a };
	*Object << Json::Pair{ L"b", _data.b };
	return Object; // we return the Object as a JsonGeneric::Ptr
}

// deserialization:
const JsonGeneric & operator >> ( const JsonGeneric & _generic, MyStruct & _data )
{
	const auto & Object{ _generic.As< Json::Object >() }; // we just need to cast the input Generic in the proper type (here: Json::Object), before accessing the data
	Object >> Json::Pair{ L"a", _data.a };
	Object >> Json::Pair{ L"b", _data.b };
	return _generic;
}
```

or the "the easy way", just by using the following reflection preprocessor definition:

```cpp
JsonReflection( MyStruct, a, b );
```


Now that JSON reflection is ready for our specific structure, we can easily deal with regular <b>serialization</b> and <b>deserialization</b> process:

```cpp
const MyStruct a{ 42, 4.7 }:
const auto a{ Json::Object::Serialize( a ) };
const auto b{ obj.DeSerialize< MyStruct >() }; // b={ a=42, b=4.7 }
```


Finally, now that everything is well undestood, we can safely play with complex structures:

```cpp
struct Test2
{
	double a;
	std::list< std::vector< int > > b;
};
JsonReflection( Test2, a, b );

struct Test1
{
	int a;
	std::list< Test2 > b;
};
JsonReflection( Test1, a, b );
```

dealing easily with both serialization, stringification, parsing and deserialization:

```cpp
const Test1 test1{ 42, { Test2{ 2.8, { { 1, 2, 3 }, { 4, 5, 6 }, } }, Test2{ 3.4, { { 7, 8, 9 }, { 10, 11, 12 }, } }, } };
const auto objJsonReadable{ Json::Object::Serialize( test1 ).Stringify( L"\n" ) };
const auto statusAnd{ Json::Object::Parse( objJsonReadable ) };
System::Assert( statusAnd, L"JSON parsing issue" );
const auto test2{ statusAnd.object->DeSerialize< Test1 >() }; // test2 == test1
```

"objJsonReadable" being:

```json
{
	"a": 42,
	"b": [
		{
			"a": 2.8,
			"b": [
				[
					1,
					2,
					3
				],
				[
					4,
					5,
					6
				]
			]
		},
		{
			"a": 3.4,
			"b": [
				[
					7,
					8,
					9
				],
				[
					10,
					11,
					12
				]
			]
		}
	]
}
```

#### JsonPath Expressions

The JSON library provides a partial support for <a href="https://docs.oracle.com/cd/E60058_01/PDF/8.0.8.x/8.0.8.0.0/PMF_HTML/JsonPath_Expressions.htm">JsonPath expressions</a>, via possible calls of the <b>Path()</b> method on both Json::Object and Json::Array.

##### Support and limitations


Please note that only the following <b>operators</b> are currently supported in the JsonPath expressions:

<table>
<tr><th>Operator</th><th>Description</th></tr>
<tr><td>$</td><td>root element to query</td></tr>
<tr><td>.name</td><td>dot-notated child</td></tr>
<tr><td>['name']</td><td>bracket-notated child</td></tr>
<tr><td>[Number]</td><td>Array index</td></tr>
</table>

<b>Functions</b> and <b>filter</b> operators are <b>not supported yet</b>, nor the following <b>operators</b>:

<table>
<tr><th>Operator</th><th>Description</th></tr>
<tr><td>\@</td><td>the current node being processed by a filter predicate</td></tr>
<tr><td>*</td><td>wildcard</td></tr>
<tr><td>..</td><td>deep scan</td></tr>
<tr><td>['name',..]</td><td>bracket-notated children</td></tr>
<tr><td>[Number,..]</td><td>Array indexes</td></tr>
<tr><td>[start:end]</td><td>Array slice operator</td></tr>
<tr><td>[?(exp)]</td><td>filter Boolean expression</td></tr>
</table>


#### JsonPath expressions example

```cpp
// set values (same as in "objJsonReadable"):
const Test1 test1{ 42, { Test2{ 2.8, { { 1, 2, 3 }, { 4, 5, 6 }, } }, Test2{ 3.4, { { 7, 8, 9 }, { 10, 11, 12 }, } }, } };

// Object serialization:
const auto obj{ Json::Object::Serialize( test1 ) };

// JsonPath expression accesses (note: every Path() result need to be checked):
obj.Path< Test1 >( L"$" ); // == test1
obj.Path< int >( L"$.a" ); // == 42
obj.Path< int >( L"$['a']" ); // == 42
obj.Path< double >( L"$.b[0].a" ); // == 2.8
obj.Path< double >( L"$.b[1]['a']" ); // == 3.4
obj.Path< int >( L"$.b[0].b[0][0]" ); // == 1
obj.Path< int >( L"$.b[1].b[1][2]" ); // == 12
obj.Path< int >( L"$['b'][1]['b'][1][1]" ); // == 11
```


Note also that JsonPath expressions are not limited to basic types: you can also get complex elements like <b>containers</b>, which is particularly useful considering than a Json::Object can be naturally reflected as a *std::map< std::wstring, xxx >*:

```cpp
// Array pasing from data file:
const auto data{ File::LoadText( L"data.json" ) }; // [{"a":[1,2,3,4],"b":{"a":true,"b":false,"c":true}},{"a":[5,6,7,8],"b":{"a":false,"b":true,"c":false}}]
System::Assert( data, L"Loading file issue" );
const auto arr{ Json::Object::Parse( *data.object ) };
System::Assert( arr, L"file parsing issue" );

// JsonPath expression accesses (note: every Path() result need to be checked):
arr.object->Path< std::vector< int > >( L"$[0].a" ); // == { 1, 2, 3, 4 }
arr.object->Path< std::map< std::wstring, bool > >( L"$[1].b" ); // == { { L"a", false }, { L"b", true }, { L"c", false } }
```

**Note** *Path()* returns a **status** and the value, taking care properly of errors, so you need to check the return before accessing the value:

```c++
auto statusAndObj{ obj.Path< int >( L"$" ) };
if( statusAndObj )
    statusAndObj.object->a; // ==42
```

### Lambda

Allowing JavaScript to call properly C++ lambdas, by providing a way to add/remove and call registered C++ lambdas using a unique identifier.

### Status

#### Purpose

*Status* introduces a common object that can be used to perform proper **error management in *C++*** without the usage of *exceptions*.

#### Declaration

Statuses must be declared the following way:

```c++
// success statuses:
DeclStatus( EverythingOkNoMsg );
DeclStatus( EverythingOk, L"Everything's good" );
DeclStatus( FileAlreadyCached, L"The file '%' was already put in cache" );

// error statuses:
DeclStatus( UnknownNoMsg );
DeclStatus( Unknown, L"Unknown error" );
DeclStatus( FileNotFound, L"The file '%' was not found" );
DeclStatus( JSONIllformed, L"The Json file '%' is ill-formed: '%' at position (%,%)" );
```

> **Note** at this level you can define uniformly *success* and *failure* statuses.

A status is declared using the "*DeclStatus*" macro, providing a status **strong type** and an *optional* UCS-2 **message**.

> **Note** you can avoid the declaration of a specific success status without message by using the default **StatusOk**.
>
> **Note** also a failure status MUST expose a message.

Statuses' optional message can expose multiple **tokens** using the "'%" characters in order to feed specific data dynamically depending of the execution context.

#### Creation

##### Success status

**Success** statuses are created the following way:

```c++
StatusSuccess( EverythingOk );
StatusSuccess( FileAlreadyCached, L"C:/myfile.txt" );
StatusOk;
```

Providing then the status **strong type** and optionally the value(s) used for **tokens' replacement**.

> **Note** an assertion failure occurs if too many values are provided regarding the message's token count.

When dealing with simple success status, you can safely rely over the usage of "**StatusOk**", creating a common *generic success* status.

##### Error status

**Error** statuses are created the following way:

```c++
StatusFailure( Unknown );
StatusFailure( FileNotFound, L"C:/myFile.txt" );
StatusFailure( JSONIllformed, L"C:/myFile.json", L"missing identifier", 10, 20 );
```

Using the exact same syntax as for success statuses.

#### Error handling

*Status* object are meant to be **returned** by functions (enforcing then the usage of the RAII idiom in your code):

```c++
DeclStatus( FileDoesntExists, L"The file '%' doesn't exists: %" );

Status FileExists( const std::wstring & _filePath )
{
    std::error_code ec;
    if( !std::filesystem::exists( _filePath, ec ) )
		return StatusFailure( FileDoesntExists, _filePath, System::MbcsToWide( ec.message() ) );
    return StatusOk;
}
```

#### Checking policy

Statuses <u>requires</u> to be **disarmed** in order to ensure proper error management in the application.

To **disarm** a status, you can:

- **check** the status, using:

  - the ***boolean* operator** (most used)

    ```c++
    // checking example #1:
    auto fileExists{ FileExists( L"C:/robot_config.json" ) };
    if( !fileExists ) // <- properly checked here
        return StatusFailure( InitRobotFailed );
    ```

    ```c++
    // checking example #2:
    Assert( FileExists( L"C:/robot_config.json" ) ); // <- implicit check via Assert() call
    ```

  - a call to the ***Is***() method (type comparison)

    > *please check the dedicated chapter below for examples*

  - an implicit call to *Is*() via the **comparison operators** or a ***Contains***() call

    > *please check the dedicated chapter below for examples*

- **stack** the status in a *parent status*, using the *<<* operator

  > *please check the dedicated chapter below for examples*

- explicitly call the ***Disarm***() method (not recommended)

  ```c++
  auto fileExists{ FileExists( L"C:/robot_config.json" ) };
  fileExists.Disarm(); // <- explicit disarming
  ```

At its destruction, the *status* object **assert** for its proper disarming.

In case of failure it will dump information (output) about the status (*type*, *identifier* and *message*), in order to help the developer to find the issue root cause:

```
STATUS NOT DISARMED !
> Status stack:
[FAILURE] [InitRobotFailed] Fail to init the robot
        [moth::InitRobot] C:\project\1.1.0\backend\ApplicationDemo.Exe.Cpp\[main].cpp(53)
```

#### Stacking

A status can be *stacked* using the "<<" operator, which is particularly useful when dealing with low-level error that are handled as an higher-level:

```c++
DeclStatus( InitRobotFailed, L"Fail to init the robot" );

Status InitRobot()
{
	auto fileExists{ FileExists( L"C:/robot_config.json" ) };
	if( !fileExists )
		return StatusFailure( InitRobotFailed ) << std::move( fileExists );

	// ...
	return StatusOk;
}
```

> Remember that stacking an object is enough to *disarm* it (properly took in account).

Note you can stack only a **single child** status object.

#### Dump

A status content can be **dumped** calling the *Dump()* method, showing additional *origin* information optionally, with an optional *log* dump, with or without its *children* information dump:

```
[FAILURE] [InitRobotFailed] Fail to init the robot
        [moth::InitRobot] C:\project\1.1.0\backend\ApplicationDemo.Exe.Cpp\[main].cpp(52)
[FAILURE] [FileDoesntExists] The file 'C:/robot_config.json' doesn't exists: The operation completed successfully.
        [moth::FileExists] C:\project\1.1.0\backend\ApplicationDemo.Exe.Cpp\[main].cpp(41)
```

#### Copy and move

Statuses are both *copiable* and *movable*.

Please note that both operations doesn't change the original **arming status**.

#### Identification

The **textual identification** of a status (determined at compile time using its strong C++ type) is accessible using the **Identifier**() method or the **string conversion operator**.

#### Type testing

If the *boolean operator* is enough in most of the cases to know if we're facing a **success** or a **failure**, you need sometimes to get more information.

#### Is

A status can be evaluated regarding a certain type, using the **Is**() method:

```c++
auto fileExists{ FileExists( L"C:/robot_config.json" ) };
if( fileExists.Is< FileDoesntExists >() ) // <- check for exact "FileDoesntExists" error status
    return StatusFailure( InitRobotFailed ) << std::move( fileExists );
```

> **Note** this kind of evaluation concerns only the current status element (children are not evaluated)

##### Comparison

Of course, we can also compare two statuses using regular comparison operators ("==" and "!="):

```c++
auto fileExists{ FileExists( L"C:/robot_config.json" ) };
if( fileExists == StatusFailure( FileDoesntExists ) ) // <- comparison
    return StatusFailure( InitRobotFailed ) << std::move( fileExists );
```

##### Contains

We can also check if a status **contains** a certain status type at every level (current or children):

```c++
auto status{ InitRobot() };
if( status.Contains< FileDoesntExists >() ) // <- child contains the "FileDoesntExists" error status
    return 0;
```

##### Status and

Because we always want to **return** objects (modern C++ paradigm), we will quickly encounter the case where we need to return both a **status and an object**.

To do so, an **And** object is available, providing an easy way to return a status alongside an optional object in case of success, with access facilitations:

```c++
DeclStatus( UnableToOpenFile, L"Unable to open the file '%'" );
DeclStatus( UnableToReadFile, L"Unable to read the file '%'" );

// return a status AND a file content using a vector:
Status::And< std::vector< unsigned char > > LoadFile( const std::wstring & _filePath )
{
	// try to open file:
    std::ifstream srcStream;
    srcStream.open( _filePath, std::ifstream::binary );
 	if( !srcStream.good() )
		return { StatusFailure( UnableToOpenFile, _filePath ) }; // error status

	// try to read file content:
	std::filebuf * pFileBuffer{ srcStream.rdbuf() };
	const auto size{ pFileBuffer->pubseekoff( 0, srcStream.end, srcStream.in ) };
	pFileBuffer->pubseekpos( 0, srcStream.in );
	std::vector< unsigned char > fileData;
	fileData.resize( size );
	if( pFileBuffer->sgetn( reinterpret_cast< char * >( fileData.data() ), size ) != size )
		return { StatusFailure( UnableToReadFile, _filePath ) }; // error status

	// return data:
	return { StatusOk, std::move( fileData ) }; // success status with content
}
```

```c++
DeclStatus( InitRobotFailed, L"Fail to init the robot" );

Status InitRobot()
{
	// try to load file content:
	auto fileContent{ LoadFile( L"C:/robot_config.json" ) };
	if( !fileContent ) // <- fileContent is a Status AND... but can be checked as a regular status
		return StatusFailure( InitRobotFailed ) << std::move( fileContent );

	// is there any file content ?
	Assert( !fileContent.object->empty() ); // <- access the object content here

	return StatusOk;
}
```

**Status::And<>** provides:

- a direct access to the returned status:

  ```c++
  fileContent.status; // <- this is a regular Status
  ```

- a direct access to the optional object content:

  ```c++
  fileContent.object; // <- this a std::optional< _Type >
  ```

- a **boolean operator** redirecting to the *Status*' one

- a status r-value reference conversion operator, allowing easy stacking:

  ```c++
  auto fileContent{ LoadFile( L"C:/robot_config.json" ) };
  if( !fileContent )
  	return StatusFailure( InitRobotFailed ) << std::move( fileContent ); // <- not it seems we're moving the whole "StatusAnd<>" but in fact we're moving only the "status" variable, so we're doing regular Status stacking
  ```

#### Json reflection

*Statuses* can be naturally reflected in Json, allowing then network **marshalling**:

```c++
auto status{ InitRobot() };

// serialization / stringification (with beautification):
const auto jsonStatus{ Json::Object::Serialize( status ).Stringify( L"\t" ) };
```

```json
{
	"m_armed": true,
	"m_identifier": "InitRobotFailed",
	"m_message": "Fail to init the robot",
	"m_originFile": "C:\\project\\1.1.0\\backend\\ApplicationDemo.Exe.Cpp\\[main].cpp",
	"m_originFunction": "moth::InitRobot",
	"m_originLine": 67,
	"m_stack": [
		{
			"m_armed": false,
			"m_identifier": "UnableToOpenFile",
			"m_message": "Unable to open the file 'C:/robot_config.json'",
			"m_originFile": "C:\\project\\1.1.0\\backend\\ApplicationDemo.Exe.Cpp\\[main].cpp",
			"m_originFunction": "moth::LoadFile",
			"m_originLine": 43,
			"m_stack": [
			],
			"m_success": false
		}
	],
	"m_success": false
}
```

```c++
// parsing / deserialization:
auto marshalledStatus{ Json::Object::Parse( jsonStatus ).DeSerialize< Status >() };
```

### StdIncludes

Single-header **standard library** inclusions.

### System

All purposes methods (output messages, contract-programming using *assertions*, *Mbcs/Wide conversions* etc.).

### VariadicExpanderMacros

Helper to deal with code generation based over variadic arguments.

### WebSocket

A very light C++ abstraction of *emscripten* client *WebSocket*.
