//! \brief declare statuses with message:
#define DeclStatus( _Type, ... ) struct _Type { static constexpr const wchar_t * message{ __VA_ARGS__ }; }

//! \brief specific success status creation:
#define StatusSuccess( _Type, ... ) Moth::Status::Success< _Type >( __FILE__, __LINE__, __FUNCTION__, ## __VA_ARGS__ )

//! \brief generic "ok" success status creation:
#define StatusOk StatusSuccess( Status::GenericSuccess )

//! \brief specific failure status creation:
#define StatusFailure( _Type, ... ) Moth::Status::Failure< _Type >( __FILE__, __LINE__, __FUNCTION__, ## __VA_ARGS__ )