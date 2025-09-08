#[macro_export]
macro_rules! create_empty_scope_fn {
    ($fn_name:ident) => {
        use ntex::web::{scope, ServiceConfig};
        pub fn $fn_name(cnf: &mut ServiceConfig) {
            cnf.service(scope("/"));
        }
    };
}
