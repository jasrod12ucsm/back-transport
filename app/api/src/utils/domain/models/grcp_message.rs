use std::sync::Arc;
use tokio::sync::Mutex;

use tonic::{Request, transport::Channel};

use crate::utils::infrastructure::grcp_client::geocache::{
    self, GetDevicesPolygonRequest, GetDevicesPolygonResponse, IdAndLocation,
    get_devices_polygon_service_client::GetDevicesPolygonServiceClient,
};

#[derive(Clone)]
pub struct GrpcMessage {
    geo_cache: Arc<
        Mutex<
            geocache::get_devices_polygon_service_client::GetDevicesPolygonServiceClient<
                tonic::transport::Channel,
            >,
        >,
    >,
}
impl GrpcMessage {
    pub fn new(client: Channel) -> Self {
        Self {
            geo_cache: Arc::new(Mutex::new(GetDevicesPolygonServiceClient::new(client))),
        }
    }

    pub async fn get_grcp_message(
        &self,
        latitude1: f64,
        latitude2: f64,
        latitude3: f64,
        latitude4: f64,
        longitude1: f64,
        longitude2: f64,
        longitude3: f64,
        longitude4: f64,
    ) -> Result<Vec<IdAndLocation>, Box<dyn std::error::Error>> {
        let mut client = self.geo_cache.lock().await;

        let request = Request::new(GetDevicesPolygonRequest {
            latitude_1: latitude1,
            longitude_1: longitude1,
            latitude_2: latitude2,
            longitude_2: longitude2,
            latitude_3: latitude3,
            longitude_3: longitude3,
            latitude_4: latitude4,
            longitude_4: longitude4,
        });

        // 🚀 Aquí haces realmente la llamada RPC
        match client.get_devices_polygon(request).await {
            Ok(response) => {
                let resp: &GetDevicesPolygonResponse = response.get_ref();
                Ok(resp.device_ids.clone()) // devuelve la lista de ids
            }
            Err(status) => {
                println!("gRPC call failed: {:?}", status);

                // 🔄 Recreate channel & client
                let channel = Channel::from_static("http://[::1]:50051").connect().await?;
                *client = GetDevicesPolygonServiceClient::new(channel);

                Err("gRPC call failed, client reconnected".into())
            }
        }
    }
}
