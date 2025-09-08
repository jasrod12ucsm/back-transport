use std::collections::HashMap;

pub trait GetAttributes {
    fn get_attributes(&self) -> HashMap<&'static str, String>;
}
#[macro_export]
macro_rules! impl_get_attributes {
    ($ty:ty, $($field:ident),*) => {
        impl $crate::macros::get_attributes::GetAttributes for $ty {
            fn get_attributes(&self) -> std::collections::HashMap<&'static str, std::string::String> {
                let mut attributes = std::collections::HashMap::new();
                $(
                    attributes.insert(stringify!($field), format!("{:?}", self.$field));
                )*
                attributes
            }
        }
    };
}
