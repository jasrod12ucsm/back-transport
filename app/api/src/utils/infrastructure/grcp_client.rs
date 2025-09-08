use tonic::{Request, Response, Status, transport::Channel};
pub mod geocache {
    tonic::include_proto!("geocache");
}
