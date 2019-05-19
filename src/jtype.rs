use std::clone::Clone;

use crate::cache::Ref;
use crate::string::JString;
use crate::uint;

pub type TypeId = uint;

// TODO: add new function
#[derive(Debug)]
pub struct Type(pub(crate) TypeId, pub(crate) Ref<JString>);

impl Clone for Type {
    fn clone(&self) -> Self {
        Type(self.0, self.1.clone())
    }
}

impl PartialEq<Type> for Type {
    fn eq(&self, other: &Type) -> bool {
        self.0 == other.0
    }
}
