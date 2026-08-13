//! Authentication & authorization.

pub mod extractor;
pub mod jwt;
pub mod middleware;
pub mod password;

pub use extractor::CurrentUser;
pub use jwt::{Claims, JwtService, Role};
pub use middleware::{require_auth, require_role};
pub use password::{hash_password, verify_password};
