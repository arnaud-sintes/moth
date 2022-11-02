#include "Json.h"

 namespace Moth {


// -----------------
// json reflection of a status:
JsonReflectionPair( Status, m_armed, armed, m_success, success, m_identifier, identifier, m_originFile, originFile, m_originLine, originLine, m_originFunction, originFunction, m_message, message, m_child, child );



// -----------------
// json reflection of a status and:
JsonReflectionT( Status::And, status, object );



 } // Moth